//! GameDeck artwork + metadata provider: exact matches, reviewed candidates, local cache.
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, time::Duration};
use image::{DynamicImage, ImageFormat, GenericImageView};
pub mod background;
pub mod media;
pub mod display;
pub mod title;
pub mod promo;
mod background_download;
mod platform_identity;
mod alpha_api;
pub mod motion_review;
pub mod epic_cache;
pub mod epic;
pub mod battlenet_cache;

#[derive(Clone)]
pub struct Scraper { alpha_quota: std::sync::Arc<tokio::sync::Mutex<alpha_api::Quota>>, promo_network: bool, client: reqwest::Client, key: Option<String>, alpha_key: Option<String>, igdb: Option<IgdbCredentials> }
#[derive(Clone)]
pub struct IgdbCredentials { pub client_id: String, pub access_token: String }
#[derive(Debug, Clone)]
pub struct GameRef { pub id: String, pub name: String, pub provider: String, pub external_id: String }
#[derive(Debug, Clone)]
pub struct Artwork { pub kind: &'static str, pub path: PathBuf, pub source: &'static str, pub width: u32, pub height: u32 }
#[derive(Debug, Clone, Serialize)]
pub struct Candidate { pub id: u64, pub name: String, pub exact: bool }
#[derive(Debug,Clone,Deserialize)]
pub struct OwnedSteamGame { pub appid: u64, pub name: String, #[serde(default)] pub playtime_forever: i64, pub rtime_last_played: Option<i64> }
#[derive(Debug, Clone, Serialize, Default)]
pub struct GameMetadata { pub description: Option<String>, pub genres: Vec<String>, pub screenshots: Vec<String> }
#[derive(Deserialize)]
struct ApiResponse<T> { data: Vec<T> }
#[derive(Deserialize)]
struct SearchHit { id: u64, name: String }
#[derive(Deserialize)]
struct AssetHit { url: String, width: Option<u32>, height: Option<u32> }
#[derive(Deserialize)]
struct IgdbGenre { name: String }
#[derive(Deserialize)]
struct IgdbScreenshot { image_id: String }

pub fn normalize(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}
pub fn artwork_search_name(name:&str)->String{
    let clean=name.replace(['™','®','©']," ").split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(start)=clean.rfind('('){if clean.ends_with(')')&&clean[..start].chars().any(|c|c.is_alphabetic()&&!c.is_ascii()){
        let alias=&clean[start+1..clean.len()-1];if alias.chars().any(|c|c.is_ascii_alphabetic())&&alias.is_ascii(){return alias.trim().into();}
    }}
    if clean.chars().any(|c|c.is_ascii_alphabetic())&&clean.chars().any(|c|('\u{4e00}'..='\u{9fff}').contains(&c))&&!['版','重','集','试','声'].iter().any(|c|clean.contains(*c)){
        let alias=clean.chars().filter(|c|!('\u{4e00}'..='\u{9fff}').contains(c)).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
        if !["demo","dlc","prologue","ost","soundtrack","playtest"].contains(&normalize(&alias).as_str()){return alias;}
    }
    clean.trim().to_string()
}
#[cfg(test)]mod search_name_tests{use super::*;#[test]fn cleans_trademarks_and_explicit_bilingual_aliases_without_losing_editions(){assert_eq!(artwork_search_name("Stellar Blade™"),"Stellar Blade");assert_eq!(artwork_search_name("大富翁10(Richman 10)"),"Richman 10");assert_eq!(artwork_search_name("Stellar Blade™ Demo"),"Stellar Blade Demo");assert_eq!(artwork_search_name("Game (Definitive Edition)"),"Game (Definitive Edition)");assert_eq!(artwork_search_name("暖雪 Warm Snow"),"Warm Snow");assert_eq!(artwork_search_name("Dragon Cliff 龙崖"),"Dragon Cliff");assert_eq!(artwork_search_name("Game 中文版"),"Game 中文版");assert_eq!(artwork_search_name("江湖幸存者 Demo"),"江湖幸存者 Demo");assert_eq!(artwork_search_name("ACE COMBAT™7: SKIES UNKNOWN"),"ACE COMBAT 7: SKIES UNKNOWN");}}

/// Read original Steam cache images. Blurred preview files must never become hero artwork.
pub fn import_steam_cache(game: &GameRef, caches: &[PathBuf], root: &Path) -> Result<Vec<Artwork>, String> {import_cache_kinds(game,caches,root,&["poster","hero","landscape","logo"])}
pub fn import_title_references(game:&GameRef,caches:&[PathBuf],root:&Path)->Result<Vec<Artwork>,String>{import_cache_kinds(game,caches,root,&["logo"])}
fn import_cache_kinds(game:&GameRef,caches:&[PathBuf],root:&Path,kinds:&[&'static str])->Result<Vec<Artwork>,String>{
    if game.provider != "steam" || game.external_id.is_empty() || !game.external_id.chars().all(|c| c.is_ascii_digit()) { return Ok(Vec::new()); }
    let mut files = Vec::new();
    for cache in caches {
        let directory = cache.join(&game.external_id);
        if let Ok(entries) = std::fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() { files.push(path); }
                else if path.is_dir() {
                    if let Ok(children) = std::fs::read_dir(path) { files.extend(children.flatten().map(|e| e.path()).filter(|p| p.is_file())); }
                }
            }
        }
        // Older clients use appid-prefixed files directly inside librarycache.
        if let Ok(entries) = std::fs::read_dir(cache) {
            let prefix = format!("{}_", game.external_id);
            files.extend(entries.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(&prefix)).map(|e| e.path()).filter(|p| p.is_file()));
        }
    }
    let mut assets = Vec::new();
    for &kind in kinds {
        let best = files.iter().filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_lowercase();
            let suitable = match kind {
                "hero" => name.contains("library_hero") && !name.contains("blur"),
                "poster" => name.contains("library_600x900") || name.contains("library_capsule"),
                "landscape" => name == "header" || name.starts_with("header_") || name.contains("library_header") || (name.contains("library_hero")&&!name.contains("blur")),
                _ => name == "logo" || name.starts_with("logo_") || name.ends_with("_logo") || name.contains("library_logo") || name.strip_prefix(&format!("{}_",game.external_id)).is_some_and(|n|n=="logo"||n.starts_with("logo_")),
            };
            if !suitable { return None; }
            let (width, height) = image::image_dimensions(path).ok()?;
            if width == 0 || height == 0 || width > 10000 || height > 10000 { return None; }
            Some((path, width, height, name.contains("schinese")))
        }).max_by_key(|(_,w,h,localized)| (*w as u64 * *h as u64, *localized));
        if let Some((source,width,height,_)) = best {
            let directory = root.join(kind);
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let extension = source.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
            let bytes=std::fs::read(source).map_err(|e|e.to_string())?;
            use std::hash::{Hash,Hasher};let mut hash=std::collections::hash_map::DefaultHasher::new();bytes.hash(&mut hash);
            let path=directory.join(format!("{}-steam-{:016x}.{extension}",game.id,hash.finish()));
            if !path.exists(){std::fs::write(&path,bytes).map_err(|e|e.to_string())?;}
            assets.push(Artwork { kind, path, source: "steam-local", width, height });
        }
    }
    Ok(assets)
}
impl Scraper {
    pub fn new(key: Option<String>) -> Result<Self, String> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent("GameDeck/1.0 (personal game library)")
            .build().map_err(|e| e.to_string())?;
        Ok(Self { alpha_quota:Default::default(), promo_network:true, client, key: key.filter(|s| !s.trim().is_empty()), alpha_key: None, igdb: None })
    }
    pub fn cached_promo_only(mut self)->Self{self.promo_network=false;self}
    pub fn with_alpha_key(mut self, key:Option<String>)->Self { let key=key.filter(|s|!s.trim().is_empty());if self.alpha_key!=key{self.alpha_quota=Default::default();}self.alpha_key=key;self }
    pub fn with_igdb(mut self, client_id: Option<String>, token: Option<String>) -> Self {
        if let (Some(client_id), Some(access_token)) = (client_id, token) {
            if !client_id.trim().is_empty() && !access_token.trim().is_empty() {
                self.igdb = Some(IgdbCredentials { client_id, access_token });
            }
        }
        self
    }
    pub async fn candidates(&self, name: &str) -> Result<Vec<Candidate>, String> {
        let canonical=artwork_search_name(name);let name=canonical.as_str();
        let key = self.key.as_ref().ok_or("请先填写 SteamGridDB API Key")?;
        let url = format!("https://www.steamgriddb.com/api/v2/search/autocomplete/{}", encode(name));
        let response = self.client.get(url).bearer_auth(key).send().await.map_err(|e| e.to_string())?
            .error_for_status().map_err(|e| e.to_string())?
            .json::<ApiResponse<SearchHit>>().await.map_err(|e| e.to_string())?;
        Ok(response.data.into_iter().take(15).map(|x| Candidate { exact: normalize(&artwork_search_name(&x.name)) == normalize(name), id: x.id, name: x.name }).collect())
    }
    pub async fn steam_candidates(&self, name: &str) -> Result<Vec<Candidate>,String> {
        #[derive(Deserialize)] struct Search { #[serde(default)] items: Vec<SearchHit> }
        let response = self.client.get("https://store.steampowered.com/api/storesearch/")
            .query(&[("term",name),("cc","us"),("l","english")]).send().await.map_err(|e| e.to_string())?
            .error_for_status().map_err(|e| e.to_string())?.json::<Search>().await.map_err(|e| e.to_string())?;
        Ok(response.items.into_iter().take(15).map(|hit| Candidate { id:hit.id, exact:normalize(&hit.name)==normalize(name),name:hit.name }).collect())
    }
    pub async fn owned_steam_games(&self, key: &str, account: &str) -> Result<Vec<OwnedSteamGame>,String> {
        let account = account.parse::<u32>().map_err(|_| "Steam 账号无效")? as u64 + 76561197960265728;
        #[derive(Deserialize)] struct Response { response: Option<Library> }
        #[derive(Deserialize)] struct Library { games: Option<Vec<OwnedSteamGame>>, game_count: Option<usize> }
        let response = self.client.get("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/")
            .query(&[("key",key),("steamid",&account.to_string()),("include_appinfo","true"),("include_played_free_games","true")])
            .send().await.map_err(|_| "无法连接 Steam 账号库服务".to_string())?
            .error_for_status().map_err(|_| "Steam 账号库请求失败，请检查 Web API Key".to_string())?
            .json::<Response>().await.map_err(|_| "Steam 账号库返回格式错误".to_string())?;
        let library = response.response.ok_or("Steam 未返回账号库")?;
        if library.game_count.is_none() { return Err("Steam 游戏详情未公开或无法访问，请在 Steam 隐私设置中开放游戏详情".into()); }
        Ok(library.games.unwrap_or_default())
    }
    async fn sgdb_asset(&self,id:u64,kind:&str,key:&str,purpose:&str)->Result<Option<String>,String>{Ok(self.sgdb_asset_candidates(id,kind,key,purpose).await?.into_iter().next())}
    async fn sgdb_asset_candidates(&self, id: u64, kind: &str, key: &str, purpose:&str) -> Result<Vec<String>, String> {
        let url = format!("https://www.steamgriddb.com/api/v2/{kind}/game/{id}");
        let response = self.client.get(url).bearer_auth(key).send().await.map_err(|e| e.to_string())?
            .error_for_status().map_err(|e| e.to_string())?
            .json::<ApiResponse<AssetHit>>().await.map_err(|e| e.to_string())?;
        let mut assets = response.data;
        assets.retain(|asset|asset_dimensions(purpose,asset.width.unwrap_or(0),asset.height.unwrap_or(0)));
        // Prefer high resolution, never trust provider's filename or MIME.
        assets.sort_by_key(|x| std::cmp::Reverse(x.width.unwrap_or(0) as u64 * x.height.unwrap_or(0) as u64));
        Ok(assets.into_iter().filter(|x| x.url.starts_with("https://")).take(8).map(|x| x.url).collect())
    }
    async fn fetch_image(&self, url: &str, dest: &Path, max_width: u32) -> Result<Option<(u32,u32)>, String> {
        if !url.starts_with("https://") { return Err("仅允许 HTTPS 图片".into()); }
        if let Ok(parsed)=reqwest::Url::parse(url){if let Some(host)=parsed.host_str().filter(|host|host.ends_with(".steamstatic.com")){
            let bytes=background_download::original(url,host).await?;
            let (w,h)=image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format().map_err(|e|e.to_string())?.into_dimensions().map_err(|e|e.to_string())?;
            if w>10000||h>10000||w as u64*h as u64>40_000_000{return Err("图片尺寸过大".into());}
            return save_image(image::load_from_memory(&bytes).map_err(|e|e.to_string())?,dest,max_width).map(Some);
        }}
        let response = self.client.get(url).send().await.map_err(|e| e.to_string())?;
        if !response.status().is_success() { return Ok(None); }
        let ct = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
        if !["image/jpeg", "image/png", "image/webp"].iter().any(|x| ct.starts_with(x)) { return Ok(None); }
        if response.content_length().is_some_and(|n| n > 16_000_000) { return Ok(None); }
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        if bytes.is_empty() || bytes.len() > 16_000_000 { return Ok(None); }
        let image = image::load_from_memory(&bytes).map_err(|e| format!("无效图片: {e}"))?;
        if image.width() > 10000 || image.height() > 10000 { return Err("图片尺寸过大".into()); }
        let (w,h) = save_image(image, dest, max_width)?;
        Ok(Some((w,h)))
    }
    async fn fetch_artwork(&self,url:&str,dest:&Path,max_width:u32,kind:&str)->Result<Option<(u32,u32)>,String>{
        static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
        let pending=dest.with_extension(format!("pending-{}-{}",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
        let result=match self.fetch_image(url,&pending,max_width).await{
            Ok(Some((w,h))) if asset_dimensions(kind,w,h)=>std::fs::rename(&pending,dest).map(|_|Some((w,h))).map_err(|e|e.to_string()),
            Ok(Some(_))=>Ok(None),other=>other,
        };
        let _=std::fs::remove_file(&pending);let _=std::fs::remove_file(pending.with_extension("tmp"));result
    }
    async fn fetch_sgdb_media(&self,url:&str,dest:&Path,kind:&'static str)->Result<Option<Artwork>,String>{
        let valid=|u:&reqwest::Url|u.scheme()=="https"&&matches!(u.host_str(),Some("cdn2.steamgriddb.com"|"cdn.steamgriddb.com"));
        if !valid(&reqwest::Url::parse(url).map_err(|_|"无效素材地址")?){return Err("素材来源无效".into());}
        let mut response=self.client.get(url).send().await.map_err(|_|"素材下载失败")?;
        if !response.status().is_success(){return Ok(None);}if !valid(response.url()){return Err("素材跳转来源无效".into());}
        if response.content_length().is_some_and(|n|n>24_000_000){return Err("素材超过 24 MB".into());}
        let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|_|"素材传输失败")?{if bytes.len()+chunk.len()>24_000_000{return Err("素材超过 24 MB".into());}bytes.extend_from_slice(&chunk);}
        let info=media::inspect(&bytes)?;if !asset_dimensions(kind,info.width,info.height){return Ok(None);}
        use std::hash::{Hash,Hasher};let mut hash=std::collections::hash_map::DefaultHasher::new();bytes.hash(&mut hash);
        let base=dest.with_file_name(format!("{}-sgdb-{:016x}.png",dest.file_stem().unwrap_or_default().to_string_lossy(),hash.finish()));
        let (path,info)=media::publish(&bytes,&base)?;
        Ok(Some(Artwork{kind,path,source:"steamgriddb",width:info.width,height:info.height}))
    }
    pub async fn scrape_motion_cards(&self,game:&GameRef,root:&Path,confirmed:Option<u64>)->(Vec<Artwork>,Vec<String>){
        self.scrape_motion_cards_for(game,root,confirmed,&["poster","landscape"]).await
    }
    pub async fn scrape_motion_cards_for(&self,game:&GameRef,root:&Path,confirmed:Option<u64>,kinds:&[&'static str])->(Vec<Artwork>,Vec<String>){
        self.motion_cards_impl(game,root,confirmed,kinds,false).await
    }
    pub async fn collect_motion_cards_for(&self,game:&GameRef,root:&Path,confirmed:Option<u64>,kinds:&[&'static str])->(Vec<Artwork>,Vec<String>){
        self.motion_cards_impl(game,root,confirmed,kinds,true).await
    }
    async fn motion_cards_impl(&self,game:&GameRef,root:&Path,confirmed:Option<u64>,kinds:&[&'static str],collect:bool)->(Vec<Artwork>,Vec<String>){
        let mut reviewed=Vec::new();let mut pending=Vec::new();for &kind in kinds{if let Some(asset)=motion_review::approved(root,&game.id,kind){reviewed.push(asset);if collect{pending.push(kind);}}else{pending.push(kind);}}
        if pending.is_empty(){return(reviewed,Vec::new());}
        let Some(key)=&self.key else{return (reviewed,Vec::new())};
        let id=if let Some(id)=confirmed{Some(id)}else{self.candidates(&game.name).await.ok().and_then(unique_exact)};
        let Some(id)=id else{return (reviewed,vec!["动态卡片未找到唯一游戏匹配".into()])};
        let result=async {
            let data=self.client.get(format!("https://www.steamgriddb.com/api/v2/grids/game/{id}")).bearer_auth(key).query(&[("types","animated"),("nsfw","false"),("humor","false")]).send().await.map_err(|_|"动态素材查询连接失败")?.error_for_status().map_err(|_|"动态素材查询失败")?.json::<ApiResponse<AssetHit>>().await.map_err(|_|"动态素材数据无效")?;
            Ok::<_,String>(data.data)
        }.await;
        let hits=match result{Ok(h)=>h,Err(e)=>return(reviewed,vec![e])};let mut errors=Vec::new();
        for kind in pending {
            let dir=root.join(kind);if let Err(e)=std::fs::create_dir_all(&dir){errors.push(e.to_string());continue;}
            let mut eligible:Vec<_>=hits.iter().filter(|h|asset_dimensions(kind,h.width.unwrap_or(0),h.height.unwrap_or(0))).collect();eligible.sort_by_key(|h|std::cmp::Reverse(h.width.unwrap_or(0) as u64*h.height.unwrap_or(0) as u64));
            for hit in eligible.into_iter().take(4){match self.fetch_sgdb_media(&hit.url,&dir.join(format!("{}.png",game.id)),kind).await{
                Ok(Some(art)) if art.path.to_string_lossy().contains(".motion.")=>{errors.push(format!("{kind}: 已缓存动图候选，确认游戏名称/Logo清晰后再应用"));if !collect{break;}},Ok(_)=>(),Err(e)=>errors.push(format!("{kind}: {e}")),
            }}
        }
        (reviewed,errors)
    }
    pub async fn igdb_metadata(&self, name: &str) -> Result<Option<GameMetadata>, String> {
        let creds = match &self.igdb { Some(x) => x, None => return Ok(None) };
        let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
        let body = format!("search \"{}\"; fields name,summary,genres.name,screenshots.image_id; limit 10;", escaped);
        #[derive(Deserialize)] struct NamedGame { name: String, summary: Option<String>, genres: Option<Vec<IgdbGenre>>, screenshots: Option<Vec<IgdbScreenshot>> }
        let hits = self.client.post("https://api.igdb.com/v4/games")
            .header("Client-ID", &creds.client_id).bearer_auth(&creds.access_token)
            .body(body).send().await.map_err(|e| e.to_string())?
            .error_for_status().map_err(|e| e.to_string())?
            .json::<Vec<NamedGame>>().await.map_err(|e| e.to_string())?;
        Ok(hits.into_iter().find(|g| normalize(&g.name) == normalize(name)).map(|g| GameMetadata {
            description: g.summary, genres: g.genres.unwrap_or_default().into_iter().map(|g| g.name).collect(),
            screenshots: g.screenshots.unwrap_or_default().into_iter().take(8)
                .map(|s| format!("https://images.igdb.com/igdb/image/upload/t_1080p/{}.jpg", s.image_id)).collect(),
        }))
    }
    pub async fn scrape(&self, game: &GameRef, root: &Path, force: bool, confirmed_id: Option<u64>, confirmed_steam: Option<u64>) -> (Vec<Artwork>, Vec<String>) {
        self.scrape_kinds(game,root,force,confirmed_id,confirmed_steam,&["poster","hero","landscape","logo"]).await
    }
    pub async fn scrape_kinds(&self,game:&GameRef,root:&Path,force:bool,confirmed_id:Option<u64>,confirmed_steam:Option<u64>,kinds:&[&'static str])->(Vec<Artwork>,Vec<String>){
        let mut assets = Vec::new(); let mut errors = Vec::new();
        if kinds.contains(&"logo"){match self.scrape_title(game,root,confirmed_id,confirmed_steam).await{Ok(Some(asset))=>assets.push(asset),Ok(None)=>(),Err(e)=>errors.push(format!("{} Logo: {e}",game.name))}}
        if !kinds.iter().any(|k|*k!="logo"){return(assets,errors);}
        let sgdb = if confirmed_id.is_some() { confirmed_id } else if self.key.is_some() {
            match self.candidates(&game.name).await {
                Ok(hits) => unique_exact(hits),
                Err(e) => { errors.push(format!("{}: {e}", game.name)); None }
            }
        } else { None };
        let steam_id = if game.provider == "steam" { game.external_id.parse::<u64>().ok() } else if confirmed_steam.is_some() { confirmed_steam } else if game.provider=="battlenet" && game.external_id=="diablo_iv" {Some(2344520)} else {
            match self.steam_candidates(&game.name).await {
                Ok(hits) => unique_exact(hits),
                Err(error) => { errors.push(format!("{} Steam 素材搜索: {error}",game.name)); None }
            }
        };
        for (kind, endpoint, max_width) in [("poster", "grids", 800), ("hero", "heroes", 3840), ("landscape", "grids", 1920)] {
            if !kinds.contains(&kind){continue;}
            if kind=="hero"{match self.scrape_background(game,root,sgdb,steam_id,force).await{Ok(asset)=>assets.push(asset),Err(error)=>errors.push(format!("{} 背景: {error}",game.name))}continue;}
            let dir = root.join(kind);
            if let Err(e) = tokio::fs::create_dir_all(&dir).await { errors.push(e.to_string()); continue; }
            let path = dir.join(format!("{}.png", game.id));
            if !force && path.exists() {
                if let Ok(img) = image::open(&path) {
                    let (width,height) = img.dimensions();
                    if asset_dimensions(kind,width,height){assets.push(Artwork { kind, path, source: "cache", width, height });continue;}
                }
            }
            let mut saved = false;
            if game.provider=="battlenet" && ["diablo_iii","diablo_iii_cn"].contains(&game.external_id.as_str()) {
                let url=match kind {"poster"=>Some("https://bnetproduct-a.akamaihd.net/fbd/bafaafcfb7c6c620067662a04409ba66-prod-card-tall.jpg"),"hero"|"landscape"=>Some("https://bnetproduct-a.akamaihd.net/fad/6a06a79f8b1134a80d794dc24c9cd2d1-prod-mobile-bg.jpg"),_=>None};
                if let Some(url)=url {match self.fetch_artwork(url,&path,max_width,kind).await {Ok(Some((width,height)))=>{assets.push(Artwork{kind,path:path.clone(),source:"blizzard",width,height});saved=true;},Ok(None)=>(),Err(error)=>errors.push(format!("{} {kind}: {error}",game.name))}}
            }
            if let (Some(id), Some(key)) = (sgdb, &self.key) {
                match self.sgdb_asset(id, endpoint, key, kind).await {
                    Ok(Some(url)) => match self.fetch_sgdb_media(&url, &path,kind).await {
                        Ok(Some(asset))=>{assets.push(asset);saved=true;},
                        Ok(None)=>(),Err(e)=>errors.push(format!("{} {kind}: {e}",game.name)),
                    },
                    Ok(None) => (), Err(e) => errors.push(format!("{} {kind}: {e}", game.name)),
                }
            }
            if !saved && steam_id.is_some() {
                let suffix = match kind { "poster" => "library_600x900.jpg", "hero" => "library_hero.jpg", "logo" => "logo.png", _ => "library_hero.jpg" };
                let url = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/{suffix}", steam_id.unwrap());
                match self.fetch_artwork(&url, &path, max_width,kind).await {
                    Ok(Some((width,height))) if asset_dimensions(kind,width,height) => assets.push(Artwork { kind, path, source: "steam", width, height }),
                    Ok(Some(_))=>(),
                    Ok(None) => (), Err(e) => errors.push(format!("{} {kind}: {e}", game.name)),
                }
            }
        }
        // Dynamic replacements use the same short-loop validation as manual imports.
        // Run after static scraping so a later refresh cannot overwrite a valid loop
        // merely because the static API response happened to list it first.
        let card_kinds:Vec<_>=kinds.iter().copied().filter(|k|["poster","landscape"].contains(k)).collect();
        let (motion,issues)=self.scrape_motion_cards_for(game,root,sgdb,&card_kinds).await;
        errors.extend(issues);
        for animated in motion{assets.retain(|asset|asset.kind!=animated.kind);assets.push(animated);}
        if assets.is_empty() && errors.is_empty() { errors.push(format!("{}：没有精确素材匹配，请在单款游戏中搜索并确认候选或导入本地图片",game.name)); }
        (assets, errors)
    }
    pub async fn cache_screenshots(&self, game: &GameRef, urls: &[String], root: &Path) -> (Vec<Artwork>, Vec<String>) {
        let mut saved = Vec::new(); let mut errors = Vec::new();
        let dir = root.join("screenshots");
        if let Err(e) = tokio::fs::create_dir_all(&dir).await { return (saved, vec![e.to_string()]); }
        for (i,url) in urls.iter().take(8).enumerate() {
            let path = dir.join(format!("{}-{}.png", game.id, i));
            let kind: &'static str = match i { 0 => "screenshot_0", 1 => "screenshot_1", 2 => "screenshot_2", 3 => "screenshot_3", 4 => "screenshot_4", 5 => "screenshot_5", 6 => "screenshot_6", _ => "screenshot_7" };
            if path.exists() {
                if let Ok(img) = image::open(&path) { let (width,height)=img.dimensions(); saved.push(Artwork {kind,path,source:"cache",width,height}); continue; }
            }
            match self.fetch_image(url, &path, 1920).await {
                Ok(Some((width,height))) => saved.push(Artwork {kind,path,source:"igdb",width,height}),
                Ok(None) => (), Err(e) => errors.push(format!("截图 {}: {e}", i+1)),
            }
        }
        (saved, errors)
    }
    pub fn import_image(src: &Path, dest: &Path, kind: &str) -> Result<(u32,u32), String> {
        let max_width = match kind { "poster" => 800, "hero" => 7680, "landscape"=>3840, "logo" => 1200, _ => return Err("未知图片类型".into()) };
        let image = image::open(src).map_err(|e| e.to_string())?;
        save_image(image, dest, max_width)
    }
}
fn asset_dimensions(kind:&str,w:u32,h:u32)->bool{match kind{"poster"=>w>=400&&h>=600&&h>w,"landscape"=>w>=900&&h>=400&&(1.4..=3.6).contains(&(w as f64/h as f64)),"hero"=>background::suitable(w,h),"logo"=>w>=100&&h>=20&&w<=6000&&h<=4000,_=>false}}
#[cfg(test)]mod wide_card_tests{#[test]fn accepts_full_size_horizontal_grid_but_rejects_its_thumbnail(){assert!(!super::asset_dimensions("logo",16000,4000));assert!(super::asset_dimensions("logo",1280,400));assert!(super::asset_dimensions("landscape",920,430));assert!(!super::asset_dimensions("landscape",460,215));assert!(!super::asset_dimensions("hero",920,430));}}
fn unique_exact(hits:Vec<Candidate>)->Option<u64>{let mut ids=hits.into_iter().filter(|hit|hit.exact).map(|hit|hit.id);let first=ids.next()?;if ids.next().is_none(){Some(first)}else{None}}
fn save_image(image: DynamicImage, dest: &Path, max_width: u32) -> Result<(u32,u32), String> {
    let resized = if image.width() > max_width { image.resize(max_width, 4096, image::imageops::FilterType::Lanczos3) } else { image };
    let dims = resized.dimensions();
    let tmp = dest.with_extension("tmp");
    resized.save_with_format(&tmp, ImageFormat::Png).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(dims)
}
fn encode(s: &str) -> String {
    s.as_bytes().iter().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(b) { (*b as char).to_string() } else { format!("%{b:02X}") }).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_cache_uses_original_artwork_and_skips_blurred_previews() {
        let dir = std::env::temp_dir().join(format!("gamedeck-local-cache-{}", std::process::id()));
        let cache = dir.join("librarycache");
        let hashed = cache.join("570/hash");
        std::fs::create_dir_all(&hashed).unwrap();
        image::RgbImage::new(600,200).save(hashed.join("library_hero.jpg")).unwrap();
        image::RgbImage::new(1200,400).save(hashed.join("library_hero_blur.jpg")).unwrap();
        image::RgbImage::new(600,900).save(hashed.join("library_capsule.jpg")).unwrap();
        image::RgbImage::new(460,215).save(hashed.join("library_header.jpg")).unwrap();
        let game = GameRef { id: "fixture".into(), name: "Dota 2".into(), provider: "steam".into(), external_id: "570".into() };
        let assets = import_steam_cache(&game,&[cache],&dir.join("output")).unwrap();
        assert_eq!(assets.len(),3);
        let hero = assets.iter().find(|a| a.kind == "hero").unwrap();
        assert_eq!((hero.width,hero.height),(600,200));
        assert_eq!(std::fs::read(&hero.path).unwrap(),std::fs::read(hashed.join("library_hero.jpg")).unwrap());
        assert!(assets.iter().any(|a| a.kind == "landscape"));
        image::RgbImage::new(1920,620).save(hashed.join("library_hero.jpg")).unwrap();
        let upgraded=import_steam_cache(&game,&[dir.join("librarycache")],&dir.join("output")).unwrap();
        let upgraded=upgraded.iter().find(|asset|asset.kind=="hero").unwrap();
        assert_ne!(hero.path,upgraded.path);
        assert_eq!(image::image_dimensions(&upgraded.path).unwrap(),(1920,620));

        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]fn asset_roles_are_not_interchangeable(){assert!(!asset_dimensions("landscape",600,900));assert!(asset_dimensions("poster",600,900));assert!(!asset_dimensions("hero",1920,620));assert!(asset_dimensions("landscape",1920,620));}
    #[test]fn ambiguous_names_require_confirmation(){let hit=|id|Candidate{id,name:"Same title".into(),exact:true};assert_eq!(unique_exact(vec![hit(1)]),Some(1));assert_eq!(unique_exact(vec![hit(1),hit(2)]),None);}
    #[test] fn matching_is_strict() { assert_eq!(normalize("Diablo IV: Ultimate"), normalize("diablo iv ultimate")); assert_ne!(normalize("Diablo III"), normalize("Diablo IV")); }
    #[test] fn utf8_search_encoding() { assert_eq!(encode("鸣潮"), "%E9%B8%A3%E6%BD%AE"); }
    #[test] fn image_resize_preserves_alpha() {
        let dir = std::env::temp_dir().join(format!("gamedeck-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap(); let out = dir.join("logo.png");
        let rgba = image::RgbaImage::from_pixel(2000, 400, image::Rgba([255,0,0,0]));
        let (w,_) = save_image(DynamicImage::ImageRgba8(rgba), &out, 1200).unwrap();
        assert_eq!(w,1200); assert_eq!(image::open(&out).unwrap().to_rgba8().get_pixel(0,0)[3],0);
        let _ = std::fs::remove_dir_all(dir);
    }
}

pub mod branding;

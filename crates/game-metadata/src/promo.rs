//! Versioned promotional-background policy. No per-game artwork allowlist.
use super::*;
use std::{collections::BTreeMap,hash::{Hash,Hasher}};
pub const POLICY:u32=5;
#[derive(Clone)]struct Source{url:String,provider:&'static str,title:String,evidence:String,official:bool,page:String,host:String,w:u32,h:u32}
#[derive(Default,Serialize)]struct Audit{policy:u32,mode:String,game_id:String,game:String,selected:Option<serde_json::Value>,candidates:Vec<serde_json::Value>,issues:Vec<String>}
pub fn semantic(text:&str)->Result<&'static str,&'static str>{let t=text.to_lowercase();
 if t.split_whitespace().collect::<Vec<_>>().windows(3).any(|w|w[1]=="x"&&w[0].chars().any(char::is_alphabetic)&&w[2].chars().any(char::is_alphabetic)){return Err("实机/场景/同人/跨游戏主题，不作为宣传主视觉");}
 if ["crossover","cross-over","cross over","fanart","fan art","cosplay","ai-generated","ai generated","联动","跨界","同人","screenshot","screen shot","gameplay","in-game","ingame","photo mode","photomode","实机","截图","游戏画面","concept art","environment art","landscape scenery","概念图","场景图"].iter().any(|s|t.contains(s)){return Err("实机/场景/同人/跨游戏主题，不作为宣传主视觉");}
 if ["key art","keyart","key visual","keyvisual","promotional","promo art","cover art","coverart","box art","boxart","poster","official artwork","main visual","主视觉","宣传海报","宣传图","封面原画"].iter().any(|s|t.contains(s)){return Ok("宣传海报/主视觉类型证据");}
 if t.contains("logo")||t.contains("徽标"){return Err("独立 Logo，不是宣传主视觉");}
 Err("只有壁纸/角色/游戏标签，未确认宣传主视觉类型")
}
pub fn tier(w:u32,h:u32,official:bool)->u8{
 if w==0||h==0||w>10000||h>10000||w as u64*h as u64>40_000_000{return 0;}let ratio=w as f64/h as f64;
 if !(1.5..=3.6).contains(&ratio){return 0;}
 if w>=3840&&h>=2160{4}else if w>=2560&&h>=1440{3}else if official&&ratio>2.1&&w>=1600&&h>=600{1}else if official&&w>=1920&&h>=1080{2}else if official&&w>=1600&&h>=600{1}else{0}
}
fn ranking(w:u32,h:u32,official:bool,proof:u8)->(u8,u8,u64){(tier(w,h,official),proof,(w as u64*1080).min(h as u64*1920))}
fn preference(c:&background::BackgroundChoice)->(bool,u8,bool,u8,u64){let(t,p,n)=ranking(c.width,c.height,c.score>=100,c.score as u8);(!c.title_detected&&t>=3,t,!c.title_detected,p,n)}
fn grade(t:u8)->&'static str{match t{4=>"4K",3=>"2K",2=>"官方1080P回退",_=>"官方宣传横幅回退"}}
fn clean_url(value:&str)->String{reqwest::Url::parse(value).map(|mut u|{u.set_query(None);u.set_fragment(None);u.to_string()}).unwrap_or_default()}
fn attr(tag:&str,key:&str)->Option<String>{
 let bytes=tag.as_bytes();let mut i=0;while i<bytes.len(){while i<bytes.len()&&!(bytes[i].is_ascii_alphanumeric()||bytes[i]==b'-'){i+=1;}let start=i;while i<bytes.len()&&(bytes[i].is_ascii_alphanumeric()||[b'-',b':',b'_'].contains(&bytes[i])){i+=1;}let name=&tag[start..i];while i<bytes.len()&&bytes[i].is_ascii_whitespace(){i+=1;}if i==bytes.len(){break;}if bytes[i]!=b'='{continue;}i+=1;while i<bytes.len()&&bytes[i].is_ascii_whitespace(){i+=1;}if i==bytes.len(){break;}let quote=bytes[i];let value=if quote==b'\''||quote==b'"'{i+=1;let a=i;while i<bytes.len()&&bytes[i]!=quote{i+=1;}let v=&tag[a..i];i=(i+1).min(bytes.len());v}else{let a=i;while i<bytes.len()&&!bytes[i].is_ascii_whitespace()&&bytes[i]!=b'>'{i+=1;}&tag[a..i]};if name.eq_ignore_ascii_case(key){return Some(value.replace("&amp;","&").replace("&quot;","\"").replace("&#39;","'"));}}
 None
}
fn tags<'a>(html:&'a str,name:&str)->Vec<&'a str>{let lower=html.to_ascii_lowercase();let needle=format!("<{name}");let mut result=Vec::new();let mut i=0;while let Some(start)=lower[i..].find(&needle){let start=i+start;let Some(end)=lower[start..].find('>')else{break;};let end=start+end+1;if end-start<8192{result.push(&html[start..end]);}i=end;if result.len()>=1500{break;}}result}
fn meta(html:&str,key:&str)->Option<String>{tags(html,"meta").into_iter().find_map(|t|{let n=attr(t,"property").or_else(||attr(t,"name"))?;if n.eq_ignore_ascii_case(key){attr(t,"content")}else{None}})}
async fn html(client:&reqwest::Client,url:&str)->Result<String,String>{let mut r=client.get(url).timeout(Duration::from_secs(12)).send().await.map_err(|_|"官网页面连接失败/超时")?;if !r.status().is_success(){return Err(format!("官网页面 HTTP {}",r.status().as_u16()));}let mut data=Vec::new();while let Some(b)=r.chunk().await.map_err(|_|"官网页面读取失败")?{if data.len()+b.len()>2_500_000{break;}data.extend_from_slice(&b);}Ok(String::from_utf8_lossy(&data).into_owned())}
fn valid_url(url:&str)->Option<reqwest::Url>{let u=reqwest::Url::parse(url).ok()?;if u.scheme()!="https"||u.host_str().is_none_or(|h|h=="localhost"||h.ends_with(".local")||h.parse::<std::net::IpAddr>().is_ok()){return None;}Some(u)}
fn official_source(url:String,title:String,page:String,provider:&'static str)->Option<Source>{let mut resolved=reqwest::Url::parse(&url).or_else(|_|reqwest::Url::parse(&page).and_then(|base|base.join(&url))).ok()?;if resolved.scheme()=="http"{resolved.set_scheme("https").ok()?;}let u=valid_url(resolved.as_str())?;Some(Source{host:u.host_str()?.into(),url:u.to_string(),provider,title,evidence:"官方游戏页面主视觉".into(),official:true,page,w:0,h:0})}
fn steam_asset_sources(data:&serde_json::Value,id:u64,store:bool)->Vec<Source>{
 let mut result=Vec::new();let Some(items)=data["response"]["store_items"].as_array()else{return result;};
 for item in items{if item["id"].as_u64()!=Some(id){continue;}let assets=&item["assets"];let Some(format)=assets["asset_url_format"].as_str()else{continue;};
  let fields:&[(&str,&str)]=if store{&[("hero_capsule_2x","steam-store-keyart"),("hero_capsule","steam-store-keyart"),("main_capsule_2x","steam-store-keyart"),("main_capsule","steam-store-keyart")]}else{&[("library_hero_2x","steam-library-2x"),("library_hero","steam-library")]};
  for (field,provider) in fields{let Some(file)=assets[*field].as_str()else{continue;};let path=file.split('?').next().unwrap_or("");if ![".jpg",".png",".webp"].iter().any(|ext|path.ends_with(ext))||file.contains("..")||file.contains(":"){continue;}
   if !store&&!path.ends_with(&format!("{field}.jpg")){continue;}
   let relative=format.replace("${FILENAME}",file);let url=if relative.starts_with("https://"){relative}else if relative.starts_with(&format!("steam/apps/{id}/")){format!("https://shared.fastly.steamstatic.com/store_item_assets/{relative}")}else if relative.starts_with(&format!("store_item_assets/steam/apps/{id}/")){format!("https://shared.fastly.steamstatic.com/{relative}")}else{continue;};
   let Some(u)=valid_url(&url)else{continue;};if !u.host_str().is_some_and(|h|h.ends_with(".steamstatic.com"))||!u.path().contains(&format!("/apps/{id}/")){continue;}
   if let Some(mut source)=official_source(url,"Steam 官方宣传主视觉".into(),format!("https://store.steampowered.com/app/{id}"),provider){source.evidence=if store{"Steam StoreBrowse 官方商店宣传封面"}else{"Steam StoreBrowse 官方素材元数据返回的本作 library_hero"}.into();result.push(source);}
  }
 }result
}
fn steam_library_sources(data:&serde_json::Value,id:u64)->Vec<Source>{steam_asset_sources(data,id,false)}
fn demo_parent(data:&serde_json::Value,id:u64)->Option<u64>{data["response"]["store_items"].as_array()?.iter().find_map(|item|{if item["id"].as_u64()!=Some(id)||item["type"].as_u64()!=Some(1){return None;}item["related_items"]["parent_appid"].as_u64().filter(|parent|*parent>0&&*parent!=id)})}
fn gamespress_sources(page:&str,body:&str)->Vec<Source>{
 let links:Vec<_>=tags(body,"a").into_iter().filter_map(|tag|attr(tag,"href")).filter_map(|url|{let u=valid_url(&url)?;u.host_str().is_some_and(|h|h.ends_with(".blob.core.windows.net")||h.ends_with(".gamespress.com")).then_some((url,u))}).collect();
 let mut links=links;for image in tags(body,"img"){if let Some(url)=attr(image,"data-src"){if let Some(u)=valid_url(&url){if u.host_str().is_some_and(|h|h.ends_with(".gamespress.com")){links.push((u.to_string(),u));}}}}
 let mut result=Vec::new();for image in tags(body,"img"){
  let Some(title)=attr(image,"alt")else{continue;};let lower=title.to_lowercase();
  if !["keyart","key_art","key-art","wallpaper","cover"].iter().any(|s|lower.contains(s))||["screenshot","concept","season","_s0","_s1","crossover"].iter().any(|s|lower.contains(s)){continue;}
  let stem=normalize(title.rsplit_once('.').map_or(title.as_str(),|(s,_)|s));
  let Some((url,_))=links.iter().find(|(_,u)|normalize(u.path()).contains(&stem))else{continue;};
  if let Some(mut source)=official_source(url.clone(),title,page.into(),"publisher-press"){
   source.evidence="发行商官方新闻包 Key Art / Wallpaper 原图".into();
   if let Some(info)=attr(image,"data-info"){if let Some(dims)=info.split('|').nth(1){let parts:Vec<_>=dims.split('x').collect();if parts.len()==2{source.w=parts[0].trim().replace(',',"").parse().unwrap_or(0);source.h=parts[1].trim().replace(',',"").parse().unwrap_or(0);}}}
   if tier(source.w,source.h,true)>0{result.push(source);}
  }
 }result
}

fn identity(tags:&serde_json::Value,term:&str)->bool{tags.as_array().is_some_and(|v|v.iter().any(|t|t["name"].as_str().is_some_and(|n|normalize(&artwork_search_name(n))==normalize(term))))}
fn references(root:&Path,id:&str)->Vec<DynamicImage>{let mut result=Vec::new();for kind in ["hero","poster"]{if let Ok(entries)=std::fs::read_dir(root.join(kind)){let mut files:Vec<_>=entries.flatten().map(|e|e.path()).filter(|p|p.file_name().is_some_and(|n|{let n=n.to_string_lossy();(n.starts_with(&format!("{id}-steam"))||(kind=="poster"&&n.starts_with(&format!("{id}-battlenet-v2-"))))&&!n.contains("blur")&&!n.contains(".motion.")&&[".png",".jpg",".jpeg",".webp"].iter().any(|ext|n.ends_with(ext))})).collect();files.sort();for file in files.into_iter().take(2){if let Ok(image)=image::open(file){result.push(image.thumbnail(512,512));}}}}result}
async fn product_references(game:&GameRef,root:&Path,online:bool)->(Vec<DynamicImage>,Vec<String>){
 let directory=root.join("references");let mut refs=Vec::new();let mut issues=Vec::new();
 if let Ok(entries)=std::fs::read_dir(&directory){for entry in entries.flatten(){let path=entry.path();if path.file_name().is_some_and(|n|n.to_string_lossy().starts_with(&format!("{}-bnet-",game.id))){if let Ok(image)=image::open(path){refs.push(image.thumbnail(512,512));}}}}
 if !online||game.provider!="battlenet"||!refs.is_empty(){return (refs,issues);}
 for url in crate::battlenet_cache::product_reference_urls(game).into_iter().take(2){let Some(u)=valid_url(&url)else{continue;};let mut hash=std::collections::hash_map::DefaultHasher::new();url.hash(&mut hash);let destination=directory.join(format!("{}-bnet-{:016x}.png",game.id,hash.finish()));
  match crate::background_download::original(&url,u.host_str().unwrap_or("")).await{Ok(bytes)=>match media::inspect(&bytes){Ok(info) if !info.animated=>{if let Ok(image)=image::load_from_memory(&bytes){std::fs::create_dir_all(&directory).ok();if media::publish(&bytes,&destination).is_ok(){refs.push(image.thumbnail(512,512));}}},Ok(_)=>issues.push("战网商品参考图是动画，未用于静态主视觉匹配".into()),Err(_)=>issues.push("战网官方商品图返回了当前解码器不支持的格式".into())},Err(e)=>issues.push(format!("战网官方商品参考图：{e}"))}
 }
 (refs,issues)
}
fn visual_match(image:&DynamicImage,refs:&[DynamicImage])->bool{
 if refs.is_empty(){return false;}let image=image.thumbnail(768,768);
 for reference in refs{let ratio=reference.width() as f64/reference.height() as f64;for align in [0.0,0.25,0.5,0.75,1.0]{let (w,h)=(image.width(),image.height());let (cw,ch)=if w as f64/h as f64>ratio{((h as f64*ratio).round() as u32,h)}else{(w,(w as f64/ratio).round() as u32)};let crop=image.crop_imm(((w-cw) as f64*align) as u32,((h-ch) as f64*align) as u32,cw.max(1),ch.max(1));let a=crop.resize_exact(32,32,image::imageops::FilterType::Triangle).to_rgb8();let b=reference.resize_exact(32,32,image::imageops::FilterType::Triangle).to_rgb8();let difference=a.as_raw().iter().zip(b.as_raw()).map(|(a,b)|(*a as f64-*b as f64).abs()).sum::<f64>()/(32.0*32.0*3.0*255.0);if difference<=0.075{return true;}}}false
}
fn cached_evidence(c:&background::BackgroundChoice,refs:&[DynamicImage],term:&str)->Option<String>{
 let validation=semantic(&c.title);
 if validation.err().is_some_and(|e|e.starts_with("实机")||e.starts_with("独立 Logo")){return None;}
 if c.policy_version>=4&&!c.proof.is_empty(){return Some(c.proof.clone());}
 if normalize(&c.title).contains(&normalize(term)){if let Ok(proof)=validation{return Some(proof.into());}}
 if !refs.is_empty(){if let Ok(image)=image::open(&c.path){if visual_match(&image,refs){return Some("与本作平台官方宣传参考图视觉匹配".into());}}}
 None
}
pub fn cached(root:&Path,id:&str)->Vec<background::BackgroundChoice>{let mut result=Vec::new();if let Ok(entries)=std::fs::read_dir(root.join("hero")){for entry in entries.flatten(){let name=entry.file_name().to_string_lossy().into_owned();if name.starts_with(&format!("{id}-background-"))&&name.ends_with(".candidate.json"){if let Ok(bytes)=std::fs::read(entry.path()){if let Ok(c)=serde_json::from_slice::<background::BackgroundChoice>(&bytes){if c.path.is_file()&&c.path.parent()==Some(root.join("hero").as_path())&&!c.animated{result.push(c);}}}}}}result}
fn write_choice(root:&Path,id:&str,game_name:&str,path:PathBuf,w:u32,h:u32,title:String,provider:&str,proof:String,official:bool)->Result<background::BackgroundChoice,String>{
 let branding=crate::branding::verify(&path,game_name,id,root).ok();let has_title=branding.is_some();let proof=branding.map_or(proof.clone(),|branding|format!("{proof}；{branding}"));
 let score=if official{100}else{80};let choice=background::BackgroundChoice{id:path.file_name().unwrap().to_string_lossy().into(),path:path.clone(),width:w,height:h,animated:false,duration_ms:0,title,source:provider.into(),score,reason:format!("{}；{}",proof,grade(tier(w,h,official))),policy_version:POLICY,proof,title_detected:has_title};
 let extension=path.extension().unwrap_or_default().to_string_lossy();std::fs::write(path.with_extension(format!("{extension}.candidate.json")),serde_json::to_vec(&choice).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;let _=(root,id);Ok(choice)
}
fn art(c:&background::BackgroundChoice)->Artwork{Artwork{kind:"hero",path:c.path.clone(),source:match(c.title_detected,c.score>=100){(true,true)=>"background-promo-branded-official",(true,false)=>"background-promo-branded-verified",(false,true)=>"background-promo-official",(false,false)=>"background-promo-verified"},width:c.width,height:c.height}}
impl Scraper{
 pub async fn scrape_promo_background(&self,game:&GameRef,root:&Path,confirmed_sgdb:Option<u64>,confirmed_steam:Option<u64>,force:bool)->Result<Artwork,String>{
  let directory=root.join("hero");std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;let report_path=directory.join(format!("{}-promo-report.json",game.id));let mut audit=Audit{policy:POLICY,mode:if self.promo_network{"network"}else{"cache-only"}.into(),game_id:game.id.clone(),game:game.name.clone(),..Default::default()};let mut refs=references(root,&game.id);let mut best:Option<background::BackgroundChoice>=None;let (product_refs,reference_issues)=product_references(game,root,self.promo_network).await;refs.extend(product_refs);audit.issues.extend(reference_issues);
  for mut c in cached(root,&game.id){let official=c.score>=100&&c.policy_version>=4;let validation=semantic(&c.title);
   if let Ok((w,h))=image::image_dimensions(&c.path){c.width=w;c.height=h;}else{continue;}
   let proof=cached_evidence(&c,&refs,&artwork_search_name(&game.name));
   if let Some(proof)=proof{let t=tier(c.width,c.height,official);if t>0{let choice=match write_choice(root,&game.id,&game.name,c.path.clone(),c.width,c.height,c.title.clone(),&c.source,proof,official){Ok(choice)=>choice,Err(reason)=>{audit.candidates.push(serde_json::json!({"stage":"cache-title","title":c.title,"decision":"rejected","reason":reason}));continue;}};audit.candidates.push(serde_json::json!({"title":c.title,"stage":"cache","decision":"accepted","width":c.width,"height":c.height,"proof":choice.proof}));if best.as_ref().map_or(true,|b|preference(&choice)>preference(b)){best=Some(choice);}continue;}}
   audit.candidates.push(serde_json::json!({"title":c.title,"stage":"cache","decision":"rejected","width":c.width,"height":c.height,"reason":validation.err().unwrap_or("未达到宣传背景尺寸门槛")}));
  }
  // Original Steam library heroes provide a semantically safe fallback, never an upscaled 4K claim.
  if let Ok(entries)=std::fs::read_dir(&directory){for entry in entries.flatten(){let path=entry.path();let name=entry.file_name().to_string_lossy().into_owned();if !name.starts_with(&format!("{}-steam",game.id))||name.contains("blur"){continue;}if let Ok((w,h))=image::image_dimensions(&path){if tier(w,h,true)>0{let c=match write_choice(root,&game.id,&game.name,path,w,h,"Steam 官方宣传主视觉".into(),"steam-local","本作原始 Steam library_hero 缓存".into(),true){Ok(c)=>c,Err(reason)=>{audit.candidates.push(serde_json::json!({"stage":"steam-cache-title","decision":"rejected","reason":reason}));continue;}};if best.as_ref().map_or(true,|b|preference(&c)>preference(b)){best=Some(c);}}}}}
  let mut sources=Vec::new();let mut steam_id=if game.provider=="steam"{game.external_id.parse::<u64>().ok()}else{confirmed_steam.or_else(||(game.provider=="battlenet"&&game.external_id=="diablo_iv").then_some(2344520))};
  let canonical=artwork_search_name(&game.name);let term=match(game.provider.as_str(),game.external_id.as_str()){("battlenet","diablo_iv")=>"Diablo IV",("battlenet","diablo_iii"|"diablo_iii_cn")=>"Diablo III",_=>canonical.as_str()};
  if self.promo_network&&(force||best.as_ref().is_none_or(|b|tier(b.width,b.height,b.score>=100)<4)){
   if steam_id.is_none(){match self.steam_platform_identity(term,confirmed_sgdb,root,&game.id).await{Ok(Some(id))=>{steam_id=Some(id);audit.candidates.push(serde_json::json!({"stage":"platform-identity","decision":"accepted","steam_id":id,"proof":"SteamGridDB 已验证本体的唯一 Steam 平台编号"}));},Ok(None)=>(),Err(e)=>audit.issues.push(e)}}
   if steam_id.is_none(){match self.steam_candidates(term).await{Ok(hits)=>{steam_id=unique_exact(hits);if steam_id.is_none(){audit.issues.push("Steam 商店没有唯一精确匹配，本轮不猜测其他游戏的 App ID".into());}},Err(_)=>audit.issues.push("Steam 商店名称匹配请求失败/超时".into())}}
   let mut library_bases=Vec::new();
   let mut publisher=game.provider=="battlenet";
   if let Some(id)=steam_id{
    let mapped=crate::platform_identity::cached_steam_assets(root,&game.id,term,confirmed_sgdb,id,"hero");
    let mut exact_library=!mapped.is_empty();
    for url in mapped{let provider=if url.ends_with("library_hero_2x.jpg"){"steam-library-2x"}else{"steam-library"};if let Some(mut source)=official_source(url,format!("{term} Steam 官方宣传主视觉"),format!("https://store.steampowered.com/app/{id}"),provider){source.evidence="已验证平台映射引用的 Steam library_hero 原始素材".into();sources.push(source);}}
    if !exact_library{
    let request=serde_json::json!({"ids":[{"appid":id}],"context":{"language":"english","country_code":"US"},"data_request":{"include_assets":true}}).to_string();
    let library=async{let r=self.client.get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/").query(&[("input_json",request)]).timeout(Duration::from_secs(12)).send().await.map_err(|_|"Steam 官方素材元数据请求失败/超时")?;r.json::<serde_json::Value>().await.map_err(|_|"Steam 官方素材元数据格式无效")}.await;
    match library{Ok(data)=>{let mut found=steam_library_sources(&data,id);
     if found.is_empty(){if let Some(parent)=demo_parent(&data,id){let query=serde_json::json!({"ids":[{"appid":parent}],"context":{"language":"english","country_code":"US"},"data_request":{"include_assets":true}}).to_string();if let Ok(response)=self.client.get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/").query(&[("input_json",query)]).timeout(Duration::from_secs(12)).send().await{if let Ok(parent_data)=response.json::<serde_json::Value>().await{let mut parent_sources=steam_library_sources(&parent_data,parent);if parent_sources.is_empty(){parent_sources=steam_asset_sources(&parent_data,parent,true);}for source in &mut parent_sources{source.evidence=format!("Steam 官方确认 Demo {id} 属于本体 {parent}，采用所属本体宣传图");}found.extend(parent_sources);}}}}
     if found.is_empty(){found.extend(steam_asset_sources(&data,id,true));}
     exact_library=!found.is_empty();if !exact_library{audit.issues.push("Steam 官方素材元数据未提供本作 library_hero".into());}sources.extend(found);},Err(e)=>audit.issues.push(e.into())}
    }
    let lookup=async{let r=self.client.get("https://store.steampowered.com/api/appdetails").query(&[("appids",id.to_string()),("l","english".into())]).timeout(Duration::from_secs(12)).send().await.map_err(|_|"Steam 官网信息查询失败")?;let data=r.json::<serde_json::Value>().await.map_err(|_|"Steam 官网信息无效")?;Ok::<_,String>(data[id.to_string()]["data"].clone())}.await;
    match lookup{Ok(data)=>{if let Some(header)=data["header_image"].as_str().and_then(valid_url){if header.host_str().is_some_and(|h|h.ends_with(".steamstatic.com")||h.ends_with(".akamaihd.net"))&&header.path().contains(&format!("/apps/{id}/")){if let Ok(mut base)=header.join("."){base.set_query(None);library_bases.push(base.to_string());}}}publisher|=data["publishers"].to_string().to_lowercase().contains("blizzard");if let Some(website)=data["website"].as_str().and_then(valid_url){if let Ok(body)=html(&self.client,website.as_str()).await{let title=meta(&body,"og:title").unwrap_or_default();if normalize(&title).contains(&normalize(term)){if let Some(image)=meta(&body,"og:image"){if let Some(s)=official_source(image,format!("{} 官方页面主视觉",term),website.to_string(),"publisher-og"){sources.push(s);}}}else{audit.issues.push("官网页面未能对应本作，未采用其通用品牌图".into());}}}},Err(e)=>audit.issues.push(e)}
    if !exact_library{library_bases.insert(0,format!("https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{id}/"));
    library_bases.push(format!("https://shared.fastly.steamstatic.com/steam/apps/{id}/"));library_bases.dedup();
    for base in &library_bases{for (file,provider) in [("library_hero_2x.jpg","steam-library-2x"),("library_hero.jpg","steam-library")]{let url=format!("{base}{file}");if let Some(s)=official_source(url,format!("{term} Steam 官方宣传主视觉"),format!("https://store.steampowered.com/app/{id}"),provider){sources.push(s);}}}}
   }
   if publisher&&steam_id.is_none(){let slug=term.to_lowercase().replace(" iii","3").replace(" iv","4").replace(' ',"");let page=format!("https://{slug}.blizzard.com/en-us/");match html(&self.client,&page).await{Ok(body)=>{let title=meta(&body,"og:title").unwrap_or_default();if normalize(&title).contains(&normalize(term)){if let Some(image)=meta(&body,"og:image"){if let Some(source)=official_source(image,format!("{term} 官方页面主视觉"),page.clone(),"publisher-og"){sources.push(source);}}}else{audit.issues.push("发行商官网未给出对应本作的主视觉信息".into());}},Err(e)=>audit.issues.push(format!("发行商官网 {page}: {e}"))}}
   if publisher{let slug=term.replace(' ',"-");for suffix in ["-Launch-Press-Kit","-Launch-Date-Announcement",""]{let page=format!("https://blizzard.gamespress.com/{slug}{suffix}");match html(&self.client,&page).await{Ok(body)=>{let title=meta(&body,"og:title").unwrap_or_else(||body.split("<title>").nth(1).unwrap_or("").split("</title>").next().unwrap_or("").into());if normalize(&title).contains(&normalize(term)){let found=gamespress_sources(&page,&body);if found.is_empty(){audit.issues.push(format!("官方新闻包 {slug}{suffix}: 未解析到带原始尺寸的宣传图链接"));}sources.extend(found);}else{audit.issues.push(format!("官方新闻包 {slug}{suffix}: 页面标题未对应本作，不采用其中图片"));}},Err(e)=>audit.issues.push(format!("官方新闻包 {slug}{suffix}: {e}"))}}}
   if let Some(key)=&self.alpha_key{
    for (w,h) in [("3840","2160"),("2560","1440")]{let result=self.alpha_search(key,term,w,h,root).await;
     match result{Ok(data)=>{if data["success"]==false{audit.issues.push("Alpha Coders 授权或额度错误".into());break;}if let Some(items)=data["contents"].as_array(){let mut accepted=0;for v in items{let title=v["name"].as_str().unwrap_or("未分类壁纸");let evidence=format!("{} {} {}",title,v["alt_text"].as_str().unwrap_or(""),v["tags"]);let reason=if !identity(&v["tags"],term){Err("游戏标签不对应本作")}else{semantic(&evidence)};
       if let Ok(proof)=reason{if let Some(url)=v["src"].as_str(){if let Some(u)=valid_url(url){if u.host_str().is_some_and(|h|h.starts_with("images")&&h.ends_with(".alphacoders.com"))&&!u.path().contains("thumb-"){sources.push(Source{url:url.into(),provider:"alphacoders-promo",title:title.into(),evidence:proof.into(),official:false,page:v["url_page"].as_str().unwrap_or(url).into(),host:u.host_str().unwrap().into(),w:v["width"].as_u64().unwrap_or(0) as u32,h:v["height"].as_u64().unwrap_or(0) as u32});accepted+=1;}}}}
       else{audit.candidates.push(serde_json::json!({"title":title,"stage":"alpha-metadata","decision":"rejected","width":v["width"],"height":v["height"],"reason":reason.err()}));}
      }if accepted>0{break;}}},Err(e)=>{audit.issues.push(e);break;}}
    }
   }
   // Untagged high-resolution heroes must visually match this game's official key art.
   if let Some(key)=&self.key{let id=if confirmed_sgdb.is_some(){confirmed_sgdb}else{self.candidates(&game.name).await.ok().and_then(unique_exact)};if let Some(id)=id{
    let result=async{let r=self.client.get(format!("https://www.steamgriddb.com/api/v2/heroes/game/{id}")).bearer_auth(key).query(&[("types","static"),("nsfw","false"),("humor","false")]).timeout(Duration::from_secs(15)).send().await.map_err(|_|"SteamGridDB 查询失败")?;r.json::<serde_json::Value>().await.map_err(|_|"SteamGridDB 返回无效")}.await;
    match result{Ok(data)=>{if let Some(items)=data["data"].as_array(){for v in items{
     let w=v["width"].as_u64().unwrap_or(0) as u32;let h=v["height"].as_u64().unwrap_or(0) as u32;
     if tier(w,h,false)==0{audit.candidates.push(serde_json::json!({"stage":"sgdb-metadata","decision":"rejected","width":w,"height":h,"reason":"宽高/比例不足 2K 全屏标准"}));continue;}
     let evidence=semantic(&v["tags"].to_string()).unwrap_or("").to_string();
     if let Some(url)=v["url"].as_str(){if let Some(u)=valid_url(url){if matches!(u.host_str(),Some("cdn2.steamgriddb.com"|"cdn.steamgriddb.com")){sources.push(Source{url:url.into(),provider:"sgdb-promo",title:format!("{term} 宣传主视觉候选"),evidence,official:false,page:"https://www.steamgriddb.com/".into(),host:u.host_str().unwrap().into(),w,h});}}}
    }}},Err(e)=>audit.issues.push(e.into())}
   }}
  }
  sources.sort_by_key(|s|std::cmp::Reverse((s.official,tier(s.w,s.h,s.official))));
  let mut used=BTreeMap::new();let mut downloaded=0;
  for source in sources{if source.provider=="steam-library-2x"&&best.as_ref().is_some_and(|b|b.width>=3840&&b.height>=1240){continue;}if source.provider=="steam-library"&&best.as_ref().is_some_and(|b|tier(b.width,b.height,b.score>=100)>0){continue;}let safe_url=clean_url(&source.url);if used.insert(safe_url.clone(),true).is_some(){continue;}if downloaded>=8{audit.issues.push("本轮原图下载预算已用完，剩余候选留待下次更新".into());break;}downloaded+=1;
   let attempt=async{let bytes=crate::background_download::original(&source.url,&source.host).await?;let info=media::inspect(&bytes)?;if info.animated{return Err("大背景只接受静态宣传图".into());}let proof=if source.evidence.is_empty(){let image=image::load_from_memory(&bytes).map_err(|_|"原图解码失败")?;if !visual_match(&image,&refs){return Err("无宣传类型标签，且未通过本作官方宣传图的同源画面匹配".into());}"与本作平台官方宣传参考图视觉匹配".to_string()}else{source.evidence.clone()};let t=tier(info.width,info.height,source.official);if t==0{if source.provider=="steam-store-keyart"{if let Ok(image)=image::load_from_memory(&bytes){refs.push(image.thumbnail(512,512));}}return Err(format!("原生 {}×{} 不能作为高清全屏宣传图或官方横幅回退",info.width,info.height));}
    let mut hash=std::collections::hash_map::DefaultHasher::new();bytes.hash(&mut hash);let base=directory.join(format!("{}-background-{:016x}.png",game.id,hash.finish()));let (path,info)=media::publish(&bytes,&base)?;let c=write_choice(root,&game.id,&game.name,path,info.width,info.height,source.title.clone(),source.provider,proof,source.official)?;Ok::<_,String>(c)
   }.await;
   match attempt{Ok(c)=>{if c.score>=100{if let Ok(image)=image::open(&c.path){refs.push(image.thumbnail(512,512));}}audit.candidates.push(serde_json::json!({"title":c.title,"source":source.provider,"page":clean_url(&source.page),"decision":"accepted","width":c.width,"height":c.height,"proof":c.proof}));if best.as_ref().map_or(true,|b|preference(&c)>preference(b)){best=Some(c);}},Err(reason)=>audit.candidates.push(serde_json::json!({"title":source.title,"source":source.provider,"page":clean_url(&source.page),"decision":"rejected","reason":reason}))}
  }
  for mut c in cached(root,&game.id){if c.policy_version==POLICY||semantic(&c.title).err().is_some_and(|r|r.starts_with("实机")){continue;}if let Ok((w,h))=image::image_dimensions(&c.path){c.width=w;c.height=h;}else{continue;}if tier(c.width,c.height,false)==0{continue;}
   if let Some(proof)=cached_evidence(&c,&refs,term){let choice=match write_choice(root,&game.id,&game.name,c.path.clone(),c.width,c.height,c.title.clone(),&c.source,proof,false){Ok(c)=>c,Err(reason)=>{audit.candidates.push(serde_json::json!({"stage":"cache-reference-title","title":c.title,"decision":"rejected","reason":reason}));continue;}};audit.candidates.push(serde_json::json!({"title":c.title,"stage":"cache-after-official-reference","decision":"accepted","width":c.width,"height":c.height,"proof":choice.proof}));if best.as_ref().map_or(true,|b|preference(&choice)>preference(b)){best=Some(choice);}}
  }
  if let Some(c)=&best{audit.selected=Some(serde_json::json!({"path":c.path,"width":c.width,"height":c.height,"tier":grade(tier(c.width,c.height,c.score>=100)),"proof":c.proof,"source":c.source,"title_detected":c.title_detected}));}
  if !self.promo_network{if let Ok(bytes)=std::fs::read(&report_path){if let Ok(old)=serde_json::from_slice::<serde_json::Value>(&bytes){if let Some(items)=old["candidates"].as_array(){for item in items{if item["stage"]!="cache"&&item["stage"]!="cache-after-official-reference"{let mut historical=item.clone();historical["historical"]=true.into();audit.candidates.push(historical);}}}if let Some(issues)=old["issues"].as_array(){audit.issues.extend(issues.iter().filter_map(|s|s.as_str().map(|s|format!("上次联网：{}",s.trim_start_matches("上次联网：")))));}}}}
  std::fs::write(report_path,serde_json::to_vec_pretty(&audit).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
  best.as_ref().map(art).ok_or_else(||"本轮未取得有宣传类型证据的背景；旧的未验证壁纸不再参与自动选择，详见本作报告".into())
 }
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn clean_hd_background_and_separate_logo_win_without_preferring_a_low_resolution_banner(){
  let choice=|w,h,title_detected|background::BackgroundChoice{id:"test".into(),path:"test.png".into(),width:w,height:h,animated:false,duration_ms:0,title:"Official Key Art".into(),source:"official".into(),score:100,reason:"".into(),policy_version:POLICY,proof:"verified identity and key art".into(),title_detected};
  assert!(preference(&choice(2560,1440,false))>preference(&choice(3840,2160,true)));
  assert!(preference(&choice(3840,2160,true))>preference(&choice(1920,620,false)));
  assert!(preference(&choice(3840,2160,false))>preference(&choice(2560,1440,false)));
 }

 #[test]fn game_tag_and_resolution_are_not_promotional_proof(){assert!(semantic("Stellar Blade 8K wallpaper").is_err());assert!(semantic("Overwatch x Stellar Blade crossover key art").is_err());assert!(semantic("gameplay screenshot poster").is_err());assert!(semantic("Diablo IV Key Art").is_ok());assert!(semantic("official cover art").is_ok());}
 #[test]fn press_images_match_original_links_instead_of_download_buttons(){let body=r#"<a href="https://press.blob.core.windows.net/files/Game_KeyArt_3840x2160_png_jpgcopy.jpg?token=public" class="thumbView"><a href="/Files/File"><img alt="Game_KeyArt_3840x2160.png" data-info="Windows PNG image|3840 x 2160">"#;let found=gamespress_sources("https://publisher.gamespress.com/Game",body);assert_eq!(found.len(),1);assert!(found[0].url.contains("blob.core.windows.net"));assert_eq!((found[0].w,found[0].h),(3840,2160));}
 #[test]fn press_gallery_reads_lazy_original_image_sources(){let body=r#"<img alt="Game_KeyArt_3840x2160.png" data-info="Windows PNG image|3840 x 2160" data-src="https://imguscdn.gamespress.com/cdn/files/Game_KeyArt_3840x2160_png_jpgcopy.jpg?width=300&amp;sig=publicsignature&amp;sv=version" src="/dummy.png">"#;let sources=gamespress_sources("https://publisher.gamespress.com/Game",body);assert_eq!(sources.len(),1);assert!(sources[0].url.contains("width=300"));assert!(sources[0].url.contains("sig=publicsignature"));assert!(sources[0].url.contains("sv=version"));}
 #[test]fn wide_key_art_uses_pixels_remaining_after_screen_crop(){assert_eq!(tier(9000,4132,false),4);assert_eq!(tier(5120,1440,false),3);assert_eq!(tier(3840,1240,true),1);assert_eq!(tier(3840,1240,false),0);}
 #[test]fn demo_fallback_requires_official_parent_relationship(){let mut v=serde_json::json!({"response":{"store_items":[{"id":10,"type":1,"related_items":{"parent_appid":20}}]}});assert_eq!(demo_parent(&v,10),Some(20));assert_eq!(demo_parent(&v,11),None);v["response"]["store_items"][0]["type"]=0.into();assert_eq!(demo_parent(&v,10),None);}
 #[test]fn official_images_resolve_relative_paths_and_require_https(){let relative=official_source("/keyart.jpg".into(),"game".into(),"https://publisher.example/game".into(),"test").unwrap();assert_eq!(relative.url,"https://publisher.example/keyart.jpg");let legacy=official_source("http://cdn.example/keyart.jpg".into(),"game".into(),"https://publisher.example/game".into(),"test").unwrap();assert!(legacy.url.starts_with("https://"));assert!(official_source("http://127.0.0.1/keyart.jpg".into(),"game".into(),"https://publisher.example/game".into(),"test").is_none());}
 #[test]fn steam_hero_uses_its_own_hash_from_official_metadata(){let fixture=serde_json::json!({"response":{"store_items":[{"id":42,"assets":{"asset_url_format":"steam/apps/42/${FILENAME}?t=123","header":"headerhash/header.jpg","library_hero":"herohash/library_hero.jpg","library_hero_2x":"herohash/library_hero_2x.jpg"}}]}});let result=steam_library_sources(&fixture,42);assert_eq!(result.len(),2);assert!(result.iter().all(|s|s.url.contains("/42/herohash/library_hero")));assert!(steam_library_sources(&fixture,43).is_empty());}
 #[test]fn reference_comparison_matches_same_art_and_rejects_other_art(){
 let mut pixels=image::RgbImage::new(640,360);for (x,y,p) in pixels.enumerate_pixels_mut(){*p=image::Rgb([(x%255) as u8,(y%255) as u8,((x+y)%255) as u8]);}
 let full=DynamicImage::ImageRgb8(pixels);let reference=full.thumbnail(512,512);assert!(visual_match(&full,&[reference.clone()]));
 let foreign=DynamicImage::ImageRgb8(image::RgbImage::from_pixel(640,360,image::Rgb([0,0,0])));assert!(!visual_match(&foreign,&[reference]));
 assert!(semantic("Overwatch x Stellar Blade official key art").is_err());
 }
 #[test]fn legacy_high_resolution_wallpaper_does_not_bypass_policy(){
 let mut c=background::BackgroundChoice{id:"test".into(),path:"missing.png".into(),width:8192,height:4096,animated:false,duration_ms:0,title:"Stellar Blade 8K Wallpaper".into(),source:"alpha".into(),score:0,reason:"".into(),policy_version:0,proof:"".into(),title_detected:false};
 assert!(cached_evidence(&c,&[],"Stellar Blade").is_none());
 c.title="Stellar Blade Key Art".into();assert!(cached_evidence(&c,&[],"Stellar Blade").is_some());
 c.title="Other Game Key Art".into();assert!(cached_evidence(&c,&[],"Stellar Blade").is_none());
 c.title="Overwatch x Stellar Blade crossover key art".into();c.policy_version=POLICY;c.proof="old proof".into();assert!(cached_evidence(&c,&[],"Stellar Blade").is_none());
 }
 #[test]fn tiers_distinguish_full_screen_and_banner(){assert_eq!(tier(3840,2160,false),4);assert_eq!(tier(2560,1440,false),3);assert_eq!(tier(1920,1080,false),0);assert_eq!(tier(1920,1080,true),2);assert_eq!(tier(3840,1240,false),0);assert_eq!(tier(3840,1240,true),1);}
 #[test]fn meta_reader_handles_attribute_order_and_entities(){assert_eq!(meta("<meta content='https://site/x.jpg?a=1&amp;b=2' property = 'og:image'>","og:image").as_deref(),Some("https://site/x.jpg?a=1&b=2"));}
}

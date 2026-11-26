//! Epic account library uses its own login session; never reads launcher passwords/cookies.
use serde::{Deserialize,Serialize};
use serde_json::Value;
#[cfg(test)]use serde_json::json;
use std::{collections::HashSet,time::Duration};

const CLIENT:&str="34a02cf8f4414e29b15921876da36f9a";
const CLIENT_SECRET:&str="daafbccc737745039dffe53d94fc76cf";
const TOKEN:&str="https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token";
const LIBRARY:&str="https://library-service.live.use1a.on.epicgames.com/library/api/public/items";

#[derive(Clone,Serialize,Deserialize)]
pub struct Session{pub account_id:String,#[serde(default,rename="displayName")]pub display_name:String,pub access_token:String,pub refresh_token:String,pub expires_at:String}
#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct OwnedGame{pub app_name:String,pub namespace:String,pub catalog_id:String,pub name:String,pub description:Option<String>,pub poster:Option<String>,pub landscape:Option<String>}
#[derive(Serialize,Deserialize)]pub struct Snapshot{pub games:Vec<OwnedGame>,pub total_records:usize,pub skipped:usize,pub unresolved:usize}

pub fn login_url()->String{format!("https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Fapi%2Fredirect%3FclientId%3D{CLIENT}%26responseType%3Dcode")}
pub fn authorization_code(input:&str)->Result<String,String>{
    let input=input.trim();
    let code=if input.starts_with('{'){serde_json::from_str::<Value>(input).ok().and_then(|v|v["authorizationCode"].as_str().map(str::to_owned)).ok_or("登录结果中没有一次性登录码")?}else{input.to_owned()};
    if !(16..=2048).contains(&code.len())||!code.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'-'|'_'|'.')){return Err("一次性登录码格式无效，请复制 Epic 官方页面返回的结果".into());}
    Ok(code)
}
pub fn client()->Result<reqwest::Client,String>{reqwest::Client::builder().timeout(Duration::from_secs(25)).redirect(reqwest::redirect::Policy::none()).user_agent("GameDeck/1.0").build().map_err(|_|"无法初始化 Epic 连接".into())}
async fn token(grant:&str,value:&str)->Result<Session,String>{
    let field=if grant=="authorization_code"{"code"}else{"refresh_token"};
    let response=client()?.post(TOKEN).basic_auth(CLIENT,Some(CLIENT_SECRET)).form(&[("grant_type",grant),(field,value),("token_type","eg1")]).send().await.map_err(|_|"无法连接 Epic 登录服务，请检查网络后重试")?;
    if !response.status().is_success(){return Err("Epic 登录码已失效或账号需要重新登录，请重新打开登录页".into());}
    response.json::<Session>().await.map_err(|_|"Epic 登录服务返回格式异常".into())
}
pub async fn connect(code:&str)->Result<Session,String>{token("authorization_code",&authorization_code(code)?).await}
pub async fn refresh(session:&Session)->Result<Session,String>{token("refresh_token",&session.refresh_token).await}

fn identity(value:&str)->bool{!value.is_empty()&&value.len()<=200&&value.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'_'|'-'|'.'))}
fn image(item:&Value,types:&[&str])->Option<String>{
    let images=item["keyImages"].as_array()?;
    for kind in types{for i in images{if i["type"]==*kind{let url=i["url"].as_str()?;let parsed=reqwest::Url::parse(url).ok()?;if parsed.scheme()=="https"&&matches!(parsed.host_str(),Some("cdn1.epicgames.com"|"cdn2.epicgames.com"|"cdn.epicgames.com"|"cdn1.unrealengine.com"|"cdn2.unrealengine.com")){return Some(url.into());}}}}
    None
}
pub fn parse_owned(record:&Value,item:&Value)->Result<Option<OwnedGame>,String>{
    let app=record["appName"].as_str().ok_or("缺少 Epic 应用编号")?;
    let namespace=record["namespace"].as_str().ok_or("缺少 Epic namespace")?;
    let catalog=record["catalogItemId"].as_str().ok_or("缺少 Epic 商品编号")?;
    if ![app,namespace,catalog].iter().all(|v|identity(v)){return Err("Epic 游戏身份格式异常".into());}
    if item["id"]!=catalog||item["namespace"]!=namespace{return Err("Epic 商品信息与账号库身份不一致".into());}
    if namespace=="ue"||app.starts_with("UE_")||record["sandboxName"]=="fab-listing-live"{return Ok(None);}
    if item["mainGameItem"]["id"].as_str().is_some_and(|s|!s.is_empty()){return Ok(None);}
    let categories:Vec<_>=item["categories"].as_array().into_iter().flatten().filter_map(|v|v["path"].as_str()).collect();
    if !categories.iter().any(|v|*v=="games"||v.starts_with("games/"))||categories.iter().any(|v|["addons","dlc","digitalextras","software","plugins","engines","editors"].iter().any(|c|v.split('/').any(|p|p==*c))){return Ok(None);}
    let name=item["title"].as_str().filter(|n|!n.trim().is_empty()).ok_or("Epic 商品缺少名称")?;
    let lower=name.to_lowercase();if ["bonus content","digital soundtrack","art book","artbook","hd wallpaper","mod tools"].iter().any(|s|lower.contains(s)){return Ok(None);}
    // Third-party clients can have no Epic download artifact. The Windows library
    // still owns these games, and installation must be delegated to the official client.
    if let Some(releases)=item["releaseInfo"].as_array(){
        if !releases.is_empty()&&releases.iter().all(|r|r["platform"].as_array().is_some_and(|p|!p.is_empty()))&&!releases.iter().any(|r|r["platform"].as_array().is_some_and(|p|p.iter().any(|p|p=="Windows"))){return Ok(None);}
    }
    Ok(Some(OwnedGame{app_name:app.into(),namespace:namespace.into(),catalog_id:catalog.into(),name:name.into(),description:item["description"].as_str().filter(|s|!s.is_empty()).map(str::to_owned),poster:image(item,&["DieselGameBoxTall","OfferImageTall","Thumbnail"]),landscape:image(item,&["DieselGameBox","OfferImageWide","DieselStoreFrontWide"])}))
}
pub fn page(value:&Value,seen:&mut HashSet<String>)->Result<(Vec<Value>,Option<String>),String>{
    let records=value["records"].as_array().ok_or("Epic 账号库返回格式异常")?.clone();
    if !value["responseMetadata"].is_object(){return Err("Epic 账号库缺少分页信息，未覆盖上次同步结果".into());}
    let cursor=value["responseMetadata"]["nextCursor"].as_str().filter(|c|!c.is_empty()).map(str::to_owned);
    if let Some(c)=&cursor{if !seen.insert(c.clone()){return Err("Epic 分页游标重复，已停止同步并保留上次游戏库".into());}}
    Ok((records,cursor))
}
pub async fn library(session:&Session)->Result<Snapshot,String>{
    let http=client()?;let mut records=Vec::new();let mut cursor=None;let mut seen=HashSet::new();
    loop{
        let mut query=vec![("includeMetadata","true"),("platform","Windows")];if let Some(c)=cursor.as_deref(){query.push(("cursor",c));}
        let response=http.get(LIBRARY).query(&query).bearer_auth(&session.access_token).send().await.map_err(|_|"Epic 游戏库连接失败，请稍后重试")?;
        if !response.status().is_success(){return Err(if response.status().as_u16()==401{"Epic 登录已过期，请重新连接账号"}else{"Epic 游戏库暂时不可用，已保留上次同步结果"}.into());}
        let value=response.json::<Value>().await.map_err(|_|"Epic 账号库返回格式异常")?;let (items,next)=page(&value,&mut seen)?;
        records.extend(items);if records.len()>50_000||seen.len()>1000{return Err("Epic 游戏库响应超过安全上限".into());}
        cursor=next;if cursor.is_none(){break;}
    }
    let total_records=records.len();let mut tasks=tokio::task::JoinSet::new();let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(4));
    for record in records{
        let(http,token,limit)=(http.clone(),session.access_token.clone(),limit.clone());
        tasks.spawn(async move{
            let _permit=limit.acquire_owned().await.map_err(|_|"同步任务中断")?;
            if record["namespace"]=="ue"||record["appName"].as_str().is_some_and(|s|s.starts_with("UE_"))||record["sandboxName"]=="fab-listing-live"{return Ok(None);}
            let ns=record["namespace"].as_str().filter(|v|identity(v)).ok_or("无效 namespace")?;let cid=record["catalogItemId"].as_str().filter(|v|identity(v)).ok_or("无效商品编号")?;
            let item=if record["metadata"]["title"].is_string()&&record["metadata"]["categories"].is_array()&&record["metadata"]["id"]==cid&&record["metadata"]["namespace"]==ns{record["metadata"].clone()}else{
                let response=http.get(format!("https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace/{ns}/bulk/items")).query(&[("id",cid),("includeDLCDetails","true"),("includeMainGameDetails","true"),("locale","en-US")]).bearer_auth(token).send().await.map_err(|_|"商品信息连接失败")?;
                if !response.status().is_success(){return Err("商品信息暂不可用");}response.json::<Value>().await.map_err(|_|"商品信息格式异常")?[cid].clone()
            };
            parse_owned(&record,&item).map_err(|_|"商品信息未能确认")
        });
    }
    let(mut games,mut skipped,mut unresolved)=(Vec::new(),0,0);let mut apps=HashSet::new();
    while let Some(result)=tasks.join_next().await{match result{Ok(Ok(Some(game)))=>{if apps.insert(game.app_name.clone()){games.push(game);}},Ok(Ok(None))=>skipped+=1,_=>unresolved+=1}}
    games.sort_by(|a,b|a.name.cmp(&b.name));Ok(Snapshot{games,total_records,skipped,unresolved})
}

impl super::Scraper{
    pub async fn epic_covers(&self,game:&OwnedGame,id:&str,root:&std::path::Path)->Vec<super::Artwork>{
        use std::hash::{Hash,Hasher};let mut result=Vec::new();
        for (kind,url) in [("poster",&game.poster),("landscape",&game.landscape)]{
            let Some(url)=url else{continue};let mut hash=std::collections::hash_map::DefaultHasher::new();url.hash(&mut hash);
            let dir=root.join(kind);if std::fs::create_dir_all(&dir).is_err(){continue;}let path=dir.join(format!("{id}-epic-account-{:016x}.png",hash.finish()));
            let dimensions=if path.is_file(){image::image_dimensions(&path).ok()}else{self.fetch_artwork(url,&path,1920,kind).await.ok().flatten()};
            if let Some((width,height))=dimensions{result.push(super::Artwork{kind,path,source:"epic-account",width,height});}
        }result
    }
}

#[cfg(test)]mod tests{use super::*;
 fn data()->(Value,Value){(json!({"appName":"GameApp","namespace":"space","catalogItemId":"item"}),json!({"id":"item","namespace":"space","title":"My Game","categories":[{"path":"games"}],"releaseInfo":[{"appId":"GameApp","platform":["Windows"]}]}))}
 #[test]fn imports_owned_base_games_but_not_dlc_extras_or_catalog_mismatches(){let(r,mut i)=data();assert!(parse_owned(&r,&i).unwrap().is_some());i["mainGameItem"]=json!({"id":"base"});assert!(parse_owned(&r,&i).unwrap().is_none());let(r,mut i)=data();i["categories"]=json!([{"path":"digitalextras"},{"path":"games"}]);assert!(parse_owned(&r,&i).unwrap().is_none());let(r,mut i)=data();i["id"]="other".into();assert!(parse_owned(&r,&i).is_err());}
 #[test]fn all_library_pages_are_required_and_repeated_cursors_fail(){let mut seen=HashSet::new();let value=json!({"records":[{"appName":"A"}],"responseMetadata":{"nextCursor":"page2"}});assert_eq!(page(&value,&mut seen).unwrap().0.len(),1);assert!(page(&value,&mut seen).is_err());assert!(page(&json!({"records":[],"responseMetadata":{}}),&mut seen).unwrap().1.is_none());}
 #[test]fn accepts_only_one_time_code_and_never_an_arbitrary_url(){assert_eq!(authorization_code(r#"{"authorizationCode":"0123456789abcdef"}"#).unwrap(),"0123456789abcdef");assert!(authorization_code("https://elsewhere.test/code").is_err());assert!(authorization_code("short").is_err());}
 #[test]fn third_party_client_games_do_not_require_an_epic_download_artifact(){let(r,mut i)=data();i["releaseInfo"]=json!([]);assert!(parse_owned(&r,&i).unwrap().is_some());i["releaseInfo"]=json!([{"appId":"MacArtifact","platform":["Mac"]}]);assert!(parse_owned(&r,&i).unwrap().is_none());}
}

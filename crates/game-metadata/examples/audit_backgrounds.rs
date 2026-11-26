#[allow(dead_code)]#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod credentials;
use serde_json::{Value,json};
fn dimensions(items:&[Value])->Vec<String>{let mut dims:Vec<_>=items.iter().filter_map(|x|Some(format!("{}x{}",x["width"].as_u64()?,x["height"].as_u64()?))).collect();dims.sort();dims.dedup();dims}
async fn fetch(request:reqwest::RequestBuilder)->Value{match request.send().await{Err(_)=>json!({"error":"连接失败或超时"}),Ok(r)=>{let status=r.status().as_u16();match r.json::<Value>().await{Ok(v)=>{if v.get("success").and_then(|v|v.as_bool())==Some(false){return json!({"status":status,"api_failure":true});}json!({"status":status,"payload":v})},Err(_)=>json!({"status":status,"error":"返回内容不是 JSON"})}}}}
async fn heroes(client:&reqwest::Client,key:&str,id:u64,kind:&str)->Value{
 let raw=fetch(client.get(format!("https://www.steamgriddb.com/api/v2/heroes/game/{id}")).bearer_auth(key).query(&[("types",kind),("nsfw","false"),("humor","false")])).await;
 let Some(items)=raw["payload"]["data"].as_array()else{return raw;};
 json!({"status":raw["status"],"returned":items.len(),"dimensions":dimensions(items),"meeting_minimum":items.iter().filter(|v|game_metadata::background::suitable(v["width"].as_u64().unwrap_or(0) as u32,v["height"].as_u64().unwrap_or(0) as u32)).count()})
}
fn main(){tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
 let root=std::env::current_dir().unwrap();let key=credentials::load_sgdb(&root.join("release/data")).unwrap().unwrap();let alpha=credentials::load(&root.join("release/data")).unwrap().unwrap();let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(18)).redirect(reqwest::redirect::Policy::none()).user_agent("GameDeck/1.0").build().unwrap();
 let input:Vec<Value>=serde_json::from_slice(&std::fs::read("docs/media-reasons/input.json").unwrap()).unwrap();let semaphore=std::sync::Arc::new(tokio::sync::Semaphore::new(4));let mut tasks=tokio::task::JoinSet::new();
 for mut row in input {let semaphore=semaphore.clone();let key=key.clone();let alpha=alpha.clone();let client=client.clone();tasks.spawn(async move{
  let _permit=semaphore.acquire_owned().await.unwrap();let name=row["name"].as_str().unwrap().to_string();let canonical=game_metadata::artwork_search_name(&name);row["query"]=canonical.clone().into();
  let scraper=game_metadata::Scraper::new(Some(key.clone())).unwrap();let mut id=row["confirmed"].as_u64();
  if id.is_none(){match scraper.candidates(&name).await{Ok(hits)=>{let exact:Vec<_>=hits.iter().filter(|h|h.exact).collect();row["sgdb_matching"]=json!({"returned":hits.len(),"exact":exact.len(),"names":hits.iter().map(|h|h.name.clone()).collect::<Vec<_>>()});if exact.len()==1{id=Some(exact[0].id);}},Err(_)=>row["sgdb_matching"]=json!({"error":"身份查询失败或超时"})}}
  if let Some(id)=id{row["static_heroes"]=heroes(&client,&key,id,"static").await;row["animated_heroes"]=heroes(&client,&key,id,"animated").await;}
  if row["needs_alpha"]==true {
   let raw=fetch(client.get("https://api.alphacoders.com/4.0/search").query(&[("auth",alpha.as_str()),("term",canonical.as_str()),("content_type","wallpaper"),("sort","highest_rated")])).await;
   if let Some(items)=raw["payload"]["contents"].as_array(){let matched:Vec<_>=items.iter().filter(|v|v["tags"].as_array().is_some_and(|tags|tags.iter().any(|t|t["name"].as_str().is_some_and(|n|game_metadata::normalize(n)==game_metadata::normalize(&canonical))))).cloned().collect();let games:Vec<_>=matched.iter().filter(|v|v["tags"].as_array().is_some_and(|tags|tags.iter().any(|t|t["id"]==961||t["name"].as_str().is_some_and(|n|["videogame","videogames","电子游戏"].contains(&game_metadata::normalize(n).as_str()))))).cloned().collect();
    row["alpha"]=json!({"status":raw["status"],"returned":items.len(),"exact_tag_matches":matched.len(),"game_tag_matches":games.len(),"exact_dimensions":dimensions(&matched),"game_dimensions":dimensions(&games),"matching_titles":matched.iter().take(4).map(|v|v["name"].clone()).collect::<Vec<_>>(),"meeting_minimum":games.iter().filter(|v|game_metadata::background::suitable(v["width"].as_u64().unwrap_or(0) as u32,v["height"].as_u64().unwrap_or(0) as u32)).count()});
   }else{row["alpha"]=raw;}
  }row
 });}
 let mut rows=Vec::new();while let Some(result)=tasks.join_next().await{match result{Ok(row)=>{println!("Audited {}: {}",rows.len()+1,row["display_name"]);rows.push(row);std::fs::write("docs/media-reasons/audit.json",serde_json::to_vec_pretty(&rows).unwrap()).unwrap();},Err(e)=>println!("Audit worker failed: {e}")}}
 println!("Read-only audit complete: {} games",rows.len());
});}

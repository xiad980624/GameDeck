use super::*;
pub(crate) struct Quota { next:tokio::time::Instant, limited_until:Option<tokio::time::Instant> }
impl Default for Quota {fn default()->Self{Self{next:tokio::time::Instant::now(),limited_until:None}}}
fn fingerprint(key:&str)->String{use std::hash::{Hash,Hasher};let mut h=std::collections::hash_map::DefaultHasher::new();key.hash(&mut h);format!("{:016x}",h.finish())}
fn cooldown_path(root:&Path,key:&str)->PathBuf{root.join("provider-status").join(format!("alpha-cooldown-{}.json",fingerprint(key)))}
fn epoch()->u64{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()}
fn cached_wait(root:&Path,key:&str,now:u64)->u64{std::fs::read(cooldown_path(root,key)).ok().and_then(|b|serde_json::from_slice::<serde_json::Value>(&b).ok()).and_then(|v|v["until"].as_u64()).unwrap_or(0).saturating_sub(now)}
impl Scraper{
 pub(crate) async fn alpha_search(&self,key:&str,term:&str,width:&str,height:&str,root:&Path)->Result<serde_json::Value,String>{
  // Clones in a batch share the quota. A provider failure is never image-quality evidence.
  let mut quota=self.alpha_quota.lock().await;let persisted=cached_wait(root,key,epoch());if persisted>0{return Err(format!("Alpha Coders 限流冷却中，约 {} 秒后可重试（已保存冷却状态，并非没有图片）",persisted));}let now=tokio::time::Instant::now();
  if let Some(until)=quota.limited_until{if until>now{return Err(format!("Alpha Coders 限流冷却中，约 {} 秒后可重试（并非没有图片）",(until-now).as_secs()+1));}}
  if quota.next>now{tokio::time::sleep_until(quota.next).await;}
  let client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(20)).build().map_err(|_|"壁纸查询初始化失败")?;
  let response=client.get("https://api.alphacoders.com/4.0/search").query(&[("auth",key),("content_type","wallpaper"),("term",term),("width",width),("height",height),("operator","min"),("sort","highest_rated")]).send().await.map_err(|_|"Alpha Coders 连接失败/超时")?;
  quota.next=tokio::time::Instant::now()+Duration::from_secs(2);
  if response.status()==reqwest::StatusCode::TOO_MANY_REQUESTS{let wait=response.headers().get(reqwest::header::RETRY_AFTER).and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<u64>().ok()).unwrap_or(60).clamp(1,86400);quota.limited_until=Some(tokio::time::Instant::now()+Duration::from_secs(wait));let path=cooldown_path(root,key);if let Some(dir)=path.parent(){std::fs::create_dir_all(dir).ok();}let state=serde_json::json!({"provider":"alphacoders","http_status":429,"until":epoch()+wait,"retry_after_seconds":wait});if let Ok(bytes)=serde_json::to_vec(&state){let _=std::fs::write(path,bytes);}return Err(format!("Alpha Coders HTTP 429，按限流冷却 {} 秒后重试（并非没有图片）",wait));}
  if !response.status().is_success(){return Err(format!("Alpha Coders HTTP {}",response.status().as_u16()));}
  response.json().await.map_err(|_|"Alpha Coders 返回无效".into())
 }
}

#[cfg(test)]mod tests{use super::*;#[test]fn cooldown_survives_clients_without_affecting_other_keys(){let root=std::env::temp_dir().join(format!("gamedeck-quota-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));let path=cooldown_path(&root,"test-key-a");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,br#"{"until":160}"#).unwrap();assert_eq!(cached_wait(&root,"test-key-a",100),60);assert_eq!(cached_wait(&root,"test-key-b",100),0);assert_eq!(cached_wait(&root,"test-key-a",161),0);std::fs::remove_file(path).unwrap();std::fs::remove_dir(root.join("provider-status")).unwrap();std::fs::remove_dir(root).unwrap();}}

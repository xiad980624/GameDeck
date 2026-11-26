//! Inspect public artwork records using only the saved SteamGridDB resource key.
#[allow(dead_code)]
#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod credentials;
fn main(){tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
 let root=std::env::current_dir().unwrap().join("release/data");let key=credentials::load_sgdb(&root).unwrap().expect("configured artwork source");let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build().unwrap();
 for steam in std::env::args().skip(1){
  let response=client.get(format!("https://www.steamgriddb.com/api/v2/games/steam/{steam}")).query(&[("platformdata","steam")]).bearer_auth(&key).send().await.unwrap();let status=response.status();let value=response.json::<serde_json::Value>().await.unwrap();
  std::fs::write(root.join("artwork/reports").join(format!("steam-{steam}-artwork-identity.json")),serde_json::to_vec_pretty(&value).unwrap()).unwrap();
  println!("Steam {steam} status={status}: {}",value);
 }
});}

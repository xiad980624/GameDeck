#[allow(dead_code)]#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod credentials;
fn main(){tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
 let root=std::env::current_dir().unwrap();
 if std::env::args().nth(1).as_deref()==Some("--local") {let bytes=std::fs::read(root.join("docs/motion-validation/download.motion.webp")).unwrap();match game_metadata::media::inspect(&bytes){Ok(info)=>println!("Accepted short loop: {} ms",info.duration_ms),Err(e)=>{assert!(e.contains("超过 8 秒"));println!("Rejected existing live sample: {e}");}}return;}
 let key=credentials::load_sgdb(&root.join("release/data")).unwrap().expect("configured key");
 let client=reqwest_client();let hits=game_metadata::Scraper::new(Some(key.clone())).unwrap().candidates("Dota 2").await.unwrap();let id=hits.iter().find(|h|h.exact).unwrap().id;
 let response=client.get(format!("https://www.steamgriddb.com/api/v2/grids/game/{id}")).bearer_auth(key).query(&[("types","animated"),("nsfw","false")]).send().await.unwrap().error_for_status().unwrap();
 let data:serde_json::Value=response.json().await.unwrap();let hits=data["data"].as_array().expect("assets");
 for item in hits.iter().take(12){let url=item["url"].as_str().unwrap();let parsed=reqwest::Url::parse(url).unwrap();if !matches!(parsed.host_str(),Some("cdn2.steamgriddb.com"|"cdn.steamgriddb.com")){continue;}
  let mut response=client.get(url).send().await.unwrap().error_for_status().unwrap();let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.unwrap(){if bytes.len()+chunk.len()>24_000_000{break;}bytes.extend_from_slice(&chunk);}
  if let Ok(info)=game_metadata::media::inspect(&bytes){if !info.animated{continue;}let dir=root.join("docs/motion-validation");std::fs::create_dir_all(&dir).unwrap();let (path,saved)=game_metadata::media::publish(&bytes,&dir.join("download.png")).unwrap();assert_eq!(std::fs::read(&path).unwrap(),bytes);std::fs::write(dir.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"provider":"steamgriddb","game":"Dota 2","width":saved.width,"height":saved.height,"animated":saved.animated,"duration_ms":saved.duration_ms,"frames":saved.frames,"bytes_preserved":true,"path":path})).unwrap()).unwrap();println!("Animated original preserved: {} x {}, {} ms, {} bytes",saved.width,saved.height,saved.duration_ms,bytes.len());return;}
 }
 panic!("No usable animated original found");
});}
fn reqwest_client()->reqwest::Client{reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).redirect(reqwest::redirect::Policy::none()).build().unwrap()}

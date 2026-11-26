#[allow(dead_code)]#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod credentials;
fn main(){tokio::runtime::Builder::new_multi_thread().worker_threads(3).enable_all().build().unwrap().block_on(async{
 let motion_only=std::env::var("GAMEDECK_MOTION_ONLY").as_deref()==Ok("1");
 let root=std::env::current_dir().unwrap().join("release/data");let storage=game_storage::Storage::open(&root.join("database/gamedeck.db")).await.unwrap();
 let scraper=game_metadata::Scraper::new(credentials::load_sgdb(&root).unwrap()).unwrap().with_alpha_key(credentials::load(&root).unwrap());
 let filter=std::env::var("GAMEDECK_REFRESH_FILTER").unwrap_or_default();let selected:Vec<_>=filter.split('|').filter(|s|!s.is_empty()).collect();
 let games:Vec<_>=storage.games_for_scraping().await.unwrap().into_iter().filter(|g|selected.is_empty()||selected.iter().any(|s|g.id==*s)).collect();let total=games.len();println!("Refreshing {total} library games");
 let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(3));let mut tasks=tokio::task::JoinSet::new();
 for game in games{let db=storage.clone();let scraper=scraper.clone();let root=root.clone();let limit=limit.clone();tasks.spawn(async move{
  let _permit=limit.acquire_owned().await.unwrap();let artwork=root.join("artwork");let confirmed=db.confirmed_sgdb_id(&game.id).await.unwrap_or(None);let steam=db.confirmed_steam_artwork(&game.id).await.unwrap_or(None);
  let mut report=serde_json::json!({"id":game.id,"game":game.name,"background":null,"motion_cards":0,"errors":[]});let mut errors=Vec::new();
  if !motion_only&&!db.custom_hero(&game.id).await.unwrap_or(true){match scraper.scrape_background(&game,&artwork,confirmed,steam,true).await{
   Ok(art)=>{if let Err(e)=db.save_scraped_asset(&game.id,&art).await{errors.push(e);}else{report["background"]=serde_json::json!({"width":art.width,"height":art.height,"animated":art.path.to_string_lossy().contains(".motion."),"source":art.source});}},
   Err(e)=>errors.push(e),
  }}else if !motion_only{report["background_locked"]=true.into();}
  let custom=db.artwork_choices(&game.id).await.unwrap_or_default();let (cards,issues)=scraper.scrape_motion_cards_for(&game,&artwork,confirmed,if motion_only{&["landscape"]}else{&["poster","landscape"]}).await;errors.extend(issues);
  let mut count=0;for art in cards{if custom.iter().any(|a|a["kind"]==art.kind&&a["manual"]==true){continue;}match db.save_scraped_asset(&game.id,&art).await{Ok(_)=>count+=1,Err(e)=>errors.push(e)}}
  report["motion_cards"]=count.into();report["background_candidates"]=game_metadata::background::choices(&artwork,&game.id).len().into();report["errors"]=serde_json::json!(errors);report
 });}
 std::fs::create_dir_all("docs/media-refresh").unwrap();let result_path=if motion_only{"docs/media-refresh/landscape.json"}else if selected.is_empty(){"docs/media-refresh/result.json"}else{"docs/media-refresh/recheck.json"};let mut reports=Vec::new();while let Some(result)=tasks.join_next().await{match result{Ok(r)=>{println!("{}/{} {}: background={} motion_cards={}",reports.len()+1,total,r["game"],r["background"],r["motion_cards"]);reports.push(r);},Err(e)=>{println!("Worker failed: {e}");}}
  std::fs::write(result_path,serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
 }
 println!("Refresh completed: {} games",reports.len());
});}

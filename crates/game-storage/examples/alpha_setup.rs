#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod alpha_credentials;
fn main(){
 let root=std::env::current_dir().unwrap().join("release/data");
 let mut key=String::new();std::io::stdin().read_line(&mut key).unwrap();
 if !key.trim().is_empty(){alpha_credentials::save(&root,&key).unwrap();}
 let key=alpha_credentials::load(&root).unwrap().expect("Alpha key required");
 tokio::runtime::Runtime::new().unwrap().block_on(async{
  let storage=game_storage::Storage::open(&root.join("database/gamedeck.db")).await.unwrap();
  let scraper=game_metadata::Scraper::new(alpha_credentials::load_sgdb(&root).unwrap()).unwrap().with_alpha_key(Some(key));let mut reports=Vec::new();
  for game in storage.games_for_scraping().await.unwrap().into_iter().filter(|g|(g.provider=="battlenet"||g.external_id=="570")&&std::env::args().nth(1).map_or(true,|id|g.external_id==id)){
   if storage.custom_hero(&game.id).await.unwrap(){continue;}
   match scraper.scrape_background(&game,&root.join("artwork"),None,None,true).await{
    Ok(art)=>{reports.push(serde_json::json!({"game":game.name,"width":art.width,"height":art.height,"source":art.source}));storage.save_scraped_asset(&game.id,&art).await.unwrap();},
    Err(error)=>reports.push(serde_json::json!({"game":game.name,"error":error})),
   }
  }
  std::fs::create_dir_all("docs/alpha-validation").unwrap();let path=if std::env::args().nth(1).is_some(){"docs/alpha-validation/recheck.json"}else{"docs/alpha-validation/result.json"};std::fs::write(path,serde_json::to_vec_pretty(&reports).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&reports).unwrap());
 });
}

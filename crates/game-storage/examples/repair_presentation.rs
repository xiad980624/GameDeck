#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"] mod credentials;
fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async{
 let data=std::env::current_dir().unwrap().join("release/data");let root=data.join("artwork");
 let db=game_storage::Storage::open(&data.join("database/gamedeck.db")).await.unwrap();
 let normalized=db.normalize_titles().await.unwrap();
 let games=db.list_games().await.unwrap();
 for game in &games{if game.original_name!="Death Stranding"{continue;}
  if let Some(path)=game.landscape.as_ref().filter(|p|p.contains("7b84f06af72058fb")){
   let review=game_metadata::motion_review::entry(&root,&game.id,"landscape",std::path::Path::new(path)).unwrap();game_metadata::motion_review::set_review(&root,review,false).unwrap();
   let original=root.join("hero").join(format!("{}-background-08ebae2c9076fd87.png",game.id));
   let asset=game_metadata::Artwork{kind:"landscape",path:original,source:"restored-promo-static",width:1920,height:620};db.save_scraped_asset(&game.id,&asset).await.unwrap();
  }
 }
 if std::env::args().any(|a|a=="--fetch-titles"){
  let scraper=game_metadata::Scraper::new(credentials::load_sgdb(&data).unwrap()).unwrap();
  println!("{}",db.repair_titles(scraper,&root).await.unwrap());
 }
 let missing:Vec<_>=db.list_games().await.unwrap().into_iter().filter(|g|!g.hidden&&g.logo.is_none()).map(|g|g.original_name).collect();
 println!("{}",serde_json::json!({"normalized_titles":normalized,"missing_titles":missing,"death_card":"official Steam artwork; crossover rejected"}));
});}

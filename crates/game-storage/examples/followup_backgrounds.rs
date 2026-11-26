//! Revalidate every public cache locally, then retry actual gaps through the normal scraper.
#[allow(dead_code)]
#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"] mod credentials;
fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async {
 let data=std::env::current_dir().unwrap().join("release/data");let root=data.join("artwork");let reports=root.join("reports");
 let db=game_storage::Storage::open(&data.join("database/gamedeck.db")).await.unwrap();
 let previous:serde_json::Value=serde_json::from_slice(&std::fs::read(reports.join("background-refresh-latest.json")).unwrap()).unwrap();
 let mut targets=std::collections::BTreeSet::new();for e in previous["entries"].as_array().unwrap(){if e["status"]=="no-verified-resource"{targets.insert(e["id"].as_str().unwrap().to_owned());}}
 let before=db.list_games().await.unwrap();
 let scraper=game_metadata::Scraper::new(credentials::load_sgdb(&data).unwrap()).unwrap().with_alpha_key(credentials::load(&data).unwrap());
 let mut report=if std::env::var("GAMEDECK_FOLLOWUP_SKIP_CACHE").as_deref()==Ok("1"){previous}else{db.refresh_promo_backgrounds(scraper.clone().cached_promo_only(),&root,|p|println!("Cache revalidation {}/{}",p["processed"],p["total"])).await.unwrap()};
 assert!(report["errors"].as_array().unwrap().is_empty());
 for entry in report["entries"].as_array().unwrap(){if entry["status"]=="no-verified-resource"{targets.insert(entry["id"].as_str().unwrap().to_owned());}}
 let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(3));let mut tasks=tokio::task::JoinSet::new();
 for game in db.public_games_for_scraping().await.unwrap().into_iter().filter(|g|targets.contains(&g.id)){
  let (db,scraper,root,limit)=(db.clone(),scraper.clone(),root.clone(),limit.clone());
  tasks.spawn(async move{let _permit=limit.acquire_owned().await.unwrap();db.refresh_promo_game(&scraper,&game,&root).await});
 }
 while let Some(result)=tasks.join_next().await{let entry=result.unwrap().unwrap();println!("Online gap recheck: {} {}",entry["game"],entry["status"]);let old=report["entries"].as_array_mut().unwrap().iter_mut().find(|e|e["id"]==entry["id"]).unwrap();*old=entry;}
 report["saved"]=report["entries"].as_array().unwrap().iter().filter(|e|e["status"]=="updated").count().into();
 let after=db.list_games().await.unwrap();let mut verification=Vec::new();
 for entry in report["entries"].as_array().unwrap(){
  let game=after.iter().find(|g|Some(g.id.as_str())==entry["id"].as_str()).unwrap();
  let ok=match entry["status"].as_str().unwrap_or(""){
   "updated"=>game.hero.as_deref()==entry["path"].as_str()&&game.hero.as_ref().is_some_and(|p|std::path::Path::new(p).is_file())&&entry["display_cache"].as_str().is_some_and(|p|std::path::Path::new(p).is_file()),
   "manual-preserved"=>db.custom_hero(&game.id).await.unwrap(),"no-verified-resource"=>game.hero.is_none(),_=>false};
  let old=before.iter().find(|g|g.id==game.id).unwrap();let cards=old.poster==game.poster&&old.landscape==game.landscape;
  assert!(ok&&cards,"Persisted selection or card mismatch: {}",game.name);
  verification.push(serde_json::json!({"id":game.id,"game":game.name,"verified":ok,"cards_preserved":cards,"hero":game.hero,"source":game.hero_source}));
 }
 assert_eq!(verification.len() as u64,report["total"].as_u64().unwrap());
 std::fs::write(reports.join("background-verification.json"),serde_json::to_vec_pretty(&verification).unwrap()).unwrap();
 std::fs::write(reports.join("background-refresh-latest.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
 println!("Verified {} public games; card selections unchanged",verification.len());
});}

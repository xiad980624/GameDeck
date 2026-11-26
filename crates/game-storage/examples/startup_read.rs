fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async {
 let root=std::env::current_dir().unwrap().join("release/data");let start=std::time::Instant::now();
 let db=game_storage::Storage::open(&root.join("database/gamedeck.db")).await.unwrap();let opened=start.elapsed().as_secs_f64()*1000.;
 let query=std::time::Instant::now();let games=db.list_games().await.unwrap();let listed=query.elapsed().as_secs_f64()*1000.;
 let image=std::time::Instant::now();let mut cached=false;if let Some(path)=games.iter().find(|g|!g.hidden).and_then(|g|g.hero.as_ref()){cached=game_metadata::display::background(std::path::Path::new(path),&root.join("artwork")).is_ok();}
 let result=serde_json::json!({"database_open_ms":opened,"library_query_ms":listed,"hero_cache_ms":image.elapsed().as_secs_f64()*1000.,"hero_available":cached,"public_games":games.iter().filter(|g|!g.hidden).count()});
 std::fs::create_dir_all("docs/startup-performance").unwrap();std::fs::write("docs/startup-performance/backend-before.json",serde_json::to_vec_pretty(&result).unwrap()).unwrap();println!("{result}");
});}

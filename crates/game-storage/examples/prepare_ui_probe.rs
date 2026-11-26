//! Copy only selected artwork into an isolated native test profile.
fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async{
 let destination=std::path::PathBuf::from(std::env::args().nth(1).expect("isolated profile directory"));
 let source=std::env::current_dir().unwrap().join("release/data/artwork");
 let db=game_storage::Storage::open(&destination.join("database/gamedeck.db")).await.unwrap();let games=db.list_games().await.unwrap();
 let options=sqlx::sqlite::SqliteConnectOptions::new().filename(destination.join("database/gamedeck.db"));let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
 let mut copied=0;
 for game in games.into_iter().filter(|g|!g.hidden){
  for (kind,original) in [("hero",game.hero),("logo",game.logo),("poster",game.poster),("landscape",game.landscape)]{
   let Some(original)=original else{continue};let original_path=std::path::Path::new(&original);
   let display=if kind=="hero"{game_metadata::display::background(original_path,&source).ok()}else{Some(original_path.to_owned())};let Some(display)=display else{continue};
   let dir=destination.join("artwork").join(kind);std::fs::create_dir_all(&dir).unwrap();let target=dir.join(format!("{}-{}",game.id,display.file_name().unwrap().to_string_lossy()));
   if std::fs::copy(display,&target).is_err(){continue;}
   sqlx::query("UPDATE game_assets SET path=? WHERE game_id=? AND asset_type=? AND path=?").bind(target.to_string_lossy().as_ref()).bind(&game.id).bind(kind).bind(original).execute(&pool).await.unwrap();copied+=1;
  }
 }
 pool.close().await;println!("Copied {copied} artwork selections");
});}

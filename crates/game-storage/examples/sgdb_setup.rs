#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod credentials;
fn main(){
 let root=std::env::current_dir().unwrap().join("release/data");
 let mut key=String::new();std::io::stdin().read_line(&mut key).unwrap();
 tokio::runtime::Runtime::new().unwrap().block_on(async{
  let scraper=game_metadata::Scraper::new(Some(key.trim().into())).unwrap();
  match scraper.candidates("Dota 2").await {
   Ok(hits)=>{credentials::save_sgdb(&root,&key).unwrap();assert!(credentials::load_sgdb(&root).unwrap().is_some());println!("Key validated and encrypted; Dota 2 matches: {}",hits.len());},
   Err(_)=>{eprintln!("SteamGridDB API validation failed; credential not changed");std::process::exit(1);}
  }
 });
}

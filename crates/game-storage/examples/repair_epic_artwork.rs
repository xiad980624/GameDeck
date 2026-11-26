//! Repair existing Epic artwork only; no Epic authentication, entitlement sync or launch.
#[allow(dead_code)]
#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"]mod credentials;
fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async{
    let data=std::env::current_dir().unwrap().join("release/data");
    let db=game_storage::Storage::open(&data.join("database/gamedeck.db")).await.unwrap();
    let scraper=game_metadata::Scraper::new(credentials::load_sgdb(&data).unwrap()).unwrap().with_alpha_key(credentials::load(&data).unwrap());
    let report=db.repair_epic_artwork(scraper,&data.join("artwork"),true,|progress|println!("{progress}")).await.unwrap();
    let entries:Vec<_>=db.list_games().await.unwrap().into_iter().filter(|game|game.platform=="epic"&&!game.hidden).map(|g|serde_json::json!({"game":g.name,"hero":g.hero.is_some_and(|p|std::path::Path::new(&p).is_file()),"logo":g.logo.is_some_and(|p|std::path::Path::new(&p).is_file())})).collect();
    println!("{report}");println!("{}",serde_json::to_string(&entries).unwrap());
});}

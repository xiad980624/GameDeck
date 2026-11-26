use game_metadata::{GameRef,Scraper};
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let root=std::path::PathBuf::from(args.get(1).expect("output directory required"));
 let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
 let games=[GameRef{id:"probe-steam-570".into(),name:"Dota 2".into(),provider:"steam".into(),external_id:"570".into()},GameRef{id:"probe-blizzard-d3".into(),name:"Diablo III".into(),provider:"battlenet".into(),external_id:"diablo_iii".into()}];
 let reports=runtime.block_on(async{let mut reports=Vec::new();let scraper=Scraper::new(None).unwrap();for game in games{
  let result=scraper.scrape_background(&game,&root,None,None,true).await;
  reports.push(match result{Ok(art)=>serde_json::json!({"game":game.name,"success":true,"path":art.path,"width":art.width,"height":art.height,"source":art.source}),Err(error)=>serde_json::json!({"game":game.name,"success":false,"error":error})});
 }reports});
 std::fs::create_dir_all(&root).unwrap();std::fs::write(root.join("result.json"),serde_json::to_vec_pretty(&reports).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&reports).unwrap());
}

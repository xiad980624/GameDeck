//! Apply the split background/title presentation to every public library entry.
use game_storage::Storage;
fn main()->Result<(),String>{tauri::async_runtime::block_on(async{
 let mut args=std::env::args().skip(1);let data=std::path::PathBuf::from(args.next().ok_or("portable data directory required")?).canonicalize().map_err(|e|e.to_string())?;
 let data=std::path::PathBuf::from(data.to_string_lossy().strip_prefix(r"\\?\").unwrap_or(&data.to_string_lossy()).to_string());
 let logos_only=args.next().as_deref()==Some("--logos-only");
 let root=data.join("artwork");let storage=Storage::open(&data.join("database/gamedeck.db")).await?;
 let before=storage.list_games().await?;let cards:Vec<_>=before.iter().map(|g|(&g.id,&g.poster,&g.landscape)).collect();
 let refs=storage.public_games_for_scraping().await?;let caches=game_discovery::steam::artwork_cache_roots();let mut logos=0;
 for game in &refs{
  let official=game_metadata::import_title_references(game,&caches,&root)?;
  let old=storage.asset_path(&game.id,"logo").await?;
  let candidate=official.into_iter().next().or_else(||old.as_ref().and_then(|p|{
   let bytes=std::fs::read(p).ok()?;let info=game_metadata::media::inspect(&bytes).ok()?;
   Some(game_metadata::Artwork{kind:"logo",path:p.into(),source:"title-logo-original",width:info.width,height:info.height})
  }));
  if let Some(mut art)=candidate{if game_metadata::branding::usable_logo(&art.path){art.source=if art.source=="steam-local"{"title-logo-official"}else{"title-logo-original"};storage.save_scraped_asset(&game.id,&art).await?;logos+=1;}}
 }
 println!("cached original title logos: {logos}/{}",refs.len());
 if logos_only{return Ok(());}
 let scraper=game_metadata::Scraper::new(None)?.cached_promo_only();
 let report=storage.refresh_promo_backgrounds(scraper,&root,|p|{println!("background checks: {}/{}",p["processed"],p["total"]);}).await?;
 let after=storage.list_games().await?;
 let cards_preserved=cards.iter().all(|(id,poster,landscape)|after.iter().find(|g|&g.id==*id).is_some_and(|g|&g.poster==*poster&&&g.landscape==*landscape));
 if !cards_preserved{return Err("card artwork changed unexpectedly".into());}
 let summary=serde_json::json!({"policy":game_metadata::promo::POLICY,"public_games":refs.len(),"cached_original_title_logos":logos,"backgrounds_saved":report["saved"],"cards_preserved":cards_preserved,"mode":"cache-only; no original images modified or translated"});
 std::fs::write(root.join("reports/title-layout-latest.json"),serde_json::to_vec_pretty(&summary).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 println!("{summary}");Ok(())
})}

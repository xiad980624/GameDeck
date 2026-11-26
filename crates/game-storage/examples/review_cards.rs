use std::path::{Path,PathBuf};
fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async{
 let root=std::env::current_dir().unwrap().join("release/data/artwork");let db=game_storage::Storage::open(&root.parent().unwrap().join("database/gamedeck.db")).await.unwrap();
 let rows:Vec<serde_json::Value>=serde_json::from_slice(&std::fs::read("docs/card-review/index.json").unwrap()).unwrap();let mut reports=Vec::new();
 for row in rows{let index=row["index"].as_u64().unwrap();let gid=row["game_id"].as_str().unwrap();let kind=row["kind"].as_str().unwrap();let path=Path::new(row["path"].as_str().unwrap());let rejected=[19,21].contains(&index);
  let entry=game_metadata::motion_review::entry(&root,gid,kind,path).unwrap();game_metadata::motion_review::set_review(&root,entry,!rejected).unwrap();
  if rejected{
   let mut bases:Vec<PathBuf>=std::fs::read_dir(root.join(kind)).unwrap().flatten().map(|e|e.path()).filter(|p|p.file_name().unwrap().to_string_lossy().starts_with(&format!("{gid}-steam-"))).collect();bases.sort_by_key(|p|image_dimensions(p).map(|(w,h)|w as u64*h as u64).unwrap_or(0));let base=bases.pop().or_else(||{let p=root.join(kind).join(format!("{gid}.png"));p.is_file().then_some(p)}).or_else(||{let p=root.join("hero").join(format!("{gid}-background-08ebae2c9076fd87.png"));p.is_file().then_some(p)}).expect("static fallback");let (width,height)=image_dimensions(&base).unwrap();let asset=game_metadata::Artwork{kind:if kind=="poster"{"poster"}else{"landscape"},path:base,source:"restored-promo-static",width,height};db.save_scraped_asset(gid,&asset).await.unwrap();
  }else{let asset=game_metadata::motion_review::approved(&root,gid,if kind=="poster"{"poster"}else{"landscape"}).unwrap();db.save_scraped_asset(gid,&asset).await.unwrap();}
  reports.push(serde_json::json!({"game":row["name"],"kind":kind,"approved":!rejected,"reason":if rejected{"缺少稳定清晰的游戏名称或 Logo，恢复静态宣传封面"}else{"宣传卡面或持续带有清晰游戏名称/Logo，允许短循环动画"}}));
 }
 std::fs::write("docs/card-review/result.json",serde_json::to_vec_pretty(&reports).unwrap()).unwrap();println!("Reviewed {} cards; {} approved, {} restored to static",reports.len(),reports.iter().filter(|r|r["approved"]==true).count(),reports.iter().filter(|r|r["approved"]==false).count());
});}
fn image_dimensions(p:&Path)->Result<(u32,u32),String>{let b=std::fs::read(p).map_err(|e|e.to_string())?;let i=game_metadata::media::inspect(&b)?;Ok((i.width,i.height))}

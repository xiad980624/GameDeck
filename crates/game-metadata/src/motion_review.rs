//! Positive review records bind an animation to a game and a card role.
use super::*;
use std::hash::{Hash,Hasher};
#[derive(Default,Serialize,Deserialize)]pub struct Reviews{#[serde(default)]pub approved:Vec<Entry>,#[serde(default)]pub rejected:Vec<Entry>}
#[derive(Clone,Serialize,Deserialize)]pub struct Entry{pub game_id:String,pub kind:String,pub file:String,pub fingerprint:String}
#[derive(Serialize)]pub struct Choice{pub id:String,pub path:PathBuf,pub width:u32,pub height:u32,pub duration_ms:u64,pub approved:bool}
fn fingerprint(bytes:&[u8])->String{let mut h=std::collections::hash_map::DefaultHasher::new();bytes.hash(&mut h);format!("{:016x}",h.finish())}
fn load(root:&Path)->Reviews{std::fs::read(root.join("motion-review.json")).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default()}
pub fn entry(root:&Path,game_id:&str,kind:&str,path:&Path)->Result<Entry,String>{
 if !["poster","landscape"].contains(&kind){return Err("动图仅用于竖封面和横卡片".into());}
 let path=path.canonicalize().map_err(|_|"动图不存在")?;let folder=root.join(kind).canonicalize().map_err(|_|"卡片目录不存在")?;
 if path.parent()!=Some(folder.as_path()){return Err("动图路径无效".into());}let file=path.file_name().and_then(|n|n.to_str()).ok_or("动图名称无效")?;
 if !file.starts_with(&format!("{game_id}-"))||!file.contains(".motion."){return Err("动图与游戏不匹配".into());}
 if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>24_000_000{return Err("动图文件过大".into());}
 let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;let info=media::inspect(&bytes)?;if !info.animated||!asset_dimensions(kind,info.width,info.height){return Err("卡片尺寸或格式无效".into());}
 Ok(Entry{game_id:game_id.into(),kind:kind.into(),file:file.into(),fingerprint:fingerprint(&bytes)})
}
pub fn set_review(root:&Path,e:Entry,approved:bool)->Result<(),String>{let mut data=load(root);data.approved.retain(|v|!(v.game_id==e.game_id&&v.kind==e.kind&&v.file==e.file));data.rejected.retain(|v|!(v.game_id==e.game_id&&v.kind==e.kind&&v.file==e.file));if approved{data.approved.retain(|v|!(v.game_id==e.game_id&&v.kind==e.kind));data.approved.push(e);}else{data.rejected.push(e);}let temporary=root.join("motion-review.pending");std::fs::write(&temporary,serde_json::to_vec_pretty(&data).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;std::fs::rename(temporary,root.join("motion-review.json")).map_err(|e|e.to_string())}
pub fn approved(root:&Path,game_id:&str,kind:&'static str)->Option<Artwork>{
 for e in load(root).approved.into_iter().filter(|e|e.game_id==game_id&&e.kind==kind){let path=root.join(kind).join(&e.file);let Ok(actual)=entry(root,game_id,kind,&path)else{continue;};if actual.fingerprint!=e.fingerprint{continue;}let info=media::inspect(&std::fs::read(&path).ok()?).ok()?;return Some(Artwork{kind,path,source:"reviewed-promo-motion",width:info.width,height:info.height});}None
}
pub fn choices(root:&Path,game_id:&str,kind:&str)->Result<Vec<Choice>,String>{
 if !["poster","landscape"].contains(&kind){return Err("请选择卡片类型".into());}let reviews=load(root);let mut result=Vec::new();
 if let Ok(entries)=std::fs::read_dir(root.join(kind)){for e in entries.flatten(){let name=e.file_name().to_string_lossy().into_owned();if !name.starts_with(&format!("{game_id}-"))||!name.contains(".motion.")||reviews.rejected.iter().any(|r|r.game_id==game_id&&r.kind==kind&&r.file==name){continue;}
  let Ok(actual)=entry(root,game_id,kind,&e.path())else{continue;};let info=media::inspect(&std::fs::read(e.path()).map_err(|e|e.to_string())?)?;let approved=reviews.approved.iter().any(|r|r.file==name&&r.fingerprint==actual.fingerprint);
  result.push(Choice{id:name,path:e.path(),width:info.width,height:info.height,duration_ms:info.duration_ms,approved});if result.len()>=24{break;}
 }}result.sort_by_key(|c|std::cmp::Reverse(c.approved));Ok(result)
}
#[cfg(test)]mod tests{use super::*;#[test]fn rejects_background_roles_and_foreign_paths(){assert!(entry(Path::new("root"),"game","hero",Path::new("x")).is_err());assert!(choices(Path::new("root"),"game","../hero").is_err());}
 #[test]fn only_reviewed_unchanged_card_can_be_applied(){
  let root=std::env::temp_dir().join(format!("gamedeck-review-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));let dir=root.join("poster");std::fs::create_dir_all(&dir).unwrap();let path=dir.join("game-test.motion.gif");let mut bytes=include_bytes!("../tests/fixtures/two-frame.gif").to_vec();bytes[6..8].copy_from_slice(&600u16.to_le_bytes());bytes[8..10].copy_from_slice(&900u16.to_le_bytes());std::fs::write(&path,&bytes).unwrap();
  assert!(approved(&root,"game","poster").is_none());let e=entry(&root,"game","poster",&path).unwrap();set_review(&root,e,true).unwrap();assert!(approved(&root,"game","poster").is_some());bytes[11]^=1;std::fs::write(&path,&bytes).unwrap();assert!(approved(&root,"game","poster").is_none());let e=entry(&root,"game","poster",&path).unwrap();set_review(&root,e,false).unwrap();assert!(choices(&root,"game","poster").unwrap().is_empty());std::fs::remove_file(path).unwrap();std::fs::remove_file(root.join("motion-review.json")).unwrap();std::fs::remove_dir(dir).unwrap();std::fs::remove_dir(root).unwrap();
 }
}

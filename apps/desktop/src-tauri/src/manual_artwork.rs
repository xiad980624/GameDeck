use std::{fs,path::PathBuf};
use tauri::{Manager,State};
use crate::AppState;
#[derive(serde::Serialize,serde::Deserialize)]
pub struct Draft {id:String,game_id:String,kind:String,path:PathBuf,width:u32,height:u32}
fn valid_kind(kind:&str)->Result<(),String>{if ["hero","poster","landscape","logo"].contains(&kind){Ok(())}else{Err("无效图片类型".into())}}
fn draft_root(app:&tauri::AppHandle)->Result<PathBuf,String>{Ok(app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork/drafts"))}
fn draft_name(root:&std::path::Path,id:&str,extension:&str)->Result<PathBuf,String>{uuid::Uuid::parse_str(id).map_err(|_|"无效预览标识")?;Ok(root.join(format!("{id}.{extension}")))}
#[tauri::command]
pub async fn artwork_choices(state:State<'_,AppState>,game_id:String)->Result<Vec<serde_json::Value>,String>{state.storage.artwork_choices(&game_id).await}
#[tauri::command]
pub async fn prepare_artwork(app:tauri::AppHandle,game_id:String,kind:String,path:String)->Result<Draft,String>{
    valid_kind(&kind)?;uuid::Uuid::parse_str(&game_id).map_err(|_|"无效游戏标识")?;
    let root=draft_root(&app)?;fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    tauri::async_runtime::spawn_blocking(move||{
        let id=uuid::Uuid::new_v4().to_string();let base=draft_name(&root,&id,"png")?;
        let metadata=fs::metadata(&path).map_err(|e|e.to_string())?;if metadata.len()>24_000_000{return Err("素材超过 24 MB".into());}
        let bytes=fs::read(&path).map_err(|e|e.to_string())?;let check=game_metadata::media::inspect(&bytes)?;
        if check.animated&&!matches!(kind.as_str(),"poster"|"landscape"){return Err("动图仅用于竖封面和横卡片，大背景请使用静态宣传图".into());}
        let (dest,info)=game_metadata::media::publish(&bytes,&base)?;
        let (width,height)=(info.width,info.height);
        let draft=Draft{id,game_id,kind,path:dest,width,height};
        fs::write(draft_name(&root,&draft.id,"json")?,serde_json::to_vec(&draft).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;Ok(draft)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn discard_artwork(app:tauri::AppHandle,draft_id:String)->Result<(),String>{
    let root=draft_root(&app)?;for ext in ["png","gif","motion.gif","motion.webp","json"]{let path=draft_name(&root,&draft_id,ext)?;if path.exists(){fs::remove_file(path).map_err(|e|e.to_string())?;}}Ok(())
}
#[tauri::command]
pub async fn commit_artwork(app:tauri::AppHandle,state:State<'_,AppState>,game_id:String,kind:String,draft_id:String)->Result<(),String>{
    valid_kind(&kind)?;let root=draft_root(&app)?;
    let draft:Draft=serde_json::from_slice(&fs::read(draft_name(&root,&draft_id,"json")?).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if draft.game_id!=game_id||draft.kind!=kind{return Err("预览图片与当前游戏或类型不一致，请重新选图".into());}
    let source=draft.path.clone();
    if !["png","gif","motion.gif","motion.webp"].iter().any(|ext|draft_name(&root,&draft_id,ext).ok().as_ref()==Some(&source)){return Err("预览路径无效".into());}
    let suffix=source.file_name().and_then(|s|s.to_str()).and_then(|s|s.strip_prefix(&format!("{}.",draft.id))).ok_or("预览格式无效")?;
    let folder=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork").join(&kind);fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let dest=folder.join(format!("{}-manual-{}.{}",draft.game_id,draft.id,suffix));fs::copy(source,&dest).map_err(|e|e.to_string())?;
    if let Err(error)=state.storage.save_manual_asset(&game_id,&kind,&dest,draft.width,draft.height).await{let _=fs::remove_file(&dest);return Err(error);}
    discard_artwork(app,draft_id).await
}
#[tauri::command]
pub async fn open_artwork_website(site:String)->Result<(),String>{
    let url=match site.as_str(){"alphacoders"=>"https://alphacoders.com/video-game-4k-wallpapers","steamgriddb"=>"https://www.steamgriddb.com/","wallhaven"=>"https://wallhaven.cc/",_=>return Err("未知图片网站".into())};
    game_core::background_command("explorer.exe").arg(url).spawn().map_err(|e|e.to_string())?;Ok(())
}
#[cfg(test)]mod tests{use super::*;#[test]fn draft_paths_reject_traversal(){assert!(draft_name(std::path::Path::new("root"),"../other","png").is_err());assert!(valid_kind("landscape").is_ok());assert!(valid_kind("../hero").is_err());}}

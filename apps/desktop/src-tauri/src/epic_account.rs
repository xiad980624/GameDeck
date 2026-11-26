use game_metadata::epic::{self,Session};
use serde_json::{Value,json};
use std::{path::{Path,PathBuf},sync::atomic::{AtomicBool,Ordering}};
use tauri::{AppHandle,State,Manager,Emitter};
use crate::AppState;

static SYNCING:AtomicBool=AtomicBool::new(false);
struct Guard;
impl Guard{fn acquire()->Result<Self,String>{SYNCING.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).map_err(|_|"Epic 账号正在同步，请稍后再试")?;Ok(Self)}}
impl Drop for Guard{fn drop(&mut self){SYNCING.store(false,Ordering::Release);}}
fn root(app:&AppHandle)->Result<PathBuf,String>{app.path().app_data_dir().map_err(|e|e.to_string())}
fn load(root:&Path)->Result<Option<Session>,String>{
 let path=root.join("credentials/epic-account.bin");if !path.is_file(){return Ok(None);}
 let bytes=std::fs::read(path).map_err(|_|"无法读取 Epic 连接信息")?;
 if bytes.len()>128_000{return Err("Epic 连接信息异常，请重新连接".into());}
 let plain=crate::alpha_credentials::crypt(&bytes,false).map_err(|_|"无法解密 Epic 连接信息，请重新连接账号")?;
 serde_json::from_slice(&plain).map(Some).map_err(|_|"Epic 连接信息异常，请重新连接".into())
}
fn save(root:&Path,session:&Session)->Result<(),String>{
 let dir=root.join("credentials");std::fs::create_dir_all(&dir).map_err(|_|"无法保存 Epic 连接信息")?;
 let bytes=serde_json::to_vec(session).map_err(|_|"Epic 连接信息格式异常")?;let encrypted=crate::alpha_credentials::crypt(&bytes,true)?;
 let pending=dir.join("epic-account.pending");std::fs::write(&pending,encrypted).map_err(|_|"无法保存 Epic 连接信息")?;std::fs::rename(pending,dir.join("epic-account.bin")).map_err(|_|"无法更新 Epic 连接信息".into())
}
async fn perform(app:&AppHandle,storage:&game_storage::Storage,mut session:Session)->Result<Value,String>{
 let data=root(app)?;
 let expires=chrono::DateTime::parse_from_rfc3339(&session.expires_at).map(|d|d.timestamp()).unwrap_or(0);
 if expires<chrono::Utc::now().timestamp()+120{session=epic::refresh(&session).await?;save(&data,&session)?;}
 let snapshot=epic::library(&session).await?;let count=snapshot.games.len();let unresolved=snapshot.unresolved;
 let inserted=storage.import_epic_library(&session.account_id,&snapshot).await?;
 let _=app.emit("game-library-changed",());
 let(app,storage,games)=(app.clone(),storage.clone(),snapshot.games);
 tauri::async_runtime::spawn(async move{
  if let Ok(scraper)=game_metadata::Scraper::new(None){let _=storage.cache_epic_covers(scraper,games,&data.join("artwork")).await;let _=app.emit("game-library-changed",());}
  crate::epic_artwork::enqueue(app,storage,false);
 });
 Ok(json!({"count":count,"inserted":inserted,"unresolved":unresolved,"skipped":snapshot.skipped}))
}
#[tauri::command]
pub async fn epic_account_status(app:AppHandle,state:State<'_,AppState>)->Result<Value,String>{
 let session=load(&root(&app)?)?;let mut result=state.storage.epic_library_status().await?;
 result["connected"]=json!(session.is_some());result["display_name"]=json!(session.map(|s|s.display_name).unwrap_or_default());result["syncing"]=json!(SYNCING.load(Ordering::Acquire));Ok(result)
}
#[tauri::command]
pub fn open_epic_login()->Result<(),String>{game_launcher::open_uri(&epic::login_url())}
#[tauri::command]
pub async fn connect_epic_account(app:AppHandle,state:State<'_,AppState>,code:String)->Result<Value,String>{
 let _guard=Guard::acquire()?;let session=epic::connect(&code).await?;save(&root(&app)?,&session)?;state.storage.activate_epic_account(&session.account_id).await?;perform(&app,&state.storage,session).await
}
#[tauri::command]
pub async fn sync_epic_library(app:AppHandle,state:State<'_,AppState>)->Result<Value,String>{
 let _guard=Guard::acquire()?;let session=load(&root(&app)?)?.ok_or("请先连接 Epic 账号，再同步全部已购游戏")?;perform(&app,&state.storage,session).await
}
#[tauri::command]
pub async fn disconnect_epic_account(app:AppHandle,state:State<'_,AppState>)->Result<(),String>{
 let _guard=Guard::acquire()?;let path=root(&app)?.join("credentials/epic-account.bin");if path.is_file(){std::fs::remove_file(path).map_err(|_|"无法断开 Epic 账号")?;}
 state.storage.disconnect_epic_library().await?;let _=app.emit("game-library-changed",());Ok(())
}
#[tauri::command]
pub async fn install_epic_game(state:State<'_,AppState>,game_id:String)->Result<(),String>{
 let identity=state.storage.epic_install_identity(&game_id).await?;let uri=game_launcher::epic_uri(&identity)?.replace("action=launch","action=installer");game_launcher::open_uri(&uri)
}
pub async fn sync_if_due(app:&AppHandle,storage:&game_storage::Storage)->Result<(),String>{
 let Some(session)=load(&root(app)?)?else{return Ok(())};
 let status=storage.epic_library_status().await?;
 if status["synced_at"].as_str().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok()).is_some_and(|d|chrono::Utc::now().timestamp()-d.timestamp()<6*3600){return Ok(());}
 let Ok(_guard)=Guard::acquire()else{return Ok(())};perform(app,storage,session).await?;Ok(())
}

#[cfg(test)]mod tests{use super::*;
 #[test]fn epic_connection_is_encrypted_and_never_shared_with_scraper_credentials(){
  let root=std::env::temp_dir().join(format!("gamedeck-epic-auth-{}",uuid::Uuid::new_v4()));
  let session=Session{account_id:"test-account".into(),display_name:"Test".into(),access_token:"test-access-secret".into(),refresh_token:"test-refresh-secret".into(),expires_at:"2027-01-01T00:00:00Z".into()};
  save(&root,&session).unwrap();let bytes=std::fs::read(root.join("credentials/epic-account.bin")).unwrap();assert!(!bytes.windows(session.refresh_token.len()).any(|w|w==session.refresh_token.as_bytes()));assert_eq!(load(&root).unwrap().unwrap().refresh_token,session.refresh_token);assert!(!root.join("credentials/steamgriddb.key").exists());
  std::fs::write(root.join("credentials/epic-account.bin"),b"bad").unwrap();assert!(load(&root).is_err());std::fs::remove_dir_all(root).unwrap();
 }
}

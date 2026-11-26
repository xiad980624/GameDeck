//! Artwork scheduling only. This module never opens an Epic account session.
use std::sync::atomic::{AtomicBool,Ordering};
use tauri::{AppHandle,Manager,Emitter,State};
use serde_json::{Value,json};
use crate::AppState;

static RUNNING:AtomicBool=AtomicBool::new(false);
static LAST_ERROR:std::sync::Mutex<Option<String>>=std::sync::Mutex::new(None);
struct Running;
impl Drop for Running{fn drop(&mut self){RUNNING.store(false,Ordering::Release);}}
pub fn enqueue(app:AppHandle,storage:game_storage::Storage,retry:bool)->bool{
    if RUNNING.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){return false;}
    if let Ok(mut error)=LAST_ERROR.lock(){*error=None;}
    tauri::async_runtime::spawn(async move{
        let _running=Running;
        let result=async{
            let data=app.path().app_data_dir().map_err(|e|e.to_string())?;let root=data.join("artwork");
            let scraper=game_metadata::Scraper::new(crate::alpha_credentials::load_sgdb(&data)?)?.with_alpha_key(crate::alpha_credentials::load(&data)?);
            let handle=app.clone();
            storage.repair_epic_artwork(scraper,&root,retry,move|progress|{
                let _=handle.emit("game-library-changed",());
                if progress["processed"].is_number(){let _=handle.emit("epic-artwork-progress",progress);}
            }).await
        }.await;
        if let Err(error)=&result{if let Ok(mut saved)=LAST_ERROR.lock(){*saved=Some(error.clone());}}
        drop(_running);
        let event=match result{Ok(report)=>json!({"running":false,"result":report}),Err(error)=>json!({"running":false,"error":error})};
        let _=app.emit("game-library-changed",());let _=app.emit("epic-artwork-progress",event);
    });true
}
#[tauri::command]
pub fn repair_epic_artwork(app:AppHandle,state:State<'_,AppState>,retry:Option<bool>)->bool{enqueue(app,state.storage.clone(),retry.unwrap_or(false))}
#[tauri::command]
pub fn epic_artwork_status(app:AppHandle)->Result<Value,String>{
    let path=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork/reports/epic-artwork-latest.json");
    let report=std::fs::read(path).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok());
    Ok(json!({"running":RUNNING.load(Ordering::Acquire),"report":report,"error":LAST_ERROR.lock().ok().and_then(|v|v.clone())}))
}

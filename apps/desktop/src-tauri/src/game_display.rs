use std::path::PathBuf;
use std::time::{Duration,Instant};
use game_launcher::display::{DisplayFollower,Rect};
use tauri::{AppHandle,Manager};

pub fn launch_monitor(app:&AppHandle,enabled:bool)->Option<Rect>{
    if !enabled{return None;}
    let window=app.get_webview_window("main")?;
    if window.available_monitors().ok()?.len()<2{return None;}
    let monitor=window.current_monitor().ok()??;let position=monitor.position();let size=monitor.size();
    Some(Rect{x:position.x,y:position.y,width:size.width as i32,height:size.height as i32})
}
pub fn follow(app:&AppHandle,path:&str,target:Option<Rect>,session:&str){
    let Some(target)=target else{return;};
    let path=PathBuf::from(path);let root=if path.is_file(){path.parent().map(PathBuf::from).unwrap_or(path)}else{path};
    let log=app.path().app_data_dir().ok().map(|p|p.join("logs").join(format!("game-display-{session}.json")));
    // Window placement only. Borderless mode is applied at the game level by
    // game_profile::prepare before launch; forcing window styles here would
    // double-apply and can blur games that do not rebuild their swapchain.
    tauri::async_runtime::spawn_blocking(move||{
        let start=Instant::now();let mut follower=DisplayFollower::new(target);let mut entries=Vec::new();let mut seen=false;let mut last_seen=Instant::now();
        while start.elapsed()<Duration::from_secs(90)&&!follower.settled(){
            match game_launcher::game_processes(&root){
                Ok(pids)=>{if !pids.is_empty(){seen=true;last_seen=Instant::now();entries.extend(follower.update(&pids));}else if seen&&last_seen.elapsed()>Duration::from_secs(5){break;}},
                Err(error)=>{entries.push(error);break;}
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        if let Some(log)=log{let report=serde_json::json!({"monitor":{"x":target.x,"y":target.y,"width":target.width,"height":target.height},"entries":entries,"process_detected":seen});if let Ok(data)=serde_json::to_vec_pretty(&report){let _=std::fs::write(log,data);}}
    });
}

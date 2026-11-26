mod native_gamepad;
mod lobby_focus;
mod window_shortcut;
use window_shortcut::toggle_lobby_fullscreen;
use native_gamepad::native_gamepad_status;
mod game_display;
mod game_session;
mod launch_cursor;
mod alpha_credentials;
mod epic_account;
mod epic_artwork;
use epic_artwork::{repair_epic_artwork,epic_artwork_status};
use epic_account::{epic_account_status,open_epic_login,connect_epic_account,sync_epic_library,disconnect_epic_account,install_epic_game};
mod startup;
mod display_agent;
use display_agent::{display_agent_config,display_agent_pause,display_agent_status,save_display_agent_config,set_display_agent_config};
mod agent_host;
use agent_host::{autostart_status,install_autostart,uninstall_autostart};
mod game_profile;
mod manual_artwork;
use manual_artwork::{open_artwork_website,artwork_choices,prepare_artwork,discard_artwork,commit_artwork};
mod accelerators;
use accelerators::{list_accelerators,set_accelerator_path,open_accelerator};
use chrono::Utc;
use game_core::{Game, ScanReport, SessionInfo};
use game_launcher::LaunchResult;
use game_storage::Storage;
use std::{path::Path, fs};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

struct AppState {
    storage: Storage,
}

#[tauri::command]
async fn repair_titles(app:AppHandle,state:State<'_,AppState>)->Result<Vec<Game>,String>{
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 let stamp=root.join("title-repair-v1.json");
 let due=std::fs::metadata(&stamp).and_then(|m|m.modified()).ok().and_then(|t|t.elapsed().ok()).is_none_or(|age|age.as_secs()>6*3600);
 if due{let scraper=game_metadata::Scraper::new(sgdb_key(&app,None)?)?;let report=state.storage.repair_titles(scraper,&root).await?;std::fs::write(stamp,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;}
 state.storage.list_games().await
}

#[tauri::command]
fn alpha_key_status(app:AppHandle)->Result<bool,String>{alpha_credentials::load(&app.path().app_data_dir().map_err(|e|e.to_string())?).map(|k|k.is_some())}
#[tauri::command]
fn set_alpha_key(app:AppHandle,key:String)->Result<(),String>{alpha_credentials::save(&app.path().app_data_dir().map_err(|e|e.to_string())?,&key)}

#[tauri::command]
fn sgdb_key_status(app:AppHandle)->Result<bool,String>{alpha_credentials::load_sgdb(&app.path().app_data_dir().map_err(|e|e.to_string())?).map(|k|k.is_some())}
#[tauri::command]
fn set_sgdb_key(app:AppHandle,key:String)->Result<(),String>{alpha_credentials::save_sgdb(&app.path().app_data_dir().map_err(|e|e.to_string())?,&key)}
fn sgdb_key(app:&AppHandle,key:Option<String>)->Result<Option<String>,String>{
 if let Some(key)=key.filter(|k|!k.trim().is_empty()){return Ok(Some(key));}
 alpha_credentials::load_sgdb(&app.path().app_data_dir().map_err(|e|e.to_string())?)
}
#[tauri::command]
async fn hero_display_path(app:AppHandle,state:State<'_,AppState>,game_id:String)->Result<Option<String>,String>{
 let Some(path)=state.storage.hero_path(&game_id).await? else{return Ok(None)};
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 tauri::async_runtime::spawn_blocking(move||game_metadata::display::background(Path::new(&path),&root).map(|p|Some(p.to_string_lossy().into_owned()))).await.map_err(|_|"背景预加载失败".to_string())?
}

#[tauri::command]
async fn background_candidates(app:AppHandle,state:State<'_,AppState>,game_id:String,refresh:bool)->Result<Vec<game_metadata::background::BackgroundChoice>,String>{
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 let game=state.storage.games_for_scraping().await?.into_iter().find(|g|g.id==game_id).ok_or("未找到游戏")?;
 if refresh {let scraper=game_metadata::Scraper::new(sgdb_key(&app,None)?)?.with_alpha_key(alpha_credentials::load(root.parent().ok_or("数据目录无效")?)?);
 scraper.scrape_background(&game,&root,state.storage.confirmed_sgdb_id(&game_id).await?,state.storage.confirmed_steam_artwork(&game_id).await?,true).await?;}
 Ok(game_metadata::background::choices(&root,&game_id))
}
#[tauri::command]
async fn choose_background(app:AppHandle,state:State<'_,AppState>,game_id:String,candidate_id:String)->Result<(),String>{
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 let choice=game_metadata::background::choices(&root,&game_id).into_iter().find(|c|c.id==candidate_id).ok_or("候选不存在，请重新搜索")?;
 state.storage.save_manual_asset(&game_id,"hero",&choice.path,choice.width,choice.height).await
}

#[tauri::command]
async fn card_motion_candidates(app:AppHandle,state:State<'_,AppState>,game_id:String,kind:String,refresh:bool)->Result<Vec<game_metadata::motion_review::Choice>,String>{
 let kind=match kind.as_str(){"poster"=>"poster","landscape"=>"landscape",_=>return Err("动图仅用于竖封面和横卡片".into())};
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 let game=state.storage.games_for_scraping().await?.into_iter().find(|g|g.id==game_id).ok_or("未找到游戏")?;
 if refresh{let scraper=game_metadata::Scraper::new(sgdb_key(&app,None)?)?;let (_,issues)=scraper.collect_motion_cards_for(&game,&root,state.storage.confirmed_sgdb_id(&game_id).await?,&[kind]).await;
 let items=game_metadata::motion_review::choices(&root,&game_id,kind)?;if items.is_empty()&&!issues.is_empty(){return Err(issues.join("；"));}return Ok(items);}
 game_metadata::motion_review::choices(&root,&game_id,kind)
}
#[tauri::command]
async fn choose_motion_card(app:AppHandle,state:State<'_,AppState>,game_id:String,kind:String,candidate_id:String)->Result<(),String>{
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 let choice=game_metadata::motion_review::choices(&root,&game_id,&kind)?.into_iter().find(|c|c.id==candidate_id).ok_or("候选不存在")?;
 let review=game_metadata::motion_review::entry(&root,&game_id,&kind,&choice.path)?;
 state.storage.save_manual_asset(&game_id,&kind,&choice.path,choice.width,choice.height).await?;game_metadata::motion_review::set_review(&root,review,true)
}

async fn refresh_chinese_titles(storage: &Storage) -> Result<(),String> {
    let games = storage.games_for_scraping().await?;
    let titles = tauri::async_runtime::spawn_blocking(move || {
        let catalogue = game_discovery::appinfo::catalogue();
        games.into_iter().filter_map(|game| game_discovery::titles::chinese_title(&game.provider,&game.external_id,&game.name,&catalogue)
            .map(|title| (game.id,title))).collect::<Vec<_>>()
    }).await.map_err(|e| e.to_string())?;
    storage.save_chinese_titles(&titles).await
}

#[tauri::command]
async fn startup_library(app:AppHandle,state:State<'_,AppState>)->Result<serde_json::Value,String>{
 let games=state.storage.list_games().await?;
 let sources:Vec<_>=games.iter().filter(|g|!g.hidden).filter_map(|g|g.hero.clone()).collect();
 let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
 let displays=tauri::async_runtime::spawn_blocking(move||{let mut map=std::collections::BTreeMap::new();for source in sources{if let Ok(Some(path))=game_metadata::display::cached_background(Path::new(&source),&root){map.insert(source,path.to_string_lossy().into_owned());}}map}).await.map_err(|e|e.to_string())?;
 app.state::<startup::Trace>().mark("library-read",None);
 Ok(serde_json::json!({"games":games,"hero_displays":displays,"probe":app.state::<startup::Trace>().probe,"navigation_probe":app.state::<startup::Trace>().probe&&std::env::var("GAMEDECK_NAVIGATION_PROBE").as_deref()==Ok("1")}))
}
#[tauri::command]
fn startup_mark(app:AppHandle,stage:String,frontend_ms:f64){
 if !["shell","library","hero"].contains(&stage.as_str()){return;}
 let trace=app.state::<startup::Trace>();trace.mark(&stage,Some(frontend_ms));
 // display_agent reveals the lobby at startup; no competing fallback here.
 if stage=="hero"&&trace.probe&&std::env::var("GAMEDECK_NAVIGATION_PROBE").as_deref()!=Ok("1"){app.exit(0);}
}
async fn tokio_wait_startup(app:AppHandle){
 // No hidden window is left stranded if the renderer fails before its first commit.
 tauri::async_runtime::spawn_blocking(move||{std::thread::sleep(std::time::Duration::from_secs(8));let trace=app.state::<startup::Trace>();if trace.probe{std::thread::sleep(std::time::Duration::from_secs(12));if !trace.has("hero"){trace.mark("probe-timeout",None);app.exit(1);}}}).await.ok();
}
#[tauri::command]
fn startup_probe_result(app:AppHandle,passed:bool,checks:Vec<String>,error:Option<String>,diagnostics:Option<serde_json::Value>)->Result<(),String>{
 if !app.state::<startup::Trace>().probe{return Err("Startup probe is not enabled".into());}
 let report=serde_json::json!({"passed":passed,"checks":checks,"error":error,"diagnostics":diagnostics});
 let path=app.path().app_data_dir().map_err(|e|e.to_string())?.join("logs/navigation-probe.json");
 fs::write(path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 app.exit(if passed{0}else{1});Ok(())
}
#[tauri::command]
async fn refresh_game_status(state:State<'_,AppState>,game_id:String)->Result<Option<Game>,String>{
 if let Some(target)=state.storage.installation_for_status(&game_id).await?{
  let statuses=tauri::async_runtime::spawn_blocking(move||game_discovery::status::collect(&[target])).await.map_err(|e|e.to_string())?;
  for status in statuses{state.storage.save_installation_status(&status.provider,&status.external_id,&status.status,status.version.as_deref(),&status.detail).await?;}
 }
 Ok(state.storage.list_games().await?.into_iter().find(|game|game.id==game_id))
}
#[tauri::command]
async fn list_games(state: State<'_, AppState>) -> Result<Vec<Game>, String> {
    state.storage.list_games().await
}
#[tauri::command]
async fn improve_background(app:AppHandle,state:State<'_,AppState>,game_id:String,api_key:Option<String>)->Result<Vec<Game>,String>{
    if state.storage.custom_hero(&game_id).await?{return Err("此游戏正在使用自定义背景，请先在素材设置中恢复自动图片".into());}
    let game=state.storage.games_for_scraping().await?.into_iter().find(|g|g.id==game_id).ok_or("未找到游戏")?;
    let root=app.path().app_data_dir().map_err(|e|e.to_string())?.join("artwork");
    let scraper=game_metadata::Scraper::new(sgdb_key(&app,api_key)?)?.with_alpha_key(alpha_credentials::load(root.parent().ok_or("数据目录无效")?)?);
    let result=state.storage.refresh_promo_game(&scraper,&game,&root).await?;
    if result["status"]=="no-verified-resource" { return Err(result["reason"].as_str().unwrap_or("无合格宣传背景").into()); }
    state.storage.list_games().await
}
#[tauri::command]
async fn refresh_promo_backgrounds(app:AppHandle,state:State<'_,AppState>)->Result<serde_json::Value,String>{
    let data=app.path().app_data_dir().map_err(|e|e.to_string())?;
    let scraper=game_metadata::Scraper::new(sgdb_key(&app,None)?)?.with_alpha_key(alpha_credentials::load(&data)?);
    state.storage.refresh_promo_backgrounds(scraper,&data.join("artwork"),move |progress| {
        let _=app.emit("artwork-scrape-progress",progress);
    }).await
}
#[tauri::command]
async fn open_platform(provider:String,section:String)->Result<(),String>{
    let client=if provider=="battlenet" {tauri::async_runtime::spawn_blocking(game_discovery::battlenet::launcher_path).await.map_err(|e|e.to_string())?}else{None};
    game_launcher::open_platform(&provider,&section,client.as_deref())
}

#[derive(serde::Serialize)]
struct PlaytimeSyncReport { updated: usize }

async fn sync_steam_times(storage: &Storage) -> Result<PlaytimeSyncReport, String> {
    let selected = storage.active_steam_account().await?;
    let snapshot = tauri::async_runtime::spawn_blocking(move || game_discovery::playtime::read_steam_playtime_for(selected.as_deref()))
        .await.map_err(|e| e.to_string())??;
    let games: Vec<_> = snapshot.games.into_iter().map(|g| (g.id,g.name)).collect();
    storage.import_steam_catalogue(&snapshot.account_id,&games,"cache").await?;
    let records: Vec<_> = snapshot.records.into_iter().map(|r| (r.app_id,r.minutes,r.last_played)).collect();
    if records.is_empty() { return Err("Steam 当前账号尚无可同步的时长记录".into()); }
    let updated = storage.sync_steam_playtime(&snapshot.account_id,&records).await?;
    Ok(PlaytimeSyncReport { updated })
}

#[tauri::command]
async fn steam_accounts(state: State<'_,AppState>) -> Result<serde_json::Value,String> {
    let accounts = tauri::async_runtime::spawn_blocking(game_discovery::playtime::steam_accounts).await.map_err(|e| e.to_string())?;
    let selected = state.storage.active_steam_account().await?;
    let scope = state.storage.steam_library_scope(selected.as_deref().unwrap_or("")).await?;
    Ok(serde_json::json!({"accounts":accounts,"selected":selected,"scope":scope}))
}
#[tauri::command]
async fn select_steam_account(state: State<'_,AppState>, account_id: String) -> Result<PlaytimeSyncReport,String> {
    let selected = account_id.clone();
    // Validate/read before committing the selection, preserving a valid existing account on failure.
    let snapshot = tauri::async_runtime::spawn_blocking(move || game_discovery::playtime::read_steam_playtime_for(Some(&selected))).await.map_err(|e| e.to_string())??;
    let games: Vec<_> = snapshot.games.into_iter().map(|g| (g.id,g.name)).collect();
    state.storage.import_steam_catalogue(&account_id,&games,"cache").await?;
    let records: Vec<_> = snapshot.records.into_iter().map(|r| (r.app_id,r.minutes,r.last_played)).collect();
    let updated = state.storage.sync_steam_playtime(&account_id,&records).await?;
    state.storage.select_steam_account(&account_id).await?;
    refresh_chinese_titles(&state.storage).await?;
    Ok(PlaytimeSyncReport { updated })
}
#[tauri::command]
async fn sync_steam_library(state: State<'_,AppState>, api_key: String) -> Result<usize,String> {
    if api_key.trim().is_empty() { return Err("请填写 Steam Web API Key".into()); }
    let account = state.storage.active_steam_account().await?.ok_or("请先选择 Steam 账号")?;
    let games = game_metadata::Scraper::new(None)?.owned_steam_games(api_key.trim(),&account).await?;
    let catalogue: Vec<_> = games.iter().map(|g| (g.appid.to_string(),g.name.clone())).collect();
    state.storage.import_steam_catalogue(&account,&catalogue,"owned").await?;
    let records: Vec<_> = games.iter().map(|g| (g.appid.to_string(),g.playtime_forever,g.rtime_last_played)).collect();
    state.storage.sync_steam_playtime(&account,&records).await?;
    refresh_chinese_titles(&state.storage).await?;
    Ok(games.len())
}
#[tauri::command]
async fn install_steam_game(state: State<'_,AppState>, game_id: String) -> Result<(),String> {
    let game = state.storage.list_games().await?.into_iter().find(|g| g.id == game_id && g.platform == "steam").ok_or("未找到 Steam 游戏")?;
    game_launcher::install_steam(&game.external_id)
}

#[tauri::command]
async fn sync_playtime(state: State<'_, AppState>) -> Result<PlaytimeSyncReport, String> {
    let report = sync_steam_times(&state.storage).await?;
    refresh_chinese_titles(&state.storage).await?;
    Ok(report)
}

#[tauri::command]
async fn scan_games(app: AppHandle, state: State<'_, AppState>, background:Option<bool>) -> Result<ScanReport, String> {
    let incremental=background.unwrap_or(false);
    let progress=|label:&str|{if incremental{let _=app.emit("library-scan-progress",label);}};
    progress("后台扫描：正在检查游戏平台…");
    app.state::<startup::Trace>().mark(if incremental{"background-scan"}else{"manual-scan"},None);
    let previous=state.storage.list_games().await?;
    let known:std::collections::HashSet<_>=previous.iter().map(|g|g.id.clone()).collect();
    let results = tauri::async_runtime::spawn_blocking(game_discovery::scan_all)
        .await.map_err(|e| format!("游戏平台扫描任务异常：{e}"))?;
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let mut battle_sources=None;
    for result in results {
        if result.provider=="battlenet"&&result.errors.is_empty(){battle_sources=Some(result.games.iter().map(|g|g.external_id.clone()).collect::<Vec<_>>());}
        errors.extend(result.errors.into_iter().map(|e| format!("[{}] {e}", result.provider)));
        found.extend(result.games);
    }
    let mut caches = game_discovery::steam::artwork_cache_roots();
    for entry in &found {
        if entry.provider != "steam" { continue; }
        if let Some(steamapps) = Path::new(&entry.install_path).ancestors().find(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("steamapps"))) {
            if let Some(root) = steamapps.parent() {
                let cache = root.join("appcache").join("librarycache");
                if cache.is_dir() && !caches.contains(&cache) { caches.push(cache); }
            }
        }
    }
    progress(&format!("后台扫描：已发现 {} 款，正在同步入库…",found.len()));
    let mut report = state.storage.upsert_discovered(found, errors).await?;
    if let Err(error)=epic_account::sync_if_due(&app,&state.storage).await{report.errors.push(format!("[Epic 账号库] {error}"));}
    // Read the actual installed paths just discovered; stale Battle.net registry entries
    // are not retained when its authoritative scan succeeds.
    if let Some(sources)=battle_sources {state.storage.reconcile_battlenet(&sources).await?;}
    progress("后台扫描：正在检查全库安装和更新状态…");
    let targets=state.storage.installed_games_for_status().await?;
    let statuses=tauri::async_runtime::spawn_blocking(move||game_discovery::status::collect(&targets)).await.map_err(|e|e.to_string())?;
    for status in statuses {state.storage.save_installation_status(&status.provider,&status.external_id,&status.status,status.version.as_deref(),&status.detail).await?;}
    progress("后台扫描：正在同步游玩时长…");
    if let Err(error) = sync_steam_times(&state.storage).await { report.errors.push(error); }
    let current=state.storage.games_for_scraping().await?;
    let has_new=current.iter().any(|g|!known.contains(&g.id));
    if !incremental||has_new {progress("后台扫描：正在同步游戏名称…");if let Err(error)=refresh_chinese_titles(&state.storage).await {report.errors.push(error);}}
    let games:Vec<_>=current.into_iter().filter(|g|!incremental||!known.contains(&g.id)).collect();
    // Cached artwork is durable. Background discovery imports only newly added entries.
    if games.is_empty(){return Ok(report);}
    progress(&format!("后台扫描：正在导入 {} 款游戏的本地素材…",games.len()));
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?.join("artwork");
    let imported = tauri::async_runtime::spawn_blocking(move || {
        let epic_caches = game_metadata::epic_cache::cache_roots();
        let catalogue = game_discovery::appinfo::catalogue();
        games.into_iter().map(|game| {
            let assets = if game.provider == "epic" {
                game_discovery::epic::identity(&game.external_id).map(|identity| game_metadata::epic_cache::import(&game,&identity.namespace,&epic_caches,&root)).unwrap_or_else(|| Ok(Vec::new()))
            } else if game.provider == "steam" { game_metadata::import_steam_cache(&game,&caches,&root) }
            else if game.provider=="battlenet" {game_metadata::battlenet_cache::import(&game,&root)}
            else { Ok(Vec::new()) };
            let mut assets = assets;
            if game.provider != "steam" {
                if let Some(app) = catalogue.values().find(|app| game_metadata::normalize(&app.name) == game_metadata::normalize(&game.name) && app.kind == "game") {
                    let matched = game_metadata::GameRef { id:game.id.clone(),name:game.name.clone(),provider:"steam".into(),external_id:app.id.clone() };
                    if let Ok(extra) = game_metadata::import_steam_cache(&matched,&caches,&root) { if let Ok(ref mut assets) = assets { assets.extend(extra); } }
                }
            }
            (game.id, assets)
        }).collect::<Vec<_>>()
    }).await.map_err(|e| e.to_string())?;
    let mut local_assets = Vec::new();
    for (game_id, assets) in imported {
        match assets {
            Ok(assets) => for asset in assets {
                local_assets.push((game_id.clone(),asset));
            },
            Err(error) => report.errors.push(error),
        }
    }
    if let Err(error) = state.storage.save_local_assets(local_assets).await { report.errors.push(error); }
    Ok(report)
}

#[tauri::command]
async fn add_manual_game(state: State<'_, AppState>, path: String, name: Option<String>, hidden: Option<bool>) -> Result<String, String> {
    state.storage.import_manual(Path::new(&path), name.as_deref(), hidden.unwrap_or(false)).await
}

#[tauri::command]
async fn scan_manual_folder(path:String)->Result<game_core::ManualFolderScan,String>{
    tauri::async_runtime::spawn_blocking(move||game_storage::manual::scan_folder(Path::new(&path))).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn set_hidden(state:State<'_,AppState>,game_id:String,hidden:bool)->Result<(),String>{
    state.storage.set_hidden(&game_id,hidden).await
}

#[tauri::command]
async fn set_favorite(state: State<'_, AppState>, game_id: String, favorite: bool) -> Result<(), String> {
    state.storage.set_favorite(&game_id, favorite).await
}

/// Track verified game processes and windows for both native and client launches.
#[tauri::command]
async fn launch_game(
    app: AppHandle,
    state: State<'_, AppState>,
    installation_id: String,
    session_id: String,
    follow_display: Option<bool>,
    via_controller: Option<bool>,
) -> Result<SessionInfo, String> {
    let session_id = Uuid::parse_str(&session_id).map_err(|_| "游戏会话 ID 无效")?.to_string();
    let follow_display = follow_display.unwrap_or(true);
    let cursor = launch_cursor::Handoff::capture(&app, via_controller.unwrap_or(false) && follow_display);
    let mut target = state.storage.launch_target(&installation_id).await?;
    // Rewrite the game's own display config to borderless windowed *before*
    // launch (Steam URIs cannot carry flags). Only explicitly profiled UE
    // titles are touched; never let a profile failure block the launch.
    if display_agent::force_borderless() {
        for outcome in game_profile::prepare(&target.external_id, &target.game_id) {
            eprintln!("[game-profile] {} | {} | {} | {}",
                if outcome.applied { "APPLIED" } else { "SKIPPED" },
                outcome.game_name, outcome.detail, outcome.config_path.unwrap_or_default());
        }
    }
    let reservation=game_launcher::reserve(&target.game_id)?;
    if target.provider == "epic" {
        let install_path=target.install_path.clone();
        let app_name = target.external_id.clone();
        let identity = tauri::async_runtime::spawn_blocking(move || game_discovery::epic::identity_at(&app_name,Path::new(&install_path))).await.map_err(|e| e.to_string())?;
        if let Some(identity) = identity { target.external_id = identity.qualified(); }
    }
    let client=if target.provider=="battlenet"{tauri::async_runtime::spawn_blocking(game_discovery::battlenet::launcher_path).await.map_err(|e|e.to_string())?}
    else if target.provider=="epic"{let path=target.install_path.clone();tauri::async_runtime::spawn_blocking(move||game_discovery::epic::launcher_path(Path::new(&path))).await.map_err(|e|e.to_string())?}else{None};
    let monitor=game_display::launch_monitor(&app,follow_display);
    let parked = cursor.and_then(launch_cursor::Handoff::park);
    let launched = match game_launcher::launch_with_client(&target,client.as_deref()) {
        Ok(launched) => launched,
        Err(error) => {
            if let Some(cursor) = parked { cursor.restore(); }
            return Err(error);
        }
    };
    game_display::follow(&app,&target.install_path,monitor,&session_id);
    let started = Utc::now();
    let mut event = SessionInfo {
        session_id: session_id.clone(),
        game_id: target.game_id.clone(),
        state: "launcher_started".into(),
        timestamp: started.to_rfc3339(),
        message: None,
    };
    match launched {
        LaunchResult::Delegated => {
            event.state="awaiting_game".into();
            let storage=state.storage.clone();let handle=app.clone();let session=event.clone();
            tauri::async_runtime::spawn_blocking(move||{
                let root=Path::new(&target.install_path);let deadline=std::time::Instant::now()+std::time::Duration::from_secs(90);
                 let mut running=false;
                 let mut recording=false;
                 let mut started_at=Utc::now();
                 let mut monitor=game_session::GameSession::default();
                 let mut last_scan=std::time::Instant::now();
                 let mut processes=Vec::new();
                loop{
                     if monitor.needs_process_scan(last_scan) {
                         last_scan=std::time::Instant::now();
                         match game_launcher::game_processes(root){
                             Ok(found)=>processes=found,
                             Err(error)=>{
                                 lobby_focus::restore(&handle,&target.provider,monitor.took_foreground());
                                 drop(reservation);
                                 let _=handle.emit("game-session-changed",SessionInfo{state:"failed".into(),timestamp:Utc::now().to_rfc3339(),message:Some(format!("无法检查游戏进程：{error}")),..session.clone()});
                                 return;
                             }
                         }
                     }
                     if !running && !processes.is_empty(){
                         running=true;
                         started_at=Utc::now();
                         let result=tauri::async_runtime::block_on(storage.start_session(&session.session_id,&target.game_id,&installation_id,&started_at.to_rfc3339()));
                         recording=result.is_ok();
                         let _=handle.emit("game-session-changed",SessionInfo{state:"running".into(),timestamp:started_at.to_rfc3339(),message:result.err().map(|e|format!("游戏已运行，但记录失败：{e}")),..session.clone()});
                     }
                     if running && monitor.poll(&processes,!processes.is_empty()){
                        let elapsed=(Utc::now()-started_at).num_seconds().max(0);
                         lobby_focus::restore(&handle,&target.provider,monitor.took_foreground());
                         let result=if recording { tauri::async_runtime::block_on(storage.finish_session(&session.session_id,&target.game_id,elapsed,"normal_exit")) } else { Ok(()) };
                         drop(reservation);
                        let _=handle.emit("game-session-changed",SessionInfo{state:"exited".into(),timestamp:Utc::now().to_rfc3339(),message:result.err().map(|e|format!("保存记录失败：{e}")),..session.clone()});return;
                    }else if !running&&std::time::Instant::now()>=deadline{
                        drop(reservation);
                        let _=handle.emit("game-session-changed",SessionInfo{state:"failed".into(),timestamp:Utc::now().to_rfc3339(),message:Some(format!("{} 在 90 秒内未启动。请检查官方客户端登录状态、更新或授权提示后重试。",target.provider)),..session.clone()});return;
                    }
                     std::thread::sleep(game_session::POLL_INTERVAL);
                }
            });
        }
        LaunchResult::Direct(mut child) => {
            let recording = state.storage.start_session(&session_id, &target.game_id, &installation_id, &event.timestamp).await;
            event.state = "running".into();
            event.message = recording.as_ref().err().map(|e|format!("游戏已运行，但记录失败：{e}"));
            let storage = state.storage.clone();
            let app_handle = app.clone();
            let game_id = target.game_id;
            tauri::async_runtime::spawn_blocking(move || {
                let launched_pid = child.id();
                let path = Path::new(&target.install_path);
                let root = path.parent().unwrap_or(path);
                let mut known = std::collections::HashSet::new();
                let mut monitor = game_session::GameSession::default();
                let status = loop {
                    let status = child.try_wait();
                    let mut processes = game_launcher::child_game_processes(root, launched_pid, &mut known).unwrap_or_default();
                    let alive = matches!(status, Ok(None));
                    if alive && !processes.contains(&launched_pid) { processes.push(launched_pid); }
                    if monitor.poll(&processes, alive || !processes.is_empty()) { break status; }
                    std::thread::sleep(game_session::POLL_INTERVAL);
                };
                lobby_focus::restore(&app_handle,"manual",monitor.took_foreground());
                let elapsed = (Utc::now() - started).num_seconds().max(0);
                let reason = match status {
                    Ok(Some(status)) if !status.success() => "nonzero_exit",
                    Ok(_) => "normal_exit",
                    Err(_) => "monitor_error",
                };
                let saved = recording.and_then(|()|tauri::async_runtime::block_on(storage.finish_session(&session_id, &game_id, elapsed, reason)));
                let message = match saved {
                    Ok(()) => "游戏已退出，游玩记录已保存".into(),
                    Err(err) => format!("游戏已退出，但保存游玩记录失败：{err}"),
                };
                drop(reservation);
                let _ = app_handle.emit("game-session-changed", SessionInfo {
                    session_id, game_id, state: "exited".into(),
                    timestamp: Utc::now().to_rfc3339(), message: Some(message),
                });
            });
        }
    }
    let _ = app.emit("game-session-changed", event.clone());
    Ok(event)
}

#[derive(serde::Serialize)]
struct ScrapeReport { processed: usize, saved: usize, errors: Vec<String> }

#[tauri::command]
async fn scrape_artwork(app: AppHandle, state: State<'_, AppState>, api_key: Option<String>, force: bool, igdb_client_id: Option<String>, igdb_token: Option<String>) -> Result<ScrapeReport, String> {
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?.join("artwork");
    let scraper = game_metadata::Scraper::new(sgdb_key(&app,api_key)?)?.with_alpha_key(alpha_credentials::load(root.parent().ok_or("数据目录无效")?)?).with_igdb(igdb_client_id, igdb_token);
    let games = state.storage.public_games_for_scraping().await?;
    let total = games.len();
    let mut report = ScrapeReport { processed: 0, saved: 0, errors: Vec::new() };
    for game in games {
        report.processed += 1;
        let (assets, errors) = scraper.scrape(&game, &root, force, state.storage.confirmed_sgdb_id(&game.id).await?, state.storage.confirmed_steam_artwork(&game.id).await?).await;
        report.errors.extend(errors);
        for asset in assets {
            match state.storage.save_scraped_asset(&game.id, &asset).await {
                Ok(true) => report.saved += 1,
                Ok(false) => (),
                Err(e) => report.errors.push(format!("{}: {e}", game.name)),
            }
        }
        match scraper.igdb_metadata(&game.name).await {
            Ok(Some(metadata)) => {
                if let Err(e) = state.storage.save_metadata(&game.id, &metadata).await { report.errors.push(format!("{} IGDB: {e}", game.name)); }
                let (screenshots, issues) = scraper.cache_screenshots(&game, &metadata.screenshots, &root).await;
                report.errors.extend(issues);
                for asset in screenshots {
                    match state.storage.save_scraped_asset(&game.id, &asset).await {
                        Ok(true) => report.saved += 1, Ok(false) => (), Err(e) => report.errors.push(e),
                    }
                }
            }
            Ok(None) => (),
            Err(e) => report.errors.push(format!("{} IGDB: {e}", game.name)),
        }
        let _ = app.emit("artwork-scrape-progress", serde_json::json!({
            "processed": report.processed, "total": total, "saved": report.saved, "game_id": game.id
        }));
    }
    Ok(report)
}

#[tauri::command]
async fn list_game_screenshots(state: State<'_, AppState>, game_id: String) -> Result<Vec<String>, String> {
    state.storage.screenshots(&game_id).await
}
#[tauri::command]
async fn search_artwork_candidates(app:AppHandle,name: String, api_key: String, provider: Option<String>) -> Result<Vec<game_metadata::Candidate>, String> {
    let scraper = game_metadata::Scraper::new(sgdb_key(&app,Some(api_key))?)?;
    if provider.as_deref() == Some("steamgriddb") { scraper.candidates(&name).await } else { scraper.steam_candidates(&name).await }
}
#[tauri::command]
async fn confirm_artwork_match(state: State<'_, AppState>, game_id: String, sgdb_id: u64, provider: Option<String>) -> Result<(), String> {
    if provider.as_deref() == Some("steamgriddb") { state.storage.confirm_sgdb(&game_id, sgdb_id).await }
    else { state.storage.confirm_steam_artwork(&game_id,sgdb_id).await }
}
#[tauri::command]
async fn set_custom_artwork(app: AppHandle, state: State<'_, AppState>, game_id: String, kind: String, path: String) -> Result<(), String> {
    if !["poster", "hero", "landscape", "logo"].contains(&kind.as_str()) { return Err("无效素材类型".into()); }
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?.join("artwork").join(&kind);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let dest = root.join(format!("{}-manual-{}.png", game_id,Uuid::new_v4()));
    let (w,h) = game_metadata::Scraper::import_image(Path::new(&path), &dest, &kind)?;
    state.storage.save_manual_asset(&game_id, &kind, &dest, w, h).await
}
#[tauri::command]
async fn reset_custom_artwork(state: State<'_, AppState>, game_id: String, kind: String) -> Result<(), String> {
    if !["poster", "hero", "landscape", "logo"].contains(&kind.as_str()) { return Err("无效素材类型".into()); }
    state.storage.reset_manual_asset(&game_id, &kind).await
}

pub fn run() {
    let started=std::time::Instant::now();
    let probe=std::env::var("GAMEDECK_STARTUP_PROBE").as_deref()==Ok("1");
    let mut context = tauri::generate_context!();
    // A marker next to the EXE enables portable storage without changing the OS profile.
    let portable_root = std::env::var_os("GAMEDECK_DATA_DIR").map(std::path::PathBuf::from)
        .or_else(|| {
            let exe = std::env::current_exe().ok()?;
            let dir = exe.parent()?;
            dir.join("GameDeck.portable").is_file().then(|| dir.join("data"))
        });
    if let Some(root) = portable_root {
        let root = if root.is_absolute() { root } else {
            std::env::current_dir().expect("Cannot resolve portable data directory").join(root)
        };
        for window in &mut context.config_mut().app.windows {
            window.data_directory = Some(root.join(if probe&&std::env::var("GAMEDECK_NAVIGATION_PROBE").as_deref()==Ok("1"){"webview-navigation-probe"}else if probe{"webview-startup-probe"}else{"webview"}));
        }
        context.config_mut().app.app_directories_override =
            Some(tauri::utils::config::AppDirectoriesOverride::Root(root));
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            // Closing the lobby hides it; the tray is the way back. If tray
            // setup failed, leave close untouched so the app can still exit.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && agent_host::tray_ready() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            let dir = app.path().app_data_dir()?;
            app.manage(startup::Trace::new(started,dir.join("logs"),probe));
            if !probe {
                display_agent::start(app.handle().clone());
                if let Err(error) = agent_host::install_tray(app.handle()) {
                    eprintln!("GameDeck tray unavailable: {error}");
                }
            }
            native_gamepad::start(app.handle());
            window_shortcut::install(app.handle());
            for name in ["database", "artwork", "logs"] {
                let path = dir.join(name);
                fs::create_dir_all(&path).map_err(|e| std::io::Error::new(e.kind(), format!("创建数据目录 {} 失败: {e}", path.display())))?;
            }
            let db = dir.join("database").join("gamedeck.db");
            let storage = tauri::async_runtime::block_on(Storage::open(&db))
                .map_err(std::io::Error::other)?;
            app.manage(AppState { storage });
            app.state::<startup::Trace>().mark("database-ready",None);
            let handle=app.handle().clone();tauri::async_runtime::spawn(async move{tokio_wait_startup(handle).await;});
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![native_gamepad_status,toggle_lobby_fullscreen,repair_epic_artwork,epic_artwork_status,epic_account_status,open_epic_login,connect_epic_account,sync_epic_library,disconnect_epic_account,install_epic_game,repair_titles,startup_library,startup_mark,startup_probe_result,refresh_game_status,card_motion_candidates,choose_motion_card,hero_display_path,background_candidates,choose_background,sgdb_key_status,set_sgdb_key,alpha_key_status,set_alpha_key,
            display_agent_status,display_agent_config,set_display_agent_config,save_display_agent_config,display_agent_pause,autostart_status,install_autostart,uninstall_autostart,
            open_artwork_website, artwork_choices, prepare_artwork, discard_artwork, commit_artwork, improve_background, refresh_promo_backgrounds, list_accelerators, set_accelerator_path, open_accelerator, list_games, open_platform, scan_games, sync_playtime, steam_accounts, select_steam_account, sync_steam_library, install_steam_game, add_manual_game, scan_manual_folder, set_hidden, set_favorite, launch_game, scrape_artwork, search_artwork_candidates, confirm_artwork_match, set_custom_artwork, reset_custom_artwork, list_game_screenshots
        ])
        .run(context)
        .expect("GameDeck failed to start");
}

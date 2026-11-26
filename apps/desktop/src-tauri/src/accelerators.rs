use std::{collections::BTreeMap,fs,path::{Path,PathBuf}};
use tauri::Manager;
#[derive(serde::Serialize)]
pub struct Accelerator { id: &'static str, name: &'static str, path: Option<PathBuf>, website: &'static str }
fn identity(id:&str)->Result<(&'static str,&'static str),String>{match id{"uu"=>Ok(("UU加速器","https://uu.163.com/")),"xunyou"=>Ok(("迅游加速器","https://www.xunyou.com/")),_=>Err("未知加速器".into())}}
fn valid_exe(path:&Path)->bool{path.is_file()&&path.extension().is_some_and(|s|s.eq_ignore_ascii_case("exe"))}
fn icon_path(value:&str)->PathBuf {
    let value=value.trim();
    if let Some(value)=value.strip_prefix('"'){return PathBuf::from(value.split('"').next().unwrap_or(""));}
    let value=value.rsplit_once(',').filter(|(_,suffix)|suffix.trim().parse::<i32>().is_ok()).map(|(path,_)|path).unwrap_or(value);
    PathBuf::from(value)
}
fn config(app:&tauri::AppHandle)->Result<PathBuf,String>{Ok(app.path().app_data_dir().map_err(|e|e.to_string())?.join("accelerators.json"))}
fn overrides(app:&tauri::AppHandle)->Result<BTreeMap<String,PathBuf>,String>{
    let path=config(app)?;if !path.exists(){return Ok(BTreeMap::new());}
    serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|format!("加速器配置读取失败：{e}"))
}
fn discover(paths:BTreeMap<String,PathBuf>)->Vec<Accelerator>{
    // UTF-8 output preserves custom Chinese installation paths. This read-only
    // helper is launched with CREATE_NO_WINDOW, just like the other discovery tools.
    let script=r#"[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); $keys=@('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'); ConvertTo-Json -Compress -InputObject @(Get-ItemProperty $keys -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName -match 'UU加速|网易UU|迅游|Xunyou'} | Select-Object DisplayName,InstallLocation,DisplayIcon)"#;
    let records=game_core::background_command("powershell.exe").args(["-NoProfile","-NonInteractive","-Command",script]).output().ok()
        .filter(|o|o.status.success()).and_then(|o|serde_json::from_slice::<Vec<serde_json::Value>>(&o.stdout).ok()).unwrap_or_default();
    ["uu","xunyou"].into_iter().map(|id|{
        let (name,website)=identity(id).unwrap();let mut candidates=Vec::new();
        if let Some(path)=paths.get(id){candidates.push(path.clone());}
        let exes=if id=="uu"{vec!["uu_launcher.exe","uu.exe"]}else{vec!["xunyou.exe"]};
        for record in &records{
            let display=record["DisplayName"].as_str().unwrap_or("").to_lowercase();
            if (id=="uu"&&display.contains("uu"))||(id=="xunyou"&&(display.contains("迅游")||display.contains("xunyou"))){
                if let Some(icon)=record["DisplayIcon"].as_str(){candidates.push(icon_path(icon));}
                if let Some(root)=record["InstallLocation"].as_str(){for exe in &exes{candidates.push(PathBuf::from(root).join(exe));}}
            }
        }
        for variable in ["ProgramFiles","ProgramFiles(x86)","LOCALAPPDATA"]{
            if let Some(root)=std::env::var_os(variable){for dir in if id=="uu"{vec!["Netease/UU","UU"]}else{vec!["xunyou"]}{for exe in &exes{candidates.push(PathBuf::from(&root).join(dir).join(exe));}}}
        }
        Accelerator{id:if id=="uu"{"uu"}else{"xunyou"},name,path:candidates.into_iter().find(|p|valid_exe(p)),website}
    }).collect()
}
#[tauri::command]
pub async fn list_accelerators(app:tauri::AppHandle)->Result<Vec<Accelerator>,String>{let paths=overrides(&app)?;tauri::async_runtime::spawn_blocking(move||discover(paths)).await.map_err(|e|e.to_string())}
#[tauri::command]
pub async fn set_accelerator_path(app:tauri::AppHandle,provider:String,path:Option<String>)->Result<(),String>{
    identity(&provider)?;let mut paths=overrides(&app)?;
    if let Some(path)=path{let path=PathBuf::from(path);if !valid_exe(&path){return Err("请选择有效的加速器 EXE 文件".into());}paths.insert(provider,path);}else{paths.remove(&provider);}
    let path=config(&app)?;if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    fs::write(path,serde_json::to_vec_pretty(&paths).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn open_accelerator(app:tauri::AppHandle,provider:String,website:bool)->Result<(),String>{
    let (_,url)=identity(&provider)?;
    if website{game_core::background_command("explorer.exe").arg(url).spawn().map_err(|e|e.to_string())?;return Ok(());}
    let paths=overrides(&app)?;
    let client=tauri::async_runtime::spawn_blocking(move||discover(paths).into_iter().find(|c|c.id==provider)).await.map_err(|e|e.to_string())?
        .and_then(|c|c.path).ok_or("未找到加速器，请指定程序位置或先安装客户端")?;
    tauri::async_runtime::spawn_blocking(move||open_client(&client)).await.map_err(|e|e.to_string())?

}
// The shell honors each client's manifest and presents Windows' normal UAC
// confirmation when required. The launcher itself stays unelevated.
#[cfg(target_os="windows")]
fn open_client(path:&Path)->Result<(),String>{
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::c_void;
    #[link(name="shell32")]
    extern "system" {fn ShellExecuteW(window:*mut c_void,verb:*const u16,file:*const u16,parameters:*const u16,directory:*const u16,show:i32)->isize;}
    let verb:Vec<u16>="open".encode_utf16().chain(Some(0)).collect();
    let file:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
    let directory:Vec<u16>=path.parent().unwrap_or_else(||Path::new(".")).as_os_str().encode_wide().chain(Some(0)).collect();
    let result=unsafe{ShellExecuteW(std::ptr::null_mut(),verb.as_ptr(),file.as_ptr(),std::ptr::null(),directory.as_ptr(),1)};
    if result>32{Ok(())}else{Err(format!("Windows 未能打开加速器（错误 {result}），请检查程序路径或系统权限确认"))}
}
#[cfg(not(target_os="windows"))]
fn open_client(_path:&Path)->Result<(),String>{Err("加速器启动需要 Windows".into())}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn only_known_providers_can_open_urls(){assert!(identity("uu").is_ok());assert!(identity("xunyou").is_ok());assert!(identity("https://example.com").is_err());}
    #[test]fn registry_icon_preserves_spaces_unicode_and_commas(){assert_eq!(icon_path(r#""E:\网易 UU\uu_launcher.exe",0"#),PathBuf::from(r"E:\网易 UU\uu_launcher.exe"));assert_eq!(icon_path(r"C:\apps,local\xunyou.exe,-1"),PathBuf::from(r"C:\apps,local\xunyou.exe"));}
}

mod process;
pub mod display;
pub mod display_config;
pub use process::{child_game_processes, game_processes};
// 第一阶段启动适配：Steam URI 与本地 EXE。
// Steam 启动器转交 Steam 客户端，暂不虚报实际游戏进程和游玩时长。
use game_core::LaunchTarget;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub fn epic_uri(identity: &str) -> Result<String,String> {
    let parts: Vec<_> = identity.split(':').collect();
    if ![1,3].contains(&parts.len()) || parts.iter().any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c,'_'|'-'|'.'))) {
        return Err("Epic 启动标识无效".into());
    }
    Ok(format!("com.epicgames.launcher://apps/{}?action=launch&silent=true",parts.join("%3A")))
}

/// Epic documents an installation-directory URI in addition to catalog identities.
/// Encode the complete UTF-8 path as one URI segment, including separators and colon.
pub fn epic_install_uri(install_path:&str)->Result<String,String>{
    let path=install_path.replace('\\',"/");let bytes=path.as_bytes();
    if bytes.len()<4||!bytes[0].is_ascii_alphabetic()||bytes[1]!=b':'||bytes[2]!=b'/'||path.contains('\0')||path.split('/').any(|part|part==".."||part=="."){
        return Err("Epic 安装目录无效".into());
    }
    let encoded=bytes.iter().map(|b|if b.is_ascii_alphanumeric()||b"-_.~".contains(b){(*b as char).to_string()}else{format!("%{b:02X}")}).collect::<String>();
    Ok(format!("com.epicgames.launcher://apps/{encoded}?action=launch&silent=true"))
}
pub fn install_steam(app_id: &str) -> Result<(),String> {
    if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) { return Err("无效 Steam App ID".into()); }
    game_core::background_command("explorer.exe").arg(format!("steam://install/{app_id}")).spawn().map_err(|e| e.to_string())?; Ok(())
}

pub enum LaunchResult {
    Direct(Child),
    Delegated,
}

pub fn open_platform(provider:&str,section:&str,battlenet_client:Option<&Path>)->Result<(),String>{
    if !["library","store","updates"].contains(&section){return Err("无效平台页面".into());}
    if provider=="battlenet" && section=="store" {game_core::background_command("explorer.exe").arg("https://shop.battlenet.com.cn/").spawn().map_err(|e|e.to_string())?;return Ok(());}
    if provider=="battlenet" {
        let launcher=battlenet_client.map(Path::to_path_buf).ok_or("未找到战网客户端")?;
        game_core::background_command(launcher).spawn().map_err(|e|e.to_string())?;return Ok(());
    }
    let uri=match (provider,section){
        ("steam","store")=>"steam://store",("steam","updates")=>"steam://open/downloads",("steam",_)=>"steam://nav/games",
        ("epic","store")=>"com.epicgames.launcher://store",("epic",_)=>"com.epicgames.launcher://library",
        _=>return Err("此来源没有平台客户端".into()),
    };
    game_core::background_command("explorer.exe").arg(uri).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e|e.to_string())?;Ok(())
}

pub fn launch(target: &LaunchTarget) -> Result<LaunchResult, String> {
    launch_with_client(target,None)
}
pub fn launch_with_client(target:&LaunchTarget,battlenet_client:Option<&Path>)->Result<LaunchResult,String>{
    if !cfg!(target_os = "windows") {
        return Err("游戏启动当前只支持 Windows，网页预览可浏览界面".into());
    }
    match target.provider.as_str() {
        "steam" => {
            if target.external_id.is_empty() || !target.external_id.chars().all(|c| c.is_ascii_digit()) {
                return Err("Steam 游戏 ID 不合法".into());
            }
            let uri = format!("steam://rungameid/{}", target.external_id);
            open_uri(&uri)?;
            Ok(LaunchResult::Delegated)
        }
        "epic" => {
            if !Path::new(&target.install_path).is_dir(){return Err("Epic 游戏安装目录不存在，请重新扫描游戏库".into());}
            let uri = epic_install_uri(&target.install_path)?;
            if let Err(protocol_error)=open_uri(&uri){
                let client=battlenet_client.ok_or(protocol_error)?;
                game_core::background_command(client).current_dir(client.parent().unwrap_or(Path::new("."))).arg(&uri)
                    .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
                    .spawn().map_err(|e|format!("无法启动 Epic：{e}"))?;
            }
            Ok(LaunchResult::Delegated)
        }
        "battlenet" => {
            // Open the official Battle.net client. A stable per-title launch protocol
            // is not assumed; game-specific launch profiles will be validated separately.
            let mut candidates = Vec::new();
            if let Some(client)=battlenet_client {candidates.push(client.to_path_buf());}
            if let Some(path) = std::env::var_os("BATTLENET_LAUNCHER_EXE") {
                candidates.push(std::path::PathBuf::from(path));
            }
            for key in ["ProgramFiles(x86)", "ProgramFiles"] {
                if let Some(root) = std::env::var_os(key) {
                    let directory = std::path::PathBuf::from(root).join("Battle.net");
                    candidates.push(directory.join("Battle.net Launcher.exe"));
                    candidates.push(directory.join("Battle.net.exe"));
                }
            }
            let launcher = candidates.into_iter().find(|p| p.is_file())
                .ok_or_else(|| "未找到 Battle.net 客户端；可设置 BATTLENET_LAUNCHER_EXE 指定路径".to_string())?;
            let code=match target.external_id.as_str() {"diablo_iv"=>"Fen","diablo_iii"=>"D3","diablo_iii_cn"=>"D3CN","world_of_warcraft"=>"WoW","overwatch"=>"Pro","hearthstone"=>"WTCG","starcraft_ii"=>"S2","starcraft"=>"S1","heroes_of_the_storm"=>"Hero","warcraft_iii"=>"W3","diablo_ii_resurrected"=>"OSI",_=>return Err("不支持的战网游戏标识".into())};
            game_core::background_command(launcher).arg(format!("--exec=launch {code}")).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
                .spawn().map_err(|e| format!("无法启动 Battle.net 客户端：{e}"))?;
            Ok(LaunchResult::Delegated)
        }
        "manual" => {
            let path = Path::new(&target.install_path);
            if !path.is_file() || !path.extension().is_some_and(|ext| ext.to_str().is_some_and(|name| name.eq_ignore_ascii_case("exe"))) {
                return Err("本地游戏的 EXE 文件不存在".into());
            }
            let mut command = Command::new(path);
            if let Some(parent) = path.parent() { command.current_dir(parent); }
            let child = command.spawn().map_err(|e| format!("启动游戏失败：{e}"))?;
            Ok(LaunchResult::Direct(child))
        }
        other => Err(format!("{other} 的启动适配器尚未接入")),
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test]fn epic_installation_uri_encodes_spaces_unicode_and_query_characters(){
        assert_eq!(epic_install_uri(r"G:\DeathStranding").unwrap(),"com.epicgames.launcher://apps/G%3A%2FDeathStranding?action=launch&silent=true");
        assert_eq!(epic_install_uri("G:/Game A&x=?/游戏").unwrap(),"com.epicgames.launcher://apps/G%3A%2FGame%20A%26x%3D%3F%2F%E6%B8%B8%E6%88%8F?action=launch&silent=true");
        for path in ["","relative","G:/","G:/Games/../Other","G:/Games/./Other","G:/Game\0"]{assert!(epic_install_uri(path).is_err());}
    }
    #[test] fn qualified_epic_identity_is_encoded_and_rejects_injection() {
        assert_eq!(epic_uri("space:catalog:Ginger").unwrap(),"com.epicgames.launcher://apps/space%3Acatalog%3AGinger?action=launch&silent=true");
        for invalid in ["", "a:b", "a::b", "x?cmd=1", "a/b:c:d"] { assert!(epic_uri(invalid).is_err()); }
    }
}

/// ShellExecute reports missing associations; explorer spawning successfully does not.
#[cfg(windows)]
pub fn open_uri(uri:&str)->Result<(),String>{
    use std::os::windows::ffi::OsStrExt;
    #[link(name="shell32")]
    extern "system" {fn ShellExecuteW(window:*mut std::ffi::c_void,operation:*const u16,file:*const u16,parameters:*const u16,directory:*const u16,show:i32)->isize;}
    let wide=|s:&str|std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let op=wide("open");let file=wide(uri);
    let result=unsafe{ShellExecuteW(std::ptr::null_mut(),op.as_ptr(),file.as_ptr(),std::ptr::null(),std::ptr::null(),1)};
    if result<=32{Err(format!("平台启动失败（Windows {result}），请检查官方客户端是否已安装并登录"))}else{Ok(())}
}
#[cfg(not(windows))]pub fn open_uri(_uri:&str)->Result<(),String>{Err("仅支持 Windows".into())}

static ACTIVE:std::sync::Mutex<Vec<String>>=std::sync::Mutex::new(Vec::new());
pub struct Reservation(String);
impl Drop for Reservation{fn drop(&mut self){if let Ok(mut active)=ACTIVE.lock(){active.retain(|id|id!=&self.0);}}}
pub fn reserve(id:&str)->Result<Reservation,String>{let mut active=ACTIVE.lock().map_err(|_|"启动状态不可用")?;if active.iter().any(|v|v==id){return Err("此游戏正在启动或运行".into());}active.push(id.into());Ok(Reservation(id.into()))}

pub fn has_active_session() -> bool {
    ACTIVE.lock().map_or(true, |active| !active.is_empty())
}

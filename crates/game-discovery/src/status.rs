//! Local client state is evidence of a pending update, not a live latest-version check.
use game_core::DiscoveredGame;
use std::path::Path;
#[derive(Debug,Clone)] pub struct InstallationStatus {
    pub provider:String,pub external_id:String,pub status:String,pub version:Option<String>,pub detail:String,
}
fn state(game:&DiscoveredGame,status:&str,version:Option<String>,detail:&str)->InstallationStatus {
    InstallationStatus {provider:game.provider.clone(),external_id:game.external_id.clone(),status:status.into(),version,detail:detail.into()}
}
pub fn steam_flags(flags:Option<u64>,target:Option<&str>,build:Option<&str>)->&'static str {
    let Some(flags)=flags else{return "unknown"};
    if flags&512!=0 {"paused"} else if flags&(256|1048576|2097152|4194304)!=0 {"updating"}
    else if flags&(32|128|131072)!=0 {"repair_required"}
    else if flags&2!=0 || target.is_some_and(|t|t!="0"&&!t.is_empty()&&build.is_some_and(|b|!b.is_empty()&&b!=t)) {"update_available"}
    else if flags&4!=0 {"not_required"} else {"unknown"}
}
pub fn build_info(text:&str)->Option<String> {
    let mut lines=text.lines();let headers:Vec<_>=lines.next()?.split('|').collect();
    let version=headers.iter().position(|h|h.starts_with("Version!"))?;
    let active=headers.iter().position(|h|h.starts_with("Active!"))?;
    lines.find_map(|line|{let row:Vec<_>=line.split('|').collect();(row.get(active)==Some(&"1")).then(||row.get(version).filter(|v|!v.is_empty()).map(|v|v.to_string())).flatten()})
}
pub fn collect(games:&[DiscoveredGame])->Vec<InstallationStatus> {
    games.iter().map(|game| {
        if game.provider=="steam" {
            let path=Path::new(&game.install_path).parent().and_then(Path::parent).map(|p|p.join(format!("appmanifest_{}.acf",game.external_id)));
            if let Some(text)=path.and_then(|p|std::fs::read_to_string(p).ok()) {
                let flags=super::steam::extract_value(&text,"StateFlags").and_then(|v|v.parse().ok());
                let build=super::steam::extract_value(&text,"buildid");let target=super::steam::extract_value(&text,"TargetBuildID");
                return state(game,steam_flags(flags,target.as_deref(),build.as_deref()),build,"Steam 本地清单状态；未联网确认最新版本");
            }
        }
        if game.provider=="epic" { if let Some((status,version))=super::epic::local_status(&game.external_id) {return state(game,status,version,"Epic 本地安装清单；请在客户端检查新版本");} }
        let version=if game.provider=="battlenet" {std::fs::read_to_string(Path::new(&game.install_path).join(".build.info")).ok().and_then(|t|build_info(&t))}else{None};
        state(game,"unknown",version,if game.provider=="battlenet" {"已读取本地版本；请在战网客户端检查更新"}else{"此来源暂不提供更新状态"})
    }).collect()
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn steam_pending_update_is_not_confused_with_installed() {
        assert_eq!(steam_flags(Some(4),None,Some("100")),"not_required");
        assert_eq!(steam_flags(Some(6),None,None),"update_available");
        assert_eq!(steam_flags(Some(4),Some("101"),Some("100")),"update_available");
        assert_eq!(steam_flags(Some(514),None,None),"paused");assert_eq!(steam_flags(None,None,None),"unknown");
    }
    #[test] fn completed_download_counters_do_not_mean_pending_update(){
        let root=std::env::temp_dir().join(format!("gamedeck-status-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let folder=root.join("steamapps/common/ItTakesTwo");std::fs::create_dir_all(&folder).unwrap();
        let manifest=root.join("steamapps/appmanifest_1426210.acf");
        std::fs::write(&manifest,r#""AppState"
{
 "StateFlags" "4"
 "buildid" "26237470"
 "TargetBuildID" "26237470"
 "BytesToDownload" "47549798464"
 "BytesDownloaded" "47549798464"
}"#).unwrap();
        let game=DiscoveredGame{provider:"steam".into(),external_id:"1426210".into(),name:"It Takes Two".into(),install_path:folder.to_string_lossy().into_owned()};
        let result=collect(&[game]);assert_eq!(result[0].status,"not_required");assert_eq!(result[0].version.as_deref(),Some("26237470"));
        std::fs::remove_file(manifest).unwrap();std::fs::remove_dir(folder).unwrap();std::fs::remove_dir(root.join("steamapps/common")).unwrap();std::fs::remove_dir(root.join("steamapps")).unwrap();std::fs::remove_dir(root).unwrap();
    }
    #[test] fn build_info_uses_active_row_and_named_column() {
        assert_eq!(build_info("Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|0|old\ncn|1|3.2.1.73552\n").as_deref(),Some("3.2.1.73552"));
        assert!(build_info("malformed").is_none());
    }
}

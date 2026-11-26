//! Epic Games Launcher local installation manifests (.item).
//! Inspired by platform-specific importer architecture; no third-party source copied.
use game_core::DiscoveredGame;
use serde::Deserialize;
use std::{collections::HashSet, fs, path::{Path, PathBuf}};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Manifest {
    display_name: Option<String>,
    install_location: Option<String>,
    #[serde(rename = "AppName")]
    app_name: Option<String>,
    #[serde(rename = "CatalogItemId")]
    catalog_item_id: Option<String>,
    catalog_namespace: Option<String>,
    launch_executable: Option<String>,
    #[serde(rename = "MainGameAppName")]
    main_game_app_name: Option<String>,
    #[serde(rename = "bIsIncompleteInstall")]
    incomplete: Option<bool>,
    #[serde(rename = "bNeedsValidation")]
    needs_validation: Option<bool>,
    app_version_string: Option<String>,
}
pub fn local_status(app_name:&str)->Option<(&'static str,Option<String>)> {
    for root in roots() {let Ok(entries)=fs::read_dir(root)else{continue};for file in entries.flatten().map(|e|e.path()).filter(|p|p.extension().is_some_and(|e|e.eq_ignore_ascii_case("item"))) {
        let Some(item)=fs::read_to_string(file).ok().and_then(|s|serde_json::from_str::<Manifest>(&s).ok())else{continue};
        if item.app_name.as_deref()==Some(app_name) {return Some((if item.incomplete.unwrap_or(false){"installing"}else if item.needs_validation.unwrap_or(false){"repair_required"}else{"unknown"},item.app_version_string));}
    }}None
}

#[derive(Debug, Clone)]
pub struct EpicIdentity { pub app_name: String, pub namespace: String, pub catalog_id: String }
impl EpicIdentity {
    pub fn qualified(&self) -> String { format!("{}:{}:{}",self.namespace,self.catalog_id,self.app_name) }
}
fn is_base(item: &Manifest) -> bool {
    item.app_name.as_ref().is_some_and(|app| item.main_game_app_name.as_ref().is_none_or(|main| main.is_empty() || main == app))
        && item.launch_executable.as_ref().is_none_or(|exe| !exe.trim().is_empty())

}
pub fn identity(app_name: &str) -> Option<EpicIdentity> {
    for root in roots() {
        let Ok(entries) = fs::read_dir(root) else { continue };
        for file in entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("item"))) {
            let Some(item) = fs::read_to_string(file).ok().and_then(|s| serde_json::from_str::<Manifest>(&s).ok()) else { continue };
            if is_base(&item) && item.app_name.as_deref() == Some(app_name) {
                return Some(EpicIdentity { app_name: app_name.into(), namespace: item.catalog_namespace?, catalog_id: item.catalog_item_id? });
            }
        }
    }
    None
}

/// The installation keeps its public identity even when the global launcher manifest is unavailable.
pub fn identity_at(app_name:&str,install_path:&Path)->Option<EpicIdentity>{
    if let Some(identity)=identity(app_name){return Some(identity);}
    for entry in fs::read_dir(install_path.join(".egstore")).ok()?.flatten(){
        let path=entry.path();if !path.extension().is_some_and(|e|e.eq_ignore_ascii_case("mancpn")){continue;}
        let Some(item)=fs::read_to_string(path).ok().and_then(|s|serde_json::from_str::<Manifest>(&s).ok())else{continue};
        if item.app_name.as_deref()==Some(app_name){return Some(EpicIdentity{app_name:app_name.into(),namespace:item.catalog_namespace?,catalog_id:item.catalog_item_id?});}
    }
    None
}

pub fn launcher_path(install_path:&Path)->Option<PathBuf>{
    let mut candidates=Vec::new();
    if let Some(path)=std::env::var_os("EPIC_LAUNCHER_EXE"){candidates.push(PathBuf::from(path));}
    let mut roots=Vec::new();
    for key in ["ProgramFiles(x86)","ProgramFiles"]{if let Some(root)=std::env::var_os(key){roots.push(PathBuf::from(root).join("Epic Games"));}}
    if let Some(root)=install_path.ancestors().last(){roots.push(root.join("Epic Games"));}
    #[cfg(windows)]
    for drive in b'C'..=b'Z'{let root=PathBuf::from(format!("{}:\\",drive as char));for folder in ["Epic Games","Program Files (x86)/Epic Games","Program Files/Epic Games"]{roots.push(root.join(folder));}}
    for root in roots{for arch in ["Win32","Win64"]{candidates.push(root.join("Launcher/Portal/Binaries").join(arch).join("EpicGamesLauncher.exe"));}}
    if let Some(local)=std::env::var_os("LOCALAPPDATA"){candidates.push(PathBuf::from(local).join("Epic Games/Launcher/EpicGamesLauncher.exe"));}
    candidates.into_iter().find(|p|p.is_file())
}

fn roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(p) = std::env::var_os("EPIC_MANIFEST_DIR") { roots.push(PathBuf::from(p)); }
    if let Some(p) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(p).join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests"));
    }
    roots
}

pub fn scan() -> (Vec<DiscoveredGame>, Vec<String>) {
    scan_roots(&roots())
}

fn scan_roots(roots: &[PathBuf]) -> (Vec<DiscoveredGame>, Vec<String>) {
    let mut games = Vec::new();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            let file = entry.path();
            if !file.extension().is_some_and(|e| e.eq_ignore_ascii_case("item")) { continue; }
            let content = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => { errors.push(format!("{}: {e}", file.display())); continue; }
            };
            let item: Manifest = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => { errors.push(format!("{}: {e}", file.display())); continue; }
            };
            if !is_base(&item) { continue; }
            let Some(path) = item.install_location.filter(|s| !s.trim().is_empty()) else { continue };
            if !Path::new(&path).is_dir() { continue; }
            if item.incomplete.unwrap_or(false) && !item.launch_executable.as_deref().is_some_and(|exe|Path::new(&path).join(exe).is_file()) { continue; }
            let Some(name) = item.display_name.filter(|s| !s.trim().is_empty()) else { continue };
            let Some(app_name) = item.app_name.filter(|s| !s.trim().is_empty()) else { continue };
            // AppName is the launcher identity, CatalogItemId is not necessarily unique to an installation.
            if !seen.insert(app_name.clone()) { continue; }
            let _catalog_item_id = item.catalog_item_id;
            games.push(DiscoveredGame { provider: "epic".into(), external_id: app_name, name, install_path: path });
        }
    }
    (games, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_manifest() {
        let m: Manifest = serde_json::from_str(r#"{"DisplayName":"Example","InstallLocation":"C:\\Games\\Example","AppName":"ExampleApp","bIsIncompleteInstall":false}"#).unwrap();
        assert_eq!(m.app_name.as_deref(), Some("ExampleApp"));
        assert_eq!(m.incomplete, Some(false));
    }
    #[test] fn dlc_does_not_replace_base_game() {
        let addon: Manifest = serde_json::from_str(r#"{"AppName":"PhantomLiberty","MainGameAppName":"Ginger","LaunchExecutable":""}"#).unwrap();
        assert!(!is_base(&addon));
        let base: Manifest = serde_json::from_str(r#"{"AppName":"Ginger","MainGameAppName":"Ginger","LaunchExecutable":"redprelauncher.exe"}"#).unwrap();
        assert!(is_base(&base));
    }
}

//! Steam 本机库增量发现。扫描 libraryfolders.vdf 与 appmanifest_*.acf。
//! 无需登录 Steam API，兼容 Steam 库分布在多个盘符的情况。
use game_core::DiscoveredGame;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
fn registry_steam_root() -> Option<PathBuf> {
    let output = game_core::background_command("reg.exe")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .output().ok()?;
    if !output.status.success() { return None; }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| line.split_once("REG_SZ")
        .map(|(_, path)| PathBuf::from(path.trim().replace('/', "\\"))))
}

pub(crate) fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(target_os = "windows")]
    if let Some(p) = registry_steam_root() { roots.push(p); }
    if let Some(p) = std::env::var_os("STEAM_DIR") { roots.push(PathBuf::from(p)); }
    if let Some(p) = std::env::var_os("ProgramFiles(x86)") { roots.push(PathBuf::from(p).join("Steam")); }
    if let Some(p) = std::env::var_os("ProgramFiles") { roots.push(PathBuf::from(p).join("Steam")); }
    if let Some(p) = std::env::var_os("LOCALAPPDATA") { roots.push(PathBuf::from(p).join("Steam")); }
    roots
}

pub fn artwork_cache_roots() -> Vec<PathBuf> {
    steam_roots().into_iter().map(|root| root.join("appcache").join("librarycache"))
        .filter(|path| path.is_dir()).collect()
}

pub(crate) fn extract_value(input: &str, key: &str) -> Option<String> {
    let escaped_key = regex::escape(key);
    let re = Regex::new(&format!(r#"(?im)^\s*"{}"\s+"([^"\r\n]*)""#, escaped_key)).ok()?;
    re.captures(input).map(|c| c[1].to_string())
}

fn library_paths(root: &Path) -> Vec<PathBuf> {
    let mut results = vec![root.to_path_buf()];
    let manifest = root.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = fs::read_to_string(manifest) {
        if let Ok(re) = Regex::new(r#"(?im)^\s*"path"\s+"([^"\r\n]*)""#) {
            for cap in re.captures_iter(&text) {
                let normalized = cap[1].replace("\\\\", "\\");
                results.push(PathBuf::from(normalized));
            }
        }
    }
    results
}

/// 同一 appid 出现多个 Steam 库时保留首个有效安装，返回可供持久化的扫描结果。
pub fn scan() -> (Vec<DiscoveredGame>, Vec<String>) {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let mut seen_roots = HashSet::new();
    let mut seen_games = HashSet::new();
    for root in steam_roots() {
        if !root.is_dir() { continue; }
        for library in library_paths(&root) {
            if !seen_roots.insert(library.to_string_lossy().to_lowercase()) { continue; }
            let steamapps = library.join("steamapps");
            let Ok(entries) = fs::read_dir(&steamapps) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let filename = entry.file_name().to_string_lossy().to_string();
                if !filename.starts_with("appmanifest_") || !filename.ends_with(".acf") { continue; }
                let content = match fs::read_to_string(&path) {
                    Ok(x) => x,
                    Err(err) => { errors.push(format!("{}: {err}", path.display())); continue; }
                };
                let Some(appid) = extract_value(&content, "appid") else { continue };
                if appid == "228980" { continue; }
                if !appid.chars().all(|c| c.is_ascii_digit()) { continue; }
                if seen_games.contains(&appid) { continue; }
                let Some(folder) = extract_value(&content, "installdir") else { continue };
                // 不允许来自 manifest 的绝对路径与 .. 绕过 Steam library/common。
                let folder_path = Path::new(&folder);
                if folder_path.is_absolute() || folder_path.components().count() != 1 { continue; }
                let install_path = steamapps.join("common").join(folder);
                if !install_path.is_dir() { continue; }
                let name = extract_value(&content, "name")
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| filename.clone());
                seen_games.insert(appid.clone());
                found.push(DiscoveredGame {
                    provider: "steam".into(), external_id: appid, name,
                    install_path: install_path.to_string_lossy().to_string(),
                });
            }
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    (found, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_acf_keys() {
        let content = "\"AppState\"\n{\n  \"appid\"  \"1245620\"\n  \"name\"  \"ELDEN RING\"\n  \"installdir\"  \"ELDEN RING\"\n}";
        assert_eq!(extract_value(content, "appid").as_deref(), Some("1245620"));
        assert_eq!(extract_value(content, "installdir").as_deref(), Some("ELDEN RING"));
    }
}

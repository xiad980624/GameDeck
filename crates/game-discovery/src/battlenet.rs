//! Battle.net installed-game discovery via Windows uninstall registry entries.
//! Native product.db installation records take precedence over uninstall-registry fallbacks.
use game_core::DiscoveredGame;
use std::{collections::HashSet, path::{Path, PathBuf}};

const TITLES: &[(&str, &str, &[&str])] = &[
    ("diablo_iii_cn", "Diablo III (China)", &["Diablo III (China)","暗黑破坏神III","暗黑破坏神Ⅲ"]),
    ("world_of_warcraft", "World of Warcraft", &["World of Warcraft"]),
    ("diablo_iii", "Diablo III", &["Diablo III", "Diablo 3"]),
    ("diablo_iv", "Diablo IV", &["Diablo IV", "Diablo 4"]),
    ("overwatch", "Overwatch", &["Overwatch", "Overwatch 2"]),
    ("hearthstone", "Hearthstone", &["Hearthstone"]),
    ("starcraft_ii", "StarCraft II", &["StarCraft II", "StarCraft 2"]),
    ("starcraft", "StarCraft", &["StarCraft Remastered", "StarCraft"]),
    ("heroes_of_the_storm", "Heroes of the Storm", &["Heroes of the Storm"]),
    ("warcraft_iii", "Warcraft III", &["Warcraft III", "Warcraft 3"]),
    ("diablo_ii_resurrected", "Diablo II: Resurrected", &["Diablo II Resurrected", "Diablo II: Resurrected"]),
];

pub fn product_identity(code:&str)->Option<(&'static str,&'static str)> {
    let id=match code.to_ascii_lowercase().as_str() {
        "fenris"|"fen"=>"diablo_iv", "d3cn"=>"diablo_iii_cn", "diablo3"|"d3"=>"diablo_iii", "wow"|"wowcn"=>"world_of_warcraft",
        "prometheus"|"pro"=>"overwatch", "hs_beta"|"wtcg"=>"hearthstone", "s2"|"sc2"=>"starcraft_ii", "s1"=>"starcraft",
        "heroes"|"hero"=>"heroes_of_the_storm", "w3"=>"warcraft_iii", "osi"=>"diablo_ii_resurrected", _=>return None,
    };
    TITLES.iter().find(|(key,_,_)|*key==id).map(|(id,name,_)|(*id,*name))
}
fn database_products()->(Vec<super::product_db::Product>,Vec<String>) {
    let mut paths=Vec::new();
    if let Some(path)=std::env::var_os("BATTLENET_PRODUCT_DB") {paths.push(PathBuf::from(path));}
    if let Some(root)=std::env::var_os("ProgramData") {paths.push(PathBuf::from(root).join("Battle.net/Agent/product.db"));}
    let mut products=Vec::new();let mut errors=Vec::new();
    for path in paths {if !path.is_file(){continue} match std::fs::read(&path).map_err(|e|e.to_string()).and_then(|bytes|super::product_db::parse(&bytes)) {
        Ok(records)=>products.extend(records),Err(error)=>errors.push(format!("Battle.net product.db: {error}")),
    }}
    (products,errors)
}
pub fn launcher_path()->Option<PathBuf> {
    let (products,_)=database_products();
    products.into_iter().find(|p|p.code=="bna").and_then(|p|[p.path.join("Battle.net Launcher.exe"),p.path.join("Battle.net.exe")].into_iter().find(|p|p.is_file()))
}
fn valid_install(path:&Path)->bool { path.is_dir() && (path.join(".build.info").is_file() || ["Diablo IV.exe","Diablo III.exe","Hearthstone.exe","Overwatch Launcher.exe","StarCraft II.exe","StarCraft.exe","Warcraft III.exe","Wow.exe"].iter().any(|exe|path.join(exe).is_file())) }

fn classify(name: &str) -> Option<(&'static str, &'static str)> {
    let n = name.trim().to_ascii_lowercase();
    TITLES.iter().find_map(|(id, title, aliases)| {
        aliases.iter().any(|a| a.to_ascii_lowercase() == n).then_some((*id, *title))
    })
}

fn reg_value(record: &str, key: &str) -> Option<String> {
    record.lines().find_map(|line| {
        let line = line.trim();
        let (left, value) = line.split_once("REG_SZ")?;
        (left.trim().eq_ignore_ascii_case(key) && !value.trim().is_empty())
            .then(|| value.trim().trim_matches('"').to_string())
    })
}

fn parse_registry_entries(raw: &str) -> Vec<(String, PathBuf)> {
    raw.split("\n\n").filter_map(|block| {
        let name = reg_value(block, "DisplayName")?;
        let location = reg_value(block, "InstallLocation")?;
        Some((name, PathBuf::from(location)))
    }).collect()
}

#[cfg(target_os = "windows")]
fn registry_entries() -> (Vec<(String, PathBuf)>, Vec<String>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for hive in [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ] {
        match game_core::background_command("reg.exe").args(["query", hive, "/s"]).output() {
            Ok(output) if output.status.success() => {
                // reg.exe output is encoded in the system code page on some Windows installations.
                // ASCII Blizzard titles and paths work with lossy conversion; non-ASCII paths need
                // a future native windows-rs registry implementation.
                let output = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
                // Each key starts with HKEY; blank lines are not reliable separators.
                for block in output.split("\nHKEY") {
                    entries.extend(parse_registry_entries(&format!("HKEY{block}").replace("\n    ", "\n")));
                }
            }
            Ok(_) => {},
            Err(err) => errors.push(format!("Battle.net registry scan {hive}: {err}")),
        }
    }
    (entries, errors)
}

#[cfg(not(target_os = "windows"))]
fn registry_entries() -> (Vec<(String, PathBuf)>, Vec<String>) { (Vec::new(), Vec::new()) }

fn discover(entries: &[(String, PathBuf)]) -> Vec<DiscoveredGame> {
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for (display_name, path) in entries {
        let Some((id, name)) = classify(display_name) else { continue };
        if !valid_install(path) || !seen.insert(id) { continue; }
        found.push(DiscoveredGame {
            provider: "battlenet".into(), external_id: id.into(), name: name.into(),
            install_path: path.to_string_lossy().to_string(),
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

pub fn scan() -> (Vec<DiscoveredGame>, Vec<String>) {
    let (products,mut errors)=database_products();
    let mut entries:Vec<_>=products.into_iter().filter_map(|p|product_identity(&p.code).map(|(_,name)|(name.to_string(),p.path))).collect();
    let (fallback,registry_errors) = registry_entries();entries.extend(fallback);errors.extend(registry_errors);
    // Optional user-configured game root: only known game folder names are considered.
    if let Some(root) = std::env::var_os("BATTLENET_GAMES_DIR") {
        let root = Path::new(&root);
        for (_, title, aliases) in TITLES {
            for folder in aliases.iter().chain(std::iter::once(title)) {
                let path = root.join(folder);
                if path.is_dir() { entries.push(((*title).into(), path)); break; }
            }
        }
    }
    (discover(&entries), errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classify_known_titles_without_fuzzy_false_positives() {
        assert_eq!(classify("Diablo IV"), Some(("diablo_iv", "Diablo IV")));
        assert_eq!(classify("Overwatch 2"), Some(("overwatch", "Overwatch")));
        assert_eq!(classify("Battle.net"), None);
        assert_eq!(classify("Diablo IV Update Helper"), None);
    }
    #[test]
    fn parse_reg_values() {
        let input = "DisplayName    REG_SZ    Diablo IV\nInstallLocation    REG_SZ    C:\\Games\\Diablo IV\n";
        let items = parse_registry_entries(input);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "Diablo IV");
    }
}

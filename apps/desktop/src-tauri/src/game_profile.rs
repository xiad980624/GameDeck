//! Game-native borderless launch.
//!
//! Steam's `steam://rungameid/<id>` URI cannot carry command-line arguments and
//! Steam does not hot-reload `localconfig.vdf`, so launch flags are not a viable
//! route. Instead, for engines whose fullscreen mode is stored in a well-known
//! config file that the game reads on every launch, we rewrite that file right
//! before launching. Because GameDeck controls the launch, a game overwriting
//! the value on exit is harmless: we write it again on the next launch.
//!
//! Only profiles explicitly listed here are touched. Everything else is left
//! completely alone and reported as unsupported.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What the profile writer does for a matched game.
#[derive(Debug, Clone)]
pub struct ProfileOutcome {
    pub game_name: String,
    pub applied: bool,
    pub config_path: Option<String>,
    pub detail: String,
}

/// How to locate and rewrite a game's config for borderless windowed mode.
struct Profile {
    app_id: &'static str,
    /// Candidate config paths, relative to a known root (LOCALAPPDATA / My Games / Documents).
    /// All candidates are rewritten; some titles keep multiple copies.
    configs: &'static [&'static str],
    /// Engine convention: UE uses FullscreenMode=1 for borderless fullscreen.
    /// `MainMonitorID` (when present) pins the game to a specific physical screen
    /// and defeats the TV agent, so it is cleared.
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    /// Unreal Engine `GameUserSettings.ini`.
    Unreal,
}

// Confirmed by scanning this machine. Each path exists and contains the
// FullscreenMode / ScreenMode fields as of 2026-11-23.
static PROFILES: &[Profile] = &[
    // 影之刃零: two live copies, newest is ProjectPBZ (b1 is a stale 2025 copy).
    Profile {
        app_id: "4115450",
        configs: &[
            "LOCALAPPDATA/ProjectPBZ/Saved/Config/Windows/GameUserSettings.ini",
            "LOCALAPPDATA/b1/Saved/Config/Windows/GameUserSettings.ini",
        ],
        kind: Kind::Unreal,
    },
    // 幽灵行者: window-mode fields present, currently FullscreenMode=0.
    Profile {
        app_id: "1139900",
        configs: &["LOCALAPPDATA/Ghostrunner/Saved/Config/WindowsNoEditor/GameUserSettings.ini"],
        kind: Kind::Unreal,
    },
    Profile {
        app_id: "1249200",
        configs: &["LOCALAPPDATA/Ghostrunner/Saved/Config/WindowsNoEditor/GameUserSettings.ini"],
        kind: Kind::Unreal,
    },
    // RUINER: config exists (only LastConfirmedFullscreenMode present).
    Profile {
        app_id: "464060",
        configs: &["LOCALAPPDATA/Ruiner/Saved/Config/WindowsNoEditor/GameUserSettings.ini"],
        kind: Kind::Unreal,
    },
    // 剑星: FullscreenMode=0.
    Profile {
        app_id: "3489700",
        configs: &[
            "LOCALAPPDATA/SB/Saved/Config/WindowsNoEditor/GameUserSettings.ini",
            "DOCUMENTS/StellarBlade/76561199250348738/Demo/WindowsNoEditor/GameUserSettings.ini",
        ],
        kind: Kind::Unreal,
    },
    // 无主之地3 (Epic, identified by catalog namespace id from the DB).
    Profile {
        app_id: "Catnip",
        configs: &["MYGAMES/Borderlands 3/Saved/Config/WindowsNoEditor/GameUserSettings.ini"],
        kind: Kind::Unreal,
    },
    // 无主之地4: FullscreenMode=2 (windowed).
    Profile {
        app_id: "1285190",
        configs: &["MYGAMES/Borderlands 4/Saved/Config/Windows/GameUserSettings.ini"],
        kind: Kind::Unreal,
    },
];

fn expand_root(path: &str) -> Option<PathBuf> {
    if let Some(rel) = path.strip_prefix("LOCALAPPDATA/") {
        return std::env::var_os("LOCALAPPDATA").map(|root| Path::new(&root).join(rel));
    }
    if let Some(rel) = path.strip_prefix("MYGAMES/") {
        let docs = std::env::var_os("USERPROFILE")
            .map(|p| Path::new(&p).join("Documents").join("My Games"));
        return docs.map(|root| root.join(rel));
    }
    if let Some(rel) = path.strip_prefix("DOCUMENTS/") {
        let docs = std::env::var_os("USERPROFILE").map(|p| Path::new(&p).join("Documents"));
        return docs.map(|root| root.join(rel));
    }
    None
}

fn stamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

/// Rewrite a UE GameUserSettings.ini to borderless fullscreen and clear the
/// screen-pinning id. Returns (changed, human-readable detail).
fn rewrite_unreal(path: &Path) -> Result<(bool, String), String> {
    let original = fs::read_to_string(path).map_err(|e| format!("读取配置失败: {e}"))?;
    let mut out = String::with_capacity(original.len() + 64);
    let mut changed = false;

    for line in original.lines() {
        let trimmed = line.trim_end_matches('\r');
        let rewritten = if let Some(_rest) = trimmed.strip_prefix("FullscreenMode=") {
            if trimmed != "FullscreenMode=1" {
                changed = true;
                "FullscreenMode=1"
            } else {
                trimmed
            }
        } else if let Some(_rest) = trimmed.strip_prefix("PreferredFullscreenMode=") {
            if trimmed != "PreferredFullscreenMode=1" {
                changed = true;
                "PreferredFullscreenMode=1"
            } else {
                trimmed
            }
        } else if trimmed.starts_with("MainMonitorID=") {
            // Empty string lets the engine pick the current primary monitor,
            // which is the TV during a temp primary switch.
            if trimmed != "MainMonitorID=\"\"" {
                changed = true;
                "MainMonitorID=\"\""
            } else {
                trimmed
            }
        } else {
            trimmed
        };
        out.push_str(rewritten);
        out.push('\n');
    }

    if !changed {
        return Ok((false, "已是无边框窗口化".into()));
    }

    // Back up once so the user's original display settings are recoverable.
    let backup = path.with_extension(format!("ini.gamedeck-{}", stamp()));
    if !backup.exists() {
        let _ = fs::copy(path, &backup);
    }
    fs::write(path, out.as_bytes()).map_err(|e| format!("写入配置失败: {e}"))?;
    Ok((true, format!("已设为无边框并清除锁屏，备份 {}", backup.display())))
}

/// Apply borderless profile for the game about to launch.
/// Never fails the launch: any problem is reported and the game starts as-is.
pub fn prepare(app_id: &str, game_name: &str) -> Vec<ProfileOutcome> {
    let Some(profile) = PROFILES.iter().find(|p| p.app_id == app_id) else {
        return vec![ProfileOutcome {
            game_name: game_name.to_string(),
            applied: false,
            config_path: None,
            detail: "未收录该游戏的无边框配置（需要手动在游戏内设置）".into(),
        }];
    };

    let mut outcomes = Vec::new();
    for spec in profile.configs {
        let Some(path) = expand_root(spec) else { continue };
        if !path.is_file() {
            outcomes.push(ProfileOutcome {
                game_name: game_name.to_string(),
                applied: false,
                config_path: Some(path.to_string_lossy().into_owned()),
                detail: "配置文件不存在（该游戏可能尚未生成设置）".into(),
            });
            continue;
        }
        let result = match profile.kind {
            Kind::Unreal => rewrite_unreal(&path),
        };
        outcomes.push(match result {
            Ok((changed, detail)) => ProfileOutcome {
                game_name: game_name.to_string(),
                applied: changed,
                config_path: Some(path.to_string_lossy().into_owned()),
                detail,
            },
            Err(error) => ProfileOutcome {
                game_name: game_name.to_string(),
                applied: false,
                config_path: Some(path.to_string_lossy().into_owned()),
                detail: error,
            },
        });
    }
    outcomes
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_unreal_config_to_borderless_and_clears_monitor_pin() {
        let dir = std::env::temp_dir().join(format!("gamedeck-profile-test-{}", stamp()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("GameUserSettings.ini");
        fs::write(&file,
            "[/Script/Engine.GSEngineUserSettings]\r\nFullscreenMode=0\r\nPreferredFullscreenMode=2\r\nMainMonitorID=\"MONITOR\\AUS275A\\{guid}\\0008\"\r\nResolutionSizeX=2560\r\n")
            .unwrap();

        let (changed, _) = rewrite_unreal(&file).unwrap();
        assert!(changed);
        let after = fs::read_to_string(&file).unwrap();
        assert!(after.contains("FullscreenMode=1"), "FullscreenMode not rewritten: {after}");
        assert!(after.contains("PreferredFullscreenMode=1"));
        assert!(after.contains("MainMonitorID=\"\""), "monitor pin not cleared: {after}");
        assert!(after.contains("ResolutionSizeX=2560"), "unrelated line lost");
        assert!(after.find("FullscreenMode=1").unwrap() > 0);

        // Second run must be a no-op.
        let (changed_again, _) = rewrite_unreal(&file).unwrap();
        assert!(!changed_again, "rewrite should be idempotent");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_game_is_reported_as_unsupported() {
        let out = prepare("__never_exists__", "test");
        assert_eq!(out.len(), 1);
        assert!(!out[0].detail.is_empty());
        assert!(out[0].config_path.is_none());
    }
}

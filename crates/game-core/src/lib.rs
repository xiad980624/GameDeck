use serde::{Deserialize, Serialize};

/// Background utilities must not allocate a console when called from the GUI app.
/// CREATE_NO_WINDOW affects console allocation, not a launched GUI's main window.
pub fn background_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

/// 统一游戏身份。平台身份和安装实例分别存储在 game_sources / installations。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub original_name: String,
    pub update_status: String,
    pub installed_version: Option<String>,
    pub update_detail: Option<String>,
    pub update_checked_at: Option<String>,
    pub description: String,
    pub genre: String,
    pub platform: String,
    pub external_id: String,
    pub launch_installation_id: Option<String>,
    pub installed: bool,
    pub favorite: bool,
    #[serde(default)]
    pub hidden: bool,
    pub total_minutes: i64,
    pub last_played_at: Option<String>,
    pub added_at: String,
    pub poster: Option<String>,
    pub hero: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
    pub hero_width: Option<i64>,
    pub hero_height: Option<i64>,
    pub hero_source: Option<String>,
    pub landscape: Option<String>,
    pub local_minutes: i64,
    pub playtime_source: String,
    pub playtime_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredGame {
    pub provider: String,
    pub external_id: String,
    pub name: String,
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCandidate {
    pub path: String,
    pub relative_path: String,
    pub size: u64,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualFolderScan {
    pub name: String,
    pub candidates: Vec<ManualCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub found: usize,
    pub inserted: usize,
    pub updated: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LaunchTarget {
    pub game_id: String,
    pub provider: String,
    pub external_id: String,
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub game_id: String,
    pub state: String,
    pub timestamp: String,
    pub message: Option<String>,
}

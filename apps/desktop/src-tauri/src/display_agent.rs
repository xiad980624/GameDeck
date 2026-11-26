//! Lobby lifecycle for the copy-mode setup.
//!
//! DISPLAY1 and the TV are in a Windows "duplicate" relationship: they are one
//! logical desktop showing the same frame. There is no TV hot-plug state to
//! track and no monitor to move the lobby to — the lobby simply appears
//! full-screen after startup and holds focus so a controller can navigate it.
//!
//! Nothing here calls SetDisplayConfig.

use std::{path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const CONFIG_FILE: &str = "display-agent.json";

/// Persisted switches. Only `force_borderless` is still live; keeping the
/// struct small means an old config file with extra keys still deserializes
/// (serde ignores unknown fields by default).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Rewrite adapted UE games to borderless windowed before launch.
    pub force_borderless: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { force_borderless: true }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AgentStatus {
    pub config: AgentConfig,
}

static CONFIG: Mutex<Option<AgentConfig>> = Mutex::new(None);

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join(CONFIG_FILE))
}

fn current_config() -> AgentConfig {
    CONFIG.lock().unwrap().clone().unwrap_or_default()
}

/// Whether adapted games should be rewritten to borderless before launch.
pub fn force_borderless() -> bool {
    current_config().force_borderless
}

/// Show the lobby full-screen and force focus onto it. Called at startup, from
/// the tray, and by the startup fallbacks. A plain set_focus() can lose to the
/// Windows foreground lock, which leaves the controller unable to navigate.
pub fn reveal(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let Some(window) = app.get_webview_window("main") else { return };
        let _ = window.unminimize();
        let _ = window.show();
        crate::lobby_focus::grab_focus(&window);
    });
}

/// Load config and reveal the lobby. Kept as the startup entry point.
pub fn start(app: AppHandle) {
    let config = load_config(&app);
    *CONFIG.lock().unwrap() = Some(config);
    reveal(&app);
}

/// Tray "show lobby".
pub fn show_now(app: &AppHandle) {
    reveal(app);
}

/// Retained for older frontends that pause automatic placement on manual
/// monitor selection. There is no agent loop to pause any more.
#[tauri::command]
pub fn display_agent_pause() {}

fn load_config(app: &AppHandle) -> AgentConfig {
    let Ok(path) = config_path(app) else { return AgentConfig::default() };
    let Ok(text) = std::fs::read_to_string(path) else { return AgentConfig::default() };
    serde_json::from_str(&text).unwrap_or_default()
}

#[tauri::command]
pub fn display_agent_status() -> AgentStatus {
    AgentStatus { config: current_config() }
}

#[tauri::command]
pub fn display_agent_config() -> AgentConfig {
    current_config()
}

#[tauri::command]
pub fn set_display_agent_config(config: AgentConfig) -> Result<(), String> {
    *CONFIG.lock().unwrap() = Some(config);
    Ok(())
}

#[tauri::command]
pub fn save_display_agent_config(app: AppHandle, config: AgentConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    *CONFIG.lock().unwrap() = Some(config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_borderless_defaults_on() {
        assert!(AgentConfig::default().force_borderless);
    }

    #[test]
    fn old_config_with_retired_keys_still_loads() {
        let text = r#"{"enabled":true,"primary_switch_on_game":false,"force_borderless":false,"match_target":{"gdi_name":"display3"},"poll_ms":800}"#;
        let parsed: AgentConfig = serde_json::from_str(text).unwrap();
        assert!(!parsed.force_borderless);
    }
}

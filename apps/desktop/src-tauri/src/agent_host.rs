//! Always-on host plumbing: system tray and Windows startup registration.
//!
//! The agent lives as long as the process. Minimizing/closing the lobby hides
//! the window instead of quitting, and the tray menu is the way back.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

static TRAY_READY: AtomicBool = AtomicBool::new(false);

pub fn tray_ready() -> bool {
    TRAY_READY.load(Ordering::Relaxed)
}

pub fn install_tray(app: &AppHandle) -> Result<(), String> {
    let show = MenuItem::with_id(app, "show", "显示大厅", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let autostart = MenuItem::with_id(app, "autostart", "切换开机自启", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "退出 GameDeck", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let menu = Menu::with_items(app, &[&show, &autostart, &separator, &quit])
        .map_err(|e| e.to_string())?;
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png")).map_err(|e| e.to_string())?;

    TrayIconBuilder::with_id("gamedeck-agent")
        .icon(icon)
        .menu(&menu)
        .tooltip("GameDeck 后台运行中")
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => crate::display_agent::show_now(app),
            "autostart" => {
                match autostart_status() {
                    Ok(true) => {
                        let _ = uninstall_autostart();
                    }
                    Ok(false) => {
                        let _ = install_autostart();
                    }
                    Err(_) => {}
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::display_agent::show_now(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;
    TRAY_READY.store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const RUN_NAME: &str = "GameDeck";

#[cfg(windows)]
fn reg(args: &[&str]) -> Result<std::process::Output, String> {
    std::process::Command::new("reg")
        .args(args)
        .output()
        .map_err(|e| format!("无法运行 reg.exe: {e}"))
}

#[tauri::command]
pub fn autostart_status() -> Result<bool, String> {
    #[cfg(windows)]
    {
        let output = reg(&["query", RUN_KEY, "/v", RUN_NAME])?;
        return Ok(output.status.success());
    }
    #[cfg(not(windows))]
    Err("开机自启仅支持 Windows".into())
}

#[tauri::command]
pub fn install_autostart() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let command = format!("\"{}\"", exe.to_string_lossy());
        let output = reg(&[
            "add", RUN_KEY, "/v", RUN_NAME, "/t", "REG_SZ", "/d", &command, "/f",
        ])?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    }
    #[cfg(not(windows))]
    Err("开机自启仅支持 Windows".into())
}

#[tauri::command]
pub fn uninstall_autostart() -> Result<(), String> {
    #[cfg(windows)]
    {
        let output = reg(&["delete", RUN_KEY, "/v", RUN_NAME, "/f"])?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    }
    #[cfg(not(windows))]
    Err("开机自启仅支持 Windows".into())
}

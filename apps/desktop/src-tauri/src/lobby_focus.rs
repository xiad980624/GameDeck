use tauri::{AppHandle, Manager};

#[cfg(windows)]
pub fn gamepad_wake_target(window: usize) -> Option<usize> {
    if game_launcher::has_active_session() {
        return None;
    }
    foreground::other_display(window)
}

#[cfg(windows)]
pub fn wake_from_gamepad(app: &AppHandle, expected_foreground: usize) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window("main") else { return };
        let Ok(hwnd) = window.hwnd() else { return };
        if gamepad_wake_target(hwnd.0 as usize) != Some(expected_foreground) {
            return;
        }
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        if foreground::activate(hwnd.0 as usize, expected_foreground) {
            let webview: &tauri::Webview = window.as_ref();
            let _ = webview.set_focus();
        }
    });
}

/// Force the lobby window and its WebView into the foreground. Must run on the
/// main thread. Called at the instant the lobby is shown so that controller
/// navigation is live immediately, even when the mouse is out of reach on the
/// TV (1 screen switched off by remote). This targets the window directly,
/// without the cross-display guard used by gamepad wake-up.
pub fn grab_focus(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        if let Ok(hwnd) = window.hwnd() {
            foreground::grab(hwnd.0 as usize);
        }
    }
    let webview: &tauri::Webview = window.as_ref();
    let _ = webview.set_focus();
}

// A minimized/occluded renderer may not run its session handler until restored.
// On game exit we always take focus back: the player is on the TV with a
// controller and cannot reach the mouse if focus lands on an IME popup or a
// launcher window. No "is this our window?" guard here — that guard is exactly
// what let Sogou IME steal focus after a game closed.
pub fn restore(app: &AppHandle, _provider: &str, _took_foreground: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        for delay in [0, 150, 350, 700] {
            if delay != 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            let Some(window) = app.get_webview_window("main") else { return; };
            if delay != 0 && window.is_focused().unwrap_or(false) { return; }
            let app = app.clone();
            let _ = app.run_on_main_thread(move || {
                if window.is_focused().unwrap_or(false) { return; }
                let _ = window.unminimize();
                let _ = window.show();
                grab_focus(&window);
            });
        }
    });
}

#[cfg(windows)]
mod foreground {
    use std::ffi::c_void;
    type Handle = *mut c_void;
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> Handle;
        fn GetWindowThreadProcessId(window: Handle, pid: *mut u32) -> u32;
        fn IsWindowVisible(window: Handle) -> i32;
        fn MonitorFromWindow(window: Handle, flags: u32) -> Handle;
        fn SetForegroundWindow(window: Handle) -> i32;
        fn BringWindowToTop(window: Handle) -> i32;
        fn AttachThreadInput(from: u32, to: u32, attach: i32) -> i32;
        fn IsHungAppWindow(window: Handle) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> u32;
    }

    pub fn other_display(window: usize) -> Option<usize> {
        unsafe {
            let window = window as Handle;
            let active = GetForegroundWindow();
            if active.is_null() || active == window || IsWindowVisible(window) == 0 {
                return None;
            }
            let lobby_screen = MonitorFromWindow(window, 2);
            let active_screen = MonitorFromWindow(active, 2);
            cross_display(lobby_screen as usize, active_screen as usize).then_some(active as usize)
        }
    }

    pub fn activate(window: usize, expected: usize) -> bool {
        unsafe {
            let window = window as Handle;
            let active = GetForegroundWindow();
            if active == window { return true; }
            if active as usize != expected { return false; }
            SetForegroundWindow(window);
            if GetForegroundWindow() == window { return true; }
            if GetForegroundWindow() != active || IsHungAppWindow(active) != 0 { return false; }

            // XInput does not grant foreground activation permission through a keyboard queue.
            // Join the current desktop's input queue only for the duration of this explicit handoff.
            let owner = GetWindowThreadProcessId(active, std::ptr::null_mut());
            let current = GetCurrentThreadId();
            if owner == 0 || owner == current || AttachThreadInput(current, owner, 1) == 0 {
                return false;
            }
            if GetForegroundWindow() == active {
                BringWindowToTop(window);
                SetForegroundWindow(window);
            }
            AttachThreadInput(current, owner, 0);
            GetForegroundWindow() == window
        }
    }

    /// Unconditionally force `window` to the foreground, with no expected-window
    /// guard. Used at the exact moment the lobby is shown: a controller has no
    /// keyboard queue to grant foreground permission, so we briefly join the
    /// active window's input queue (same technique as `activate`).
    pub fn grab(window: usize) -> bool {
        unsafe {
            let window = window as Handle;
            let active = GetForegroundWindow();
            if active == window { return true; }
            SetForegroundWindow(window);
            if GetForegroundWindow() == window { return true; }
            if active.is_null() || IsHungAppWindow(active) != 0 { return false; }
            let owner = GetWindowThreadProcessId(active, std::ptr::null_mut());
            let current = GetCurrentThreadId();
            if owner == 0 || owner == current || AttachThreadInput(current, owner, 1) == 0 {
                return false;
            }
            if GetForegroundWindow() == active {
                BringWindowToTop(window);
                SetForegroundWindow(window);
            }
            AttachThreadInput(current, owner, 0);
            GetForegroundWindow() == window
        }
    }

    pub(super) fn cross_display(lobby: usize, active: usize) -> bool {
        lobby != 0 && active != 0 && lobby != active
    }

    pub fn pid() -> u32 {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(GetForegroundWindow(), &mut pid) };
        pid
    }
}

#[cfg(windows)]
pub use foreground::pid as foreground_pid;
#[cfg(not(windows))]
pub fn foreground_pid() -> u32 { 0 }

#[cfg(all(test,windows))]
mod tests {
    use super::foreground::cross_display;

    #[test]
    fn gamepad_takeover_requires_two_known_different_displays() {
        assert!(cross_display(1, 2));
        assert!(!cross_display(1, 1));
        assert!(!cross_display(1, 0));
        assert!(!cross_display(0, 2));
    }
}

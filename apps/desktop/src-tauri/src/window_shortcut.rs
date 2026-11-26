use tauri::{AppHandle, Emitter, PhysicalPosition, PhysicalSize, WebviewWindow};

#[tauri::command]
pub fn toggle_lobby_fullscreen(window: WebviewWindow) -> Result<(), String> {
    let fullscreen = window.is_fullscreen().map_err(|error| error.to_string())?;
    let screen = if fullscreen { window.current_monitor().ok().flatten() } else { None };
    window.set_fullscreen(!fullscreen).map_err(|error| error.to_string())?;
    if let Some(screen) = screen {
        // Exiting fullscreen can restore the old window bounds on a different display.
        if window.current_monitor().ok().flatten().is_some_and(|current| current.position() != screen.position()) {
            let maximized = window.is_maximized().unwrap_or(false);
            if maximized { let _ = window.unmaximize(); }
            if let Ok(size) = window.outer_size() {
                let work = screen.work_area();
                let width = size.width.min(work.size.width);
                let height = size.height.min(work.size.height);
                let _ = window.set_size(PhysicalSize::new(width, height));
                let _ = window.set_position(PhysicalPosition::new(
                    work.position.x + ((work.size.width - width) / 2) as i32,
                    work.position.y + ((work.size.height - height) / 2) as i32,
                ));
            }
            if maximized { let _ = window.maximize(); }
        }
    }
    let _ = window.emit("lobby-window-mode-changed", ());
    Ok(())
}

#[cfg(windows)]
pub fn install(app: &AppHandle) {
    use tauri::Manager;
    use webview2_com::{
        AcceleratorKeyPressedEventHandler,
        Microsoft::Web::WebView2::Win32::{
            COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
            COREWEBVIEW2_PHYSICAL_KEY_STATUS,
        },
    };

    #[link(name = "user32")]
    extern "system" {
        fn GetKeyState(key: i32) -> i16;
    }

    let Some(window) = app.get_webview_window("main") else { return };
    let app = app.clone();
    let _ = window.with_webview(move |webview| {
        let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut key = 0;
            let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
            let mut status = COREWEBVIEW2_PHYSICAL_KEY_STATUS::default();
            unsafe {
                args.VirtualKey(&mut key)?;
                if key != 0x7a || GetKeyState(0x11) < 0 || GetKeyState(0x12) < 0
                    || GetKeyState(0x5b) < 0 || GetKeyState(0x5c) < 0
                {
                    return Ok(());
                }
                args.KeyEventKind(&mut kind)?;
                args.PhysicalKeyStatus(&mut status)?;
                args.SetHandled(true)?;
            }
            if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && !status.WasKeyDown.as_bool() {
                // Leave the synchronous WebView2 callback before calling window APIs.
                let handle = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let main = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        let Some(window) = main.get_webview_window("main") else { return };
                        let _ = toggle_lobby_fullscreen(window);
                    });
                });
            }
            Ok(())
        }));
        let mut token = 0;
        // WebView2 owns the registration for the lifetime of its controller.
        let _ = unsafe { webview.controller().add_AcceleratorKeyPressed(&handler, &mut token) };
    });
}

#[cfg(not(windows))]
pub fn install(_: &AppHandle) {}

// Original short PCM cues, embedded so Windows controller input needs no browser gesture.
#[tauri::command]
pub fn play_ui_sound(sound: Option<String>) -> Result<bool, String> {
    let data: Option<&'static [u8]> = match sound.as_deref() {
        Some("select") => Some(include_bytes!("../../public/audio/select.wav")),
        Some("launch") => Some(include_bytes!("../../public/audio/launch.wav")),
        None => None,
        _ => return Err("Unknown UI sound".into()),
    };
    Ok(play(data))
}

#[cfg(windows)]
fn play(data: Option<&'static [u8]>) -> bool {
    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundW(sound: *const u16, module: *mut std::ffi::c_void, flags: u32) -> i32;
    }
    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;
    let pointer = data.map_or(std::ptr::null(), |bytes| bytes.as_ptr().cast());
    let flags = if data.is_some() { SND_ASYNC | SND_NODEFAULT | SND_MEMORY } else { 0 };
    // Static buffers outlive asynchronous playback. A new cue replaces the old one.
    unsafe { PlaySoundW(pointer, std::ptr::null_mut(), flags); }
    true
}

#[cfg(not(windows))]
fn play(_data: Option<&'static [u8]>) -> bool {
    false // Use the browser audio adapter in non-Windows previews.
}

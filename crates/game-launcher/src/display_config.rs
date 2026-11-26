//! Read-only Windows display topology probe.
//!
//! Enumerates the *active* display paths with QueryDisplayConfig and resolves
//! each path's GDI name, EDID identity and monitor device path with
//! DisplayConfigGetDeviceInfo. Nothing here changes display topology.
//!
//! The monitor device path is the stable identity across hotplug: Windows keeps
//! it while the TV is switched off and on, unlike monitor indices or positions.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayPathInfo {
    /// GDI source name, e.g. `\\.\DISPLAY3`.
    pub gdi_name: String,
    /// Human readable monitor name from EDID, e.g. `TCL 55Q9L Pro`.
    pub friendly_name: String,
    /// SetupAPI device interface path of the monitor (stable across hotplug).
    pub monitor_device_path: String,
    /// SetupAPI device interface path of the graphics adapter.
    pub adapter_path: String,
    /// Three-letter PNP manufacturer id decoded from EDID, e.g. `TCL`.
    pub edid_manufacturer: String,
    pub edid_manufacturer_id: u16,
    pub edid_product_code: u16,
    /// HDMI / DisplayPort / etc.
    pub output_technology: String,
    /// Desktop position of the source (may be negative for displays left of primary).
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
    /// Refresh rate = numerator / denominator.
    pub refresh_numerator: u32,
    pub refresh_denominator: u32,
    pub connector_instance: u32,
    pub target_available: bool,
}

#[cfg(not(windows))]
pub fn active_paths() -> Result<Vec<DisplayPathInfo>, String> {
    Err("Display topology probe is only available on Windows".into())
}

#[cfg(not(windows))]
#[derive(Clone, Debug, Default)]
pub struct LayoutSnapshot;

#[cfg(not(windows))]
pub fn snapshot_layout() -> Result<LayoutSnapshot, String> {
    Err("Display topology probe is only available on Windows".into())
}
#[cfg(not(windows))]
pub fn restore_layout(_: &LayoutSnapshot) -> Result<(), String> {
    Err("Display topology probe is only available on Windows".into())
}
#[cfg(not(windows))]
pub fn set_primary(_: &str) -> Result<bool, String> {
    Err("Display topology probe is only available on Windows".into())
}
#[cfg(not(windows))]
pub fn source_origins() -> Result<Vec<(String, i32, i32)>, String> {
    Err("Display topology probe is only available on Windows".into())
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::c_void;

    const QDC_ONLY_ACTIVE_PATHS: u32 = 0x0000_0002;
    const DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME: u32 = 1;
    const DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME: u32 = 2;
    const DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME: u32 = 4;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
    const CCHDEVICENAME: usize = 32;

    // SetDisplayConfig flags. We deliberately do NOT pass SDC_SAVE_TO_DATABASE:
    // the temporary primary switch must not overwrite the user's persisted
    // layout, so a crash or reboot naturally falls back to their own config.
    const SDC_USE_SUPPLIED_DISPLAY_CONFIG: u32 = 0x0000_0020;
    const SDC_APPLY: u32 = 0x0000_0080;
    const SDC_ALLOW_CHANGES: u32 = 0x0000_0400;

    const MODE_INFO_TYPE_SOURCE: u32 = 1;

    #[repr(C)]
    #[derive(Clone, Copy, Default, PartialEq, Eq)]
    struct Luid {
        low_part: u32,
        high_part: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct PathSourceInfo {
        adapter_id: Luid,
        id: u32,
        mode_info_idx: u32,
        status_flags: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rational {
        numerator: u32,
        denominator: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct PathTargetInfo {
        adapter_id: Luid,
        id: u32,
        mode_info_idx: u32,
        output_technology: u32,
        rotation: u32,
        scaling: u32,
        refresh_rate: Rational,
        scan_line_ordering: u32,
        target_available: i32,
        status_flags: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct PathInfo {
        source_info: PathSourceInfo,
        target_info: PathTargetInfo,
        flags: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ModeInfo {
        info_type: u32,
        id: u32,
        adapter_id: Luid,
        payload: [u8; 48],
    }
    impl Default for ModeInfo {
        fn default() -> Self {
            Self { info_type: 0, id: 0, adapter_id: Luid::default(), payload: [0; 48] }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct SourceMode {
        width: u32,
        height: u32,
        pixel_format: u32,
        position_x: i32,
        position_y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct DeviceInfoHeader {
        info_type: u32,
        size: u32,
        adapter_id: Luid,
        id: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SourceDeviceName {
        header: DeviceInfoHeader,
        view_gdi_device_name: [u16; CCHDEVICENAME],
    }
    impl Default for SourceDeviceName {
        fn default() -> Self {
            Self { header: DeviceInfoHeader::default(), view_gdi_device_name: [0; CCHDEVICENAME] }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct TargetDeviceName {
        header: DeviceInfoHeader,
        flags: u32,
        output_technology: u32,
        edid_manufacture_id: u16,
        edid_product_code_id: u16,
        connector_instance: u32,
        monitor_friendly_device_name: [u16; 64],
        monitor_device_path: [u16; 128],
    }
    impl Default for TargetDeviceName {
        fn default() -> Self {
            Self {
                header: DeviceInfoHeader::default(),
                flags: 0,
                output_technology: 0,
                edid_manufacture_id: 0,
                edid_product_code_id: 0,
                connector_instance: 0,
                monitor_friendly_device_name: [0; 64],
                monitor_device_path: [0; 128],
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AdapterName {
        header: DeviceInfoHeader,
        adapter_device_path: [u16; 128],
    }
    impl Default for AdapterName {
        fn default() -> Self {
            Self { header: DeviceInfoHeader::default(), adapter_device_path: [0; 128] }
        }
    }

    #[link(name = "user32")]
    extern "system" {
        fn QueryDisplayConfig(
            flags: u32,
            path_count: *mut u32,
            paths: *mut PathInfo,
            mode_count: *mut u32,
            modes: *mut ModeInfo,
            topology_id: *mut u32,
        ) -> i32;
        fn DisplayConfigGetDeviceInfo(request: *mut c_void) -> i32;
        fn SetDisplayConfig(
            path_count: u32,
            paths: *const PathInfo,
            mode_count: u32,
            modes: *const ModeInfo,
            flags: u32,
        ) -> i32;
    }

    fn wide(value: &[u16]) -> String {
        let end = value.iter().position(|&c| c == 0).unwrap_or(value.len());
        String::from_utf16_lossy(&value[..end])
    }

    fn technology(value: u32) -> String {
        let name = match value {
            0 => "Other",
            1 => "HD15",
            2 => "SVIDEO",
            3 => "Composite",
            4 => "Component",
            5 => "DVI",
            6 => "HDMI",
            7 => "LVDS",
            8 => "D_JPN",
            9 => "SDI",
            10 => "DisplayPort External",
            11 => "DisplayPort Embedded",
            12 => "UDI External",
            13 => "UDI Embedded",
            14 => "SDTV Dongle",
            15 => "Miracast",
            16 => "Indirect",
            _ => "Unknown",
        };
        name.to_string()
    }

    fn source_mode(mode: &ModeInfo) -> Option<SourceMode> {
        if mode.info_type != MODE_INFO_TYPE_SOURCE { return None; }
        Some(unsafe { std::ptr::read_unaligned(mode.payload.as_ptr() as *const SourceMode) })
    }

    fn write_source_position(mode: &mut ModeInfo, x: i32, y: i32) {
        if mode.info_type != MODE_INFO_TYPE_SOURCE { return; }
        let mut source = unsafe { std::ptr::read_unaligned(mode.payload.as_ptr() as *const SourceMode) };
        source.position_x = x;
        source.position_y = y;
        unsafe { std::ptr::write_unaligned(mode.payload.as_mut_ptr() as *mut SourceMode, source) };
    }

    /// A saved desktop layout: the desktop origin of each active source,
    /// keyed by its stable (adapter LUID, source id) pair. Restoring writes
    /// these positions back verbatim so the user's arrangement is reproduced.
    #[derive(Clone, Debug, Default)]
    pub struct LayoutSnapshot {
        pub entries: Vec<(u32, i32, u32, i32, i32)>, // adapter_low, adapter_high, source_id, x, y
        pub primary_gdi: String,
    }

    /// Capture the current desktop layout before any switch.
    pub fn snapshot_layout() -> Result<LayoutSnapshot, String> {
        let (paths, modes) = query_raw()?;
        let mut snapshot = LayoutSnapshot::default();
        for path in &paths {
            let mode = modes.iter()
                .find(|m| m.info_type == MODE_INFO_TYPE_SOURCE
                    && m.adapter_id == path.source_info.adapter_id && m.id == path.source_info.id)
                .and_then(source_mode)
                .unwrap_or_default();
            let name = gdi_name(path);
            if mode.position_x == 0 && mode.position_y == 0 {
                snapshot.primary_gdi = name.clone();
            }
            snapshot.entries.push((
                path.source_info.adapter_id.low_part as u32,
                path.source_info.adapter_id.high_part,
                path.source_info.id,
                mode.position_x,
                mode.position_y,
            ));
        }
        Ok(snapshot)
    }

    /// Apply a layout snapshot, restoring the exact desktop origins recorded.
    pub fn restore_layout(snapshot: &LayoutSnapshot) -> Result<(), String> {
        unsafe {
            let (paths, mut modes) = query_raw()?;
            let mut applied = false;
            for path in &paths {
                let key = (
                    path.source_info.adapter_id.low_part as u32,
                    path.source_info.adapter_id.high_part,
                    path.source_info.id,
                );
                if let Some(entry) = snapshot.entries.iter()
                    .find(|e| (e.0, e.1, e.2) == key)
                {
                    if let Some(mode) = modes.iter_mut()
                        .find(|m| m.info_type == MODE_INFO_TYPE_SOURCE
                            && m.adapter_id == path.source_info.adapter_id && m.id == path.source_info.id)
                    {
                        write_source_position(mode, entry.3, entry.4);
                        applied = true;
                    }
                }
            }
            if !applied { return Err("snapshot matched no active source".into()); }
            let status = SetDisplayConfig(
                paths.len() as u32,
                paths.as_ptr(),
                modes.len() as u32,
                modes.as_ptr(),
                SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_ALLOW_CHANGES,
            );
            if status != ERROR_SUCCESS {
                return Err(format!("SetDisplayConfig(restore) failed: {status}"));
            }
            Ok(())
        }
    }

    /// Make the display whose GDI name matches `target_gdi` the primary
    /// monitor by moving its desktop origin to (0,0) and translating every
    /// other source by the same offset (topology and sizes stay untouched).
    /// Returns true if a matching active source was found and applied.
    pub fn set_primary(target_gdi: &str) -> Result<bool, String> {
        unsafe {
            let (paths, mut modes) = query_raw()?;
            let target_path = paths.iter()
                .find(|p| gdi_name(p).eq_ignore_ascii_case(target_gdi));
            let Some(target) = target_path else { return Ok(false) };
            let target_mode = modes.iter()
                .find(|m| m.info_type == MODE_INFO_TYPE_SOURCE
                    && m.adapter_id == target.source_info.adapter_id && m.id == target.source_info.id)
                .and_then(source_mode)
                .ok_or_else(|| "target source mode not found".to_string())?;
            let dx = -target_mode.position_x;
            let dy = -target_mode.position_y;

            for mode in modes.iter_mut() {
                if mode.info_type == MODE_INFO_TYPE_SOURCE {
                    if let Some(source) = source_mode(mode) {
                        write_source_position(mode, source.position_x + dx, source.position_y + dy);
                    }
                }
            }
            let status = SetDisplayConfig(
                paths.len() as u32,
                paths.as_ptr(),
                modes.len() as u32,
                modes.as_ptr(),
                SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_ALLOW_CHANGES,
            );
            if status != ERROR_SUCCESS {
                return Err(format!("SetDisplayConfig(primary) failed: {status}"));
            }
            Ok(true)
        }
    }

    #[cfg(test)]
    mod layout_tests {
        use super::*;
        #[test]
        fn win32_struct_sizes_match_sdk() {
            // Guards against layout drift when the FFI structs are edited.
            assert_eq!(std::mem::size_of::<PathInfo>(), 72);
            assert_eq!(std::mem::size_of::<ModeInfo>(), 64);
            assert_eq!(std::mem::size_of::<TargetDeviceName>(), 420);
            assert_eq!(std::mem::size_of::<SourceDeviceName>(), 84);
        }
    }

    /// Query the raw active paths and mode infos. Shared by the read-only
    /// probe and by set_primary, which needs to rewrite source positions.
    fn query_raw() -> Result<(Vec<PathInfo>, Vec<ModeInfo>), String> {
        unsafe {
            // QueryDisplayConfig's two-phase NULL probe returns ERROR_INVALID_PARAMETER
            // on some Windows builds/sessions, so start with generous preallocated
            // buffers and only grow if the OS reports they are too small.
            let mut capacity = 16usize;
            let (mut paths, mut modes, path_count, mode_count) = loop {
                let mut pc = capacity as u32;
                let mut mc = (capacity * 2) as u32;
                let mut paths = vec![PathInfo::default(); capacity];
                let mut modes = vec![ModeInfo::default(); capacity * 2];
                let status = QueryDisplayConfig(
                    QDC_ONLY_ACTIVE_PATHS,
                    &mut pc,
                    paths.as_mut_ptr(),
                    &mut mc,
                    modes.as_mut_ptr(),
                    std::ptr::null_mut(),
                );
                if status == ERROR_SUCCESS {
                    break (paths, modes, pc, mc);
                }
                if status == ERROR_INSUFFICIENT_BUFFER && capacity < 256 {
                    capacity *= 2;
                    continue;
                }
                return Err(format!("QueryDisplayConfig failed: {status} (capacity {capacity})"));
            };
            paths.truncate(path_count as usize);
            modes.truncate(mode_count as usize);
            Ok((paths, modes))
        }
    }

    /// Resolve a path's GDI source name (e.g. `\\.\DISPLAY3`), best effort.
    fn gdi_name(path: &PathInfo) -> String {
        let mut source_name = SourceDeviceName::default();
        source_name.header.info_type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        source_name.header.size = std::mem::size_of::<SourceDeviceName>() as u32;
        source_name.header.adapter_id = path.source_info.adapter_id;
        source_name.header.id = path.source_info.id;
        if unsafe { DisplayConfigGetDeviceInfo(&mut source_name as *mut _ as *mut c_void) } == ERROR_SUCCESS {
            wide(&source_name.view_gdi_device_name)
        } else {
            String::new()
        }
    }

    /// Print current source origins; handy for verifying a switch took effect.
    pub fn source_origins() -> Result<Vec<(String, i32, i32)>, String> {
        let (paths, modes) = query_raw()?;
        Ok(active_origins(&paths, &modes))
    }

    fn active_origins(paths: &[PathInfo], modes: &[ModeInfo]) -> Vec<(String, i32, i32)> {
        paths.iter().map(|path| {
            let mode = modes.iter()
                .find(|m| m.info_type == MODE_INFO_TYPE_SOURCE
                    && m.adapter_id == path.source_info.adapter_id && m.id == path.source_info.id)
                .and_then(source_mode)
                .unwrap_or_default();
            (gdi_name(path), mode.position_x, mode.position_y)
        }).collect()
    }

    pub fn active_paths() -> Result<Vec<DisplayPathInfo>, String> {
        unsafe {
            let (paths, modes) = query_raw()?;
            if paths.is_empty() { return Ok(Vec::new()); }

            let mut result = Vec::with_capacity(paths.len());
            for path in &paths {
                let source = path.source_info;
                let target = path.target_info;

                let source_mode = modes.iter()
                    .find(|m| m.info_type == MODE_INFO_TYPE_SOURCE
                        && m.adapter_id == source.adapter_id && m.id == source.id)
                    .and_then(source_mode)
                    .unwrap_or_default();

                let mut source_name = SourceDeviceName::default();
                source_name.header.info_type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                source_name.header.size = std::mem::size_of::<SourceDeviceName>() as u32;
                source_name.header.adapter_id = source.adapter_id;
                source_name.header.id = source.id;

                let mut target_name = TargetDeviceName::default();
                target_name.header.info_type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                target_name.header.size = std::mem::size_of::<TargetDeviceName>() as u32;
                target_name.header.adapter_id = target.adapter_id;
                target_name.header.id = target.id;

                let mut adapter_name = AdapterName::default();
                adapter_name.header.info_type = DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME;
                adapter_name.header.size = std::mem::size_of::<AdapterName>() as u32;
                adapter_name.header.adapter_id = target.adapter_id;
                adapter_name.header.id = target.id;

                // Name lookups are best effort: a headless or disconnected target
                // may refuse them even though the source path is still active.
                let gdi = if DisplayConfigGetDeviceInfo(&mut source_name as *mut _ as *mut c_void) == ERROR_SUCCESS {
                    wide(&source_name.view_gdi_device_name)
                } else { String::new() };
                let adapter = if DisplayConfigGetDeviceInfo(&mut adapter_name as *mut _ as *mut c_void) == ERROR_SUCCESS {
                    wide(&adapter_name.adapter_device_path)
                } else { String::new() };

                let (friendly, monitor_path, manufacturer_id, product_code, connector, tech) =
                    if DisplayConfigGetDeviceInfo(&mut target_name as *mut _ as *mut c_void) == ERROR_SUCCESS {
                        (
                            wide(&target_name.monitor_friendly_device_name),
                            wide(&target_name.monitor_device_path),
                            target_name.edid_manufacture_id,
                            target_name.edid_product_code_id,
                            target_name.connector_instance,
                            technology(target_name.output_technology),
                        )
                    } else {
                        (String::new(), String::new(), 0, 0, 0, technology(target.output_technology))
                    };

                result.push(DisplayPathInfo {
                    gdi_name: gdi,
                    friendly_name: friendly,
                    monitor_device_path: monitor_path,
                    adapter_path: adapter,
                    // Windows reports this field byte-swapped relative to the
                    // PNP value embedded in the EDID and in monitorDevicePath
                    // (e.g. 0x6C50 here vs 0x506C = "TCL" in the path).
                    edid_manufacturer: decode_edid_manufacturer(manufacturer_id.swap_bytes()),
                    edid_manufacturer_id: manufacturer_id,
                    edid_product_code: product_code,
                    output_technology: tech,
                    position_x: source_mode.position_x,
                    position_y: source_mode.position_y,
                    width: source_mode.width,
                    height: source_mode.height,
                    refresh_numerator: target.refresh_rate.numerator,
                    refresh_denominator: target.refresh_rate.denominator,
                    connector_instance: connector,
                    target_available: target.target_available != 0,
                });
            }
            Ok(result)
        }
    }
}

#[cfg(windows)]
pub use native::{
    active_paths, restore_layout, set_primary, snapshot_layout, source_origins, LayoutSnapshot,
};

/// Decode a 16-bit EDID PNP id into three uppercase letters.
/// Bits 14..10, 9..5 and 4..0 hold letters 1..26 (A..Z).
pub fn decode_edid_manufacturer(id: u16) -> String {
    let mut out = String::with_capacity(3);
    for shift in [10, 5, 0] {
        let code = (id >> shift) & 0x1f;
        out.push(if (1..=26).contains(&code) { (b'A' + code as u8 - 1) as char } else { '?' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::decode_edid_manufacturer;

    #[test]
    fn decodes_edid_pnp_identifier() {
        // TCL: T=20, C=3, L=12 -> 0x506C
        let tcl = (20 << 10) | (3 << 5) | 12;
        assert_eq!(tcl, 0x506C);
        assert_eq!(decode_edid_manufacturer(tcl), "TCL");
        assert_eq!(decode_edid_manufacturer(0), "???");
    }

}

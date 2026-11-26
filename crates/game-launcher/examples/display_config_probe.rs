//! Read-only probe of active Windows display paths.
//!
//! Usage:
//!   cargo run -p game-launcher --example display_config_probe
//!   cargo run -p game-launcher --example display_config_probe -- --watch
//!
//! `--watch` re-queries every 2 seconds and prints only the set of active GDI
//! names when it changes, so a TV power cycle can be observed without noise.

use game_launcher::display_config::{active_paths, DisplayPathInfo};
use std::collections::BTreeSet;
use std::time::Duration;

fn refresh() -> Result<(Vec<DisplayPathInfo>, String), String> {
    let paths = active_paths()?;
    let stamp = chrono_like_timestamp();
    Ok((paths, stamp))
}

fn chrono_like_timestamp() -> String {
    // std-only UTC timestamp; enough to order watch output.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn print_paths(paths: &[DisplayPathInfo]) {
    println!("active display paths: {}\n", paths.len());
    for (index, path) in paths.iter().enumerate() {
        let refresh = if path.refresh_denominator == 0 {
            "?".to_string()
        } else {
            format!("{:.2}", path.refresh_numerator as f64 / path.refresh_denominator as f64)
        };
        println!("[{}] {}", index, path.gdi_name);
        println!("    friendly      : {}", path.friendly_name);
        println!("    EDID          : {} (raw 0x{:04X}, product 0x{:04X})",
            path.edid_manufacturer, path.edid_manufacturer_id, path.edid_product_code);
        println!("    connector     : {} #{}", path.output_technology, path.connector_instance);
        println!("    desktop       : {},{} {}x{} @ {} Hz",
            path.position_x, path.position_y, path.width, path.height, refresh);
        println!("    monitor path  : {}", path.monitor_device_path);
        println!("    adapter path  : {}", path.adapter_path);
        println!("    available     : {}", path.target_available);
        println!();
    }
}

fn active_set(paths: &[DisplayPathInfo]) -> BTreeSet<String> {
    paths.iter().map(|p| p.gdi_name.clone()).collect()
}

fn main() {
    let watch = std::env::args().any(|arg| arg == "--watch");

    if !watch {
        match refresh() {
            Ok((paths, _)) => print_paths(&paths),
            Err(error) => {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    println!("Watching active display paths every 2s (Ctrl+C to stop).\n");
    let mut previous: Option<BTreeSet<String>> = None;
    loop {
        match refresh() {
            Ok((paths, stamp)) => {
                let current = active_set(&paths);
                match &previous {
                    None => {
                        println!("{stamp} initial: {:?}", current);
                        print_paths(&paths);
                    }
                    Some(old) if old != &current => {
                        let removed: Vec<_> = old.difference(&current).collect();
                        let added: Vec<_> = current.difference(old).collect();
                        println!("{stamp} CHANGED");
                        println!("    removed: {removed:?}");
                        println!("    added  : {added:?}");
                        println!("    active : {current:?}");
                        print_paths(&paths);
                    }
                    _ => {}
                }
                previous = Some(current);
            }
            Err(error) => eprintln!("watch query failed: {error}"),
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

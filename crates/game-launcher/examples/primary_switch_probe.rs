//! Manually verify the temporary primary-monitor switch before wiring it into
//! game launches. Does not touch GameDeck's windows.
//!
//!   cargo run -p game-launcher --example primary_switch_probe --offline -- status
//!   cargo run -p game-launcher --example primary_switch_probe --offline -- snapshot
//!   cargo run -p game-launcher --example primary_switch_probe --offline -- to \\.\DISPLAY3
//!   cargo run -p game-launcher --example primary_switch_probe --offline -- restore <snapshot-file>
//!   cargo run -p game-launcher --example primary_switch_probe --offline -- roundtrip \\.\DISPLAY3

use game_launcher::display_config as dc;
use std::path::PathBuf;

fn print_origins(label: &str) {
    match dc::source_origins() {
        Ok(origins) => {
            println!("[{label}]");
            for (gdi, x, y) in origins {
                let tag = if x == 0 && y == 0 { " <- PRIMARY" } else { "" };
                println!("  {gdi:<18} origin=({x},{y}){tag}");
            }
        }
        Err(error) => println!("[{label}] error: {error}"),
    }
}

fn snapshot_file() -> PathBuf {
    std::env::temp_dir().join("gamedeck-layout-snapshot.txt")
}

// The snapshot is opaque binary-ish data; persist the raw tuple list as text.
fn save_snapshot(snapshot: &dc::LayoutSnapshot, path: &PathBuf) -> Result<(), String> {
    let mut lines = vec![snapshot.primary_gdi.clone()];
    for (al, ah, id, x, y) in &snapshot.entries {
        lines.push(format!("{al}|{ah}|{id}|{x}|{y}"));
    }
    std::fs::write(path, lines.join("\n")).map_err(|e| e.to_string())
}

fn load_snapshot(path: &PathBuf) -> Result<dc::LayoutSnapshot, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    let primary_gdi = lines.next().unwrap_or("").to_string();
    let mut entries = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 5 { continue; }
        let parse = |s: &str| s.parse::<i32>().map_err(|e| e.to_string());
        entries.push((
            parse(parts[0])? as u32,
            parse(parts[1])?,
            parse(parts[2])? as u32,
            parse(parts[3])?,
            parse(parts[4])?,
        ));
    }
    Ok(dc::LayoutSnapshot { entries, primary_gdi })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("status");
    match command {
        "status" => print_origins("current"),
        "snapshot" => {
            match dc::snapshot_layout() {
                Ok(snapshot) => {
                    let path = snapshot_file();
                    if let Err(error) = save_snapshot(&snapshot, &path) {
                        println!("save failed: {error}");
                        return;
                    }
                    println!("saved layout to {}", path.display());
                    println!("original primary: {}", snapshot.primary_gdi);
                    print_origins("captured");
                }
                Err(error) => println!("snapshot failed: {error}"),
            }
        }
        "to" => {
            let Some(target) = args.get(1) else { println!("usage: to <GDI>"); return };
            let path = snapshot_file();
            match dc::snapshot_layout() {
                Ok(snapshot) => {
                    let _ = save_snapshot(&snapshot, &path);
                    println!("saved original layout to {}", path.display());
                }
                Err(error) => println!("warning: snapshot failed: {error}"),
            }
            match dc::set_primary(target) {
                Ok(true) => println!("switched primary to {target}"),
                Ok(false) => println!("no active source matched {target}"),
                Err(error) => println!("switch failed: {error}"),
            }
            print_origins("after switch");
        }
        "restore" => {
            let path = args.get(1).map(PathBuf::from).unwrap_or_else(snapshot_file);
            match load_snapshot(&path).and_then(|s| dc::restore_layout(&s)) {
                Ok(()) => println!("restored layout from {}", path.display()),
                Err(error) => println!("restore failed: {error}"),
            }
            print_origins("after restore");
        }
        "roundtrip" => {
            let Some(target) = args.get(1) else { println!("usage: roundtrip <GDI>"); return };
            print_origins("before");
            let snapshot = match dc::snapshot_layout() {
                Ok(s) => s,
                Err(e) => { println!("snapshot failed: {e}"); return; }
            };
            match dc::set_primary(target) {
                Ok(true) => println!("switched to {target}"),
                Ok(false) => { println!("no match for {target}"); return; }
                Err(e) => { println!("switch failed: {e}"); return; }
            }
            print_origins("after switch");
            std::thread::sleep(std::time::Duration::from_secs(2));
            match dc::restore_layout(&snapshot) {
                Ok(()) => println!("restored"),
                Err(e) => println!("restore failed: {e}"),
            }
            print_origins("after restore");
        }
        _ => println!("commands: status | snapshot | to <GDI> | restore [file] | roundtrip <GDI>"),
    }
}

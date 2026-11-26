//! Discover launch candidates without running programs or following junctions.
use game_core::{ManualCandidate, ManualFolderScan};
use std::{fs, path::Path};

fn utility(name: &str) -> bool {
    let name = name.to_lowercase();
    ["unins", "uninstall", "crashreport", "unitycrash", "vcredist", "vc_redist", "dxsetup", "ueprereq", "dotnet", "setup", "installer"].iter().any(|s|name.starts_with(s))
        || ["notification_helper.exe", "cefprocess.exe", "unitycrashhandler64.exe"].contains(&name.as_str())
}

pub fn scan_folder(folder: &Path) -> Result<ManualFolderScan, String> {
    let root = folder.canonicalize().map_err(|e|format!("无法读取游戏文件夹：{e}"))?;
    if !root.is_dir() || root.parent().is_none() { return Err("请选择某一款游戏的文件夹，而不是整个磁盘".into()); }
    let name = root.file_name().and_then(|s|s.to_str()).unwrap_or("本地游戏").to_string();
    let mut result = ManualFolderScan { name, candidates:vec![], warnings:vec![] };
    let mut stack = vec![(root.clone(),0usize)];
    let mut visited = 0usize;
    let mut skipped = 0usize;
    while let Some((directory,depth)) = stack.pop() {
        let entries = match fs::read_dir(&directory) { Ok(entries)=>entries, Err(_)=>{skipped+=1;continue;} };
        for entry in entries {
            visited+=1;
            if visited>20_000 { return Err("文件夹过大，请选择具体游戏子文件夹，或直接选择 EXE".into()); }
            let entry = match entry { Ok(entry)=>entry, Err(_)=>{skipped+=1;continue;} };
            let meta = match fs::symlink_metadata(entry.path()) { Ok(meta)=>meta, Err(_)=>{skipped+=1;continue;} };
            if meta.file_type().is_symlink() { continue; }
            #[cfg(windows)]
            { use std::os::windows::fs::MetadataExt; if meta.file_attributes() & 0x400 != 0 { continue; } }
            let path=entry.path();
            let filename=entry.file_name().to_string_lossy().to_string();
            if meta.is_dir() {
                if [".git","node_modules","_commonredist","redist","directx","windows","system32"].contains(&filename.to_lowercase().as_str()) {continue;}
                if depth<6 {stack.push((path,depth+1));} else {skipped+=1;}
            } else if meta.is_file() && path.extension().is_some_and(|s|s.eq_ignore_ascii_case("exe")) && !utility(&filename) {
                if result.candidates.len()>=200 {return Err("启动程序候选过多，请选择具体游戏子文件夹，或直接选择 EXE".into());}
                let lower=filename.to_lowercase();
                let mut score=if depth==0 {30}else{0};
                if lower.contains(&result.name.to_lowercase()) {score+=100;}
                if lower.contains("shipping") {score+=40;}
                if lower.contains("launcher") {score+=20;}
                result.candidates.push(ManualCandidate {relative_path:path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().into(),path:path.to_string_lossy().into(),size:meta.len(),score});
            }
        }
    }
    result.candidates.sort_by(|a,b|b.score.cmp(&a.score).then(a.relative_path.cmp(&b.relative_path)));
    if skipped>0 {result.warnings.push("部分目录无法读取或超过扫描深度；如未找到正确程序，请直接选择 EXE。".into());}
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_nested_games_and_ignores_installers() {
        let root=std::env::temp_dir().join(format!("gamedeck-folder-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("Binaries/Win64")).unwrap();
        fs::write(root.join("Binaries/Win64/Adventure-Win64-Shipping.EXE"),b"game").unwrap();
        fs::write(root.join("unins000.exe"),b"uninstall").unwrap();
        fs::write(root.join("CrashReportClient.exe"),b"report").unwrap();
        fs::write(root.join("Launcher.exe"),b"launcher").unwrap();
        let scan=scan_folder(&root).unwrap();
        assert_eq!(scan.candidates.len(),2);
        assert!(scan.candidates.iter().any(|c|c.relative_path.ends_with("Shipping.EXE")));
        assert!(scan_folder(&root.join("missing")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

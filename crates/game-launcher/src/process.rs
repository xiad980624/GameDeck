use std::path::Path;

pub fn belongs_to_game(executable:&Path,root:&Path)->bool{
    let file=executable.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    if !file.ends_with(".exe")||["launcher","crash","unins","setup","report","redprelauncher","updater","installer"].iter().any(|s|file.contains(s)){return false;}
    let normalize=|p:&Path|p.to_string_lossy().replace('/',"\\").trim_start_matches("\\\\?\\").trim_end_matches('\\').to_ascii_lowercase();
    let root=normalize(root);let path=normalize(executable);
    !root.is_empty()&&root.len()>3&&path.starts_with(&(root+"\\"))&&!path.contains("\\.egstore\\")
}

#[cfg(windows)]
pub fn game_processes(root:&Path)->Result<Vec<u32>,String>{
    scan_processes(root, None)
}

#[cfg(windows)]
pub fn child_game_processes(root: &Path, launched_pid: u32, known: &mut std::collections::HashSet<u32>) -> Result<Vec<u32>, String> {
    scan_processes(root, Some((launched_pid, known)))
}

#[cfg(windows)]
fn scan_processes(root:&Path, lineage:Option<(u32,&mut std::collections::HashSet<u32>)>)->Result<Vec<u32>,String>{
    use std::{ffi::c_void,mem::size_of};
    type Handle=*mut c_void;
    #[repr(C)]struct Entry{size:u32,usage:u32,pid:u32,heap:usize,module:u32,threads:u32,parent:u32,priority:i32,flags:u32,exe:[u16;260]}
    #[link(name="kernel32")]extern "system"{
        fn CreateToolhelp32Snapshot(flags:u32,pid:u32)->Handle;
        fn Process32FirstW(snapshot:Handle,entry:*mut Entry)->i32;
        fn Process32NextW(snapshot:Handle,entry:*mut Entry)->i32;
        fn OpenProcess(access:u32,inherit:i32,pid:u32)->Handle;
        fn QueryFullProcessImageNameW(process:Handle,flags:u32,path:*mut u16,size:*mut u32)->i32;
        fn CloseHandle(handle:Handle)->i32;
    }
    unsafe{
        let snapshot=CreateToolhelp32Snapshot(2,0);if snapshot as isize == -1{return Err(std::io::Error::last_os_error().to_string());}
        let mut entry:Entry=std::mem::zeroed();entry.size=size_of::<Entry>() as u32;
        let mut found=Vec::new();let mut next=Process32FirstW(snapshot,&mut entry);
        if next==0{let error=std::io::Error::last_os_error().to_string();CloseHandle(snapshot);return Err(error);}
        let mut entries=Vec::new();
        while next!=0{
            if entry.pid!=0 { entries.push((entry.pid,entry.parent)); }
            next=Process32NextW(snapshot,&mut entry);
        }
        CloseHandle(snapshot);
        if let Some((launched,known))=lineage {
            let mut related=std::collections::HashSet::from([launched]);
            related.extend(known.iter().copied());
            while entries.iter().any(|&(pid,parent)|related.contains(&parent)&&related.insert(pid)) {}
            entries.retain(|(pid,_)|related.contains(pid));
            known.clear();
            known.extend(entries.iter().map(|&(pid,_)|pid));
        }
        let mut path=vec![0u16;32768];
        for (pid,_) in entries {
            let process=OpenProcess(0x1000,0,pid);
            if !process.is_null(){let mut len=path.len() as u32;
                if QueryFullProcessImageNameW(process,0,path.as_mut_ptr(),&mut len)!=0&&belongs_to_game(Path::new(&String::from_utf16_lossy(&path[..len as usize])),root){found.push(pid);}
                CloseHandle(process);
            }
        }
        Ok(found)
    }
}
#[cfg(not(windows))]pub fn game_processes(_root:&Path)->Result<Vec<u32>,String>{Err("仅支持 Windows".into())}
#[cfg(not(windows))]pub fn child_game_processes(_: &Path,_:u32,_:&mut std::collections::HashSet<u32>)->Result<Vec<u32>,String>{Err("仅支持 Windows".into())}

#[cfg(test)]mod tests{use super::*;
    #[cfg(windows)]
    #[test]fn observes_a_real_process_only_in_its_installation(){
        let root=std::env::temp_dir().join(format!("gamedeck-process-{}",std::process::id()));std::fs::create_dir_all(&root).unwrap();
        let path=root.join("game-fixture.exe");std::fs::copy(std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/ping.exe"),&path).unwrap();
        let mut child=game_core::background_command(&path).args(["-t","127.0.0.1"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
        let result=(||{for _ in 0..20{if game_processes(&root)?.contains(&child.id()){return Ok::<_,String>(true);}std::thread::sleep(std::time::Duration::from_millis(25));}Ok(false)})();
        let _=child.kill();let _=child.wait();let _=std::fs::remove_dir_all(&root);
        assert!(result.unwrap(),"The real game process must be observed");
    }
    #[test]fn process_must_belong_to_installation_and_not_be_a_bootstrap(){
        let root=Path::new("F:/Cyberpunk2077");
        assert!(belongs_to_game(Path::new("f:/cyberpunk2077/bin/x64/Cyberpunk2077.exe"),root));
        for path in ["F:/Cyberpunk2077-other/game.exe","F:/Cyberpunk2077/redprelauncher.exe","F:/Cyberpunk2077/bin/CrashReporter.exe","F:/Epic Games/Launcher/EpicGamesLauncher.exe"]{assert!(!belongs_to_game(Path::new(path),root));}
    }
}

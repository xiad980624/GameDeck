//! Place only verified game-process windows on the launcher's monitor.
use std::collections::HashMap;
use std::time::{Duration,Instant};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Rect{pub x:i32,pub y:i32,pub width:i32,pub height:i32}
impl Rect{pub fn valid(self)->bool{self.width>0&&self.height>0}}
pub fn placement(window:Rect,source:Rect,target:Rect,work:Rect)->Rect{
    let fullscreen=(window.x-source.x).abs()<=16&&(window.y-source.y).abs()<=16
        &&(window.width-source.width).abs()<=32&&(window.height-source.height).abs()<=32;
    if fullscreen{return target;}
    let width=window.width.min(work.width).max(1);let height=window.height.min(work.height).max(1);
    Rect{x:work.x+(work.width-width)/2,y:work.y+(work.height-height)/2,width,height}
}
pub struct DisplayFollower{target:Rect,attempts:HashMap<usize,(u8,Instant)>,first_window:Option<Instant>,borderless:bool,stripped:std::collections::HashSet<usize>}
/// Game windows used for session monitoring, including minimized windows.
pub fn game_windows(pids: &[u32]) -> Vec<(usize, u32)> {
    native::windows(pids, true)
}

pub fn game_window_exists(window: usize, pid: u32) -> bool {
    native::window_exists(window, pid)
}

impl DisplayFollower{
    pub fn new(target:Rect)->Self{Self::with_borderless(target,false)}
    pub fn with_borderless(target:Rect,borderless:bool)->Self{Self{target,attempts:HashMap::new(),first_window:None,borderless,stripped:std::collections::HashSet::new()}}
    pub fn settled(&self)->bool{self.first_window.is_some_and(|t|t.elapsed()>Duration::from_secs(25))}
    pub fn update(&mut self,pids:&[u32])->Vec<String>{
        if self.settled(){return Vec::new();}
        let mut results=Vec::new();
        for (window,pid) in native::windows(pids, false){
            self.first_window.get_or_insert_with(Instant::now);
            // Strip the title bar once per window so a windowed game becomes a
            // borderless window that fills the target monitor. This is a no-op
            // for games that are already borderless or exclusive fullscreen.
            if self.borderless&&!self.stripped.contains(&window){
                match native::make_borderless(window,self.target){
                    Ok(true)=>results.push(format!("Made game window (process {pid}) borderless on target monitor")),
                    Err(e)=>results.push(format!("Borderless conversion failed (process {pid}): {e}")),
                    Ok(false)=>(),
                }
                self.stripped.insert(window);
            }
            if self.attempts.get(&window).is_some_and(|(count,time)|*count>=3||time.elapsed()<Duration::from_secs(2)){continue;}
            match native::move_window(window,self.target){
                Ok(false)=>(),result=>{
                    let count=self.attempts.get(&window).map_or(1,|(count,_)|count+1);self.attempts.insert(window,(count,Instant::now()));
                    results.push(match result{Ok(true)=>format!("Moved game window (process {pid}) to {},{}",self.target.x,self.target.y),Err(e)=>format!("Game display placement failed (process {pid}): {e}"),_=>unreachable!()});
                }
            }
        }results
    }
}
#[cfg(windows)]mod native{
    use super::*;use std::ffi::c_void;
    type Handle=*mut c_void;
    #[repr(C)]#[derive(Clone,Copy,Default)]struct WinRect{left:i32,top:i32,right:i32,bottom:i32}
    impl From<WinRect> for Rect{fn from(r:WinRect)->Self{Self{x:r.left,y:r.top,width:r.right-r.left,height:r.bottom-r.top}}}
    #[repr(C)]struct Point{x:i32,y:i32}
    #[repr(C)]#[derive(Default)]struct MonitorInfo{size:u32,monitor:WinRect,work:WinRect,flags:u32}
    #[link(name="user32")]extern "system"{
        fn EnumWindows(callback:unsafe extern "system" fn(Handle,isize)->i32,data:isize)->i32;
        fn GetWindowThreadProcessId(window:Handle,pid:*mut u32)->u32;
        fn IsWindowVisible(window:Handle)->i32;fn IsIconic(window:Handle)->i32;
        fn GetWindow(window:Handle,command:u32)->Handle;
        fn GetWindowLongPtrW(window:Handle,index:i32)->isize;
        fn SetWindowLongPtrW(window:Handle,index:i32,new_long:isize)->isize;
        fn GetWindowRect(window:Handle,rect:*mut WinRect)->i32;
        fn MonitorFromWindow(window:Handle,flags:u32)->Handle;
        fn MonitorFromPoint(point:Point,flags:u32)->Handle;
        fn GetMonitorInfoW(monitor:Handle,info:*mut MonitorInfo)->i32;
        fn SetWindowPos(window:Handle,after:Handle,x:i32,y:i32,width:i32,height:i32,flags:u32)->i32;
    }
    struct Search<'a>{pids:&'a[u32],windows:Vec<(usize,u32)>,include_minimized:bool}
    unsafe extern "system" fn collect(window:Handle,data:isize)->i32{
        let search=&mut*(data as *mut Search);let mut pid=0;GetWindowThreadProcessId(window,&mut pid);
        if !search.pids.contains(&pid)||IsWindowVisible(window)==0||!GetWindow(window,4).is_null()||GetWindowLongPtrW(window,-20)&0x80!=0{return 1;}
        if IsIconic(window)!=0 {
            if search.include_minimized { search.windows.push((window as usize,pid)); }
            return 1;
        }
        let mut rect=WinRect::default();if GetWindowRect(window,&mut rect)!=0&&rect.right-rect.left>=320&&rect.bottom-rect.top>=180{search.windows.push((window as usize,pid));}1
    }
    pub fn windows(pids:&[u32],include_minimized:bool)->Vec<(usize,u32)>{
        if pids.is_empty() { return Vec::new(); }
        let mut search=Search{pids,windows:Vec::new(),include_minimized};unsafe{EnumWindows(collect,&mut search as *mut _ as isize);}search.windows
    }
    pub fn window_exists(window:usize,pid:u32)->bool {
        let mut current=0;
        unsafe { GetWindowThreadProcessId(window as Handle,&mut current)!=0 && current==pid }
    }
    unsafe fn info(monitor:Handle)->Result<MonitorInfo,String>{let mut info=MonitorInfo{size:std::mem::size_of::<MonitorInfo>() as u32,..Default::default()};if monitor.is_null()||GetMonitorInfoW(monitor,&mut info)==0{Err("Display is no longer connected".into())}else{Ok(info)}}
    pub fn move_window(window:usize,target:Rect)->Result<bool,String>{unsafe{
        let handle=window as Handle;let destination=MonitorFromPoint(Point{x:target.x+1,y:target.y+1},0);
        let dest=info(destination)?;let target:Rect=dest.monitor.into();
        // Never redirect to a replacement display after the selected screen disappears.
        let source_monitor=MonitorFromWindow(handle,2);if source_monitor==destination{return Ok(false);}
        let source=info(source_monitor)?;let mut rect=WinRect::default();if GetWindowRect(handle,&mut rect)==0{return Err(std::io::Error::last_os_error().to_string());}
        let bounds=placement(rect.into(),source.monitor.into(),target,dest.work.into());
        if !bounds.valid(){return Err("Invalid display bounds".into());}
        if SetWindowPos(handle,std::ptr::null_mut(),bounds.x,bounds.y,bounds.width,bounds.height,0x4000|0x0010|0x0004)==0{return Err(std::io::Error::last_os_error().to_string());}Ok(true)
    }}
    /// Strip the title bar and resizing border from a windowed game and make
    /// it fill the target monitor like a borderless-fullscreen window. Returns
    /// Ok(true) only when a style change was actually applied (call again is a
    /// cheap no-op). Exclusive-fullscreen swapchains have no caption already
    /// and are left untouched.
    pub fn make_borderless(window:usize,target:Rect)->Result<bool,String>{unsafe{
        let handle=window as Handle;
        const GWL_STYLE:i32=-16;
        const WS_CAPTION:isize=0x00C00000;
        const WS_THICKFRAME:isize=0x00040000;
        const WS_SYSMENU:isize=0x00080000;
        const WS_MINIMIZEBOX:isize=0x00020000;
        const WS_MAXIMIZEBOX:isize=0x00010000;
        const SWP_FRAMECHANGED:u32=0x0020;
        const SWP_SHOWWINDOW:u32=0x0040;
        const SWP_NOZORDER:u32=0x0004;
        let style=GetWindowLongPtrW(handle,GWL_STYLE);
        // Already borderless (borderless fullscreen or exclusive swapchain).
        if style&(WS_CAPTION|WS_THICKFRAME)==0{return Ok(false);}
        let stripped=style&!(WS_CAPTION|WS_THICKFRAME|WS_SYSMENU|WS_MINIMIZEBOX|WS_MAXIMIZEBOX);
        if SetWindowLongPtrW(handle,GWL_STYLE,stripped)==0&&style!=0{
            return Err(std::io::Error::last_os_error().to_string());
        }
        // FRAMECHANGED makes Windows recompute the non-client area after the
        // style change, otherwise cached borders leave dead margins.
        if SetWindowPos(handle,std::ptr::null_mut(),target.x,target.y,target.width,target.height,
            SWP_FRAMECHANGED|SWP_SHOWWINDOW|SWP_NOZORDER)==0{
            SetWindowLongPtrW(handle,GWL_STYLE,style);
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(true)
    }}
    #[cfg(test)]mod live_tests{
        use super::*;
        #[repr(C)]struct Class{style:u32,procedure:Option<unsafe extern "system" fn(Handle,u32,usize,isize)->isize>,class_extra:i32,window_extra:i32,instance:Handle,icon:Handle,cursor:Handle,background:Handle,menu:*const u16,name:*const u16}
        #[link(name="kernel32")]extern "system"{fn GetModuleHandleW(name:*const u16)->Handle;}
        #[link(name="user32")]extern "system"{
            fn EnumDisplayMonitors(dc:Handle,clip:*const WinRect,callback:unsafe extern "system" fn(Handle,Handle,*mut WinRect,isize)->i32,data:isize)->i32;
            fn RegisterClassW(class:*const Class)->u16;fn UnregisterClassW(name:*const u16,instance:Handle)->i32;
            fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,x:i32,y:i32,width:i32,height:i32,parent:Handle,menu:Handle,instance:Handle,param:Handle)->Handle;
            fn DefWindowProcW(window:Handle,message:u32,wparam:usize,lparam:isize)->isize;
            fn DestroyWindow(window:Handle)->i32;fn ShowWindow(window:Handle,command:i32)->i32;
            fn SetLayeredWindowAttributes(window:Handle,color:u32,alpha:u8,flags:u32)->i32;
        }
        unsafe extern "system" fn monitors(monitor:Handle,_:Handle,_:*mut WinRect,data:isize)->i32{if let Ok(info)=info(monitor){(*(data as *mut Vec<Rect>)).push(info.monitor.into());}1}
        #[test]fn native_window_filter_and_monitor_placement_use_real_windows(){unsafe{
            let mut screens=Vec::<Rect>::new();EnumDisplayMonitors(std::ptr::null_mut(),std::ptr::null(),monitors,&mut screens as *mut _ as isize);assert!(!screens.is_empty());
            let instance=GetModuleHandleW(std::ptr::null());let name:Vec<u16>=format!("GameDeckDisplayFixture{}",std::process::id()).encode_utf16().chain(Some(0)).collect();
            let class=Class{style:0,procedure:Some(DefWindowProcW),class_extra:0,window_extra:0,instance,icon:std::ptr::null_mut(),cursor:std::ptr::null_mut(),background:std::ptr::null_mut(),menu:std::ptr::null(),name:name.as_ptr()};assert_ne!(RegisterClassW(&class),0);
            let source=screens[0];let window=CreateWindowExW(0x80000|0x20,name.as_ptr(),name.as_ptr(),0x80000000,source.x+30,source.y+30,640,360,std::ptr::null_mut(),std::ptr::null_mut(),instance,std::ptr::null_mut());assert!(!window.is_null());
            // A transparent, nonactivated fixture never obscures the user's desktop.
            SetLayeredWindowAttributes(window,0,0,2);ShowWindow(window,4);
            let filtered=windows(&[std::process::id()],false);let empty=windows(&[],false);
            let monitored=windows(&[std::process::id()],true);
            let destination=*screens.get(1).unwrap_or(&source);let moved=move_window(window as usize,destination);let mut actual=WinRect::default();GetWindowRect(window,&mut actual);
            DestroyWindow(window);UnregisterClassW(name.as_ptr(),instance);
            assert!(filtered.iter().any(|(handle,_)|*handle==window as usize));assert!(empty.is_empty());
            assert!(monitored.contains(&(window as usize,std::process::id())));
            assert!(!window_exists(window as usize,std::process::id()));
            assert_eq!(moved.unwrap(),screens.len()>1);
            if screens.len()>1{assert!(actual.left>=destination.x&&actual.top>=destination.y&&actual.right<=destination.x+destination.width&&actual.bottom<=destination.y+destination.height);}
            println!("Native display fixture: {} monitors, cross-monitor move {}",screens.len(),screens.len()>1);
        }}
    }
}
#[cfg(not(windows))]mod native{use super::*;pub fn windows(_: &[u32],_:bool)->Vec<(usize,u32)>{Vec::new()}pub fn window_exists(_:usize,_:u32)->bool{false}pub fn move_window(_:usize,_:Rect)->Result<bool,String>{Ok(false)}pub fn make_borderless(_:usize,_:Rect)->Result<bool,String>{Ok(false)}}

#[cfg(test)]mod tests{use super::*;
 #[test]fn borderless_windows_fill_target_monitor_with_negative_coordinates(){let source=Rect{x:0,y:0,width:1920,height:1080};let tv=Rect{x:-3840,y:0,width:3840,height:2160};assert_eq!(placement(source,source,tv,tv),tv);}
 #[test]fn windowed_games_remain_windowed_and_fit_the_destination_work_area(){let source=Rect{x:0,y:0,width:3840,height:2160};let tv=Rect{x:3840,y:-200,width:1920,height:1080};let work=Rect{height:1040,..tv};assert_eq!(placement(Rect{x:200,y:100,width:1280,height:720},source,tv,work),Rect{x:4160,y:-40,width:1280,height:720});let large=placement(Rect{x:400,y:300,width:3000,height:1600},source,tv,work);assert_eq!(large,work);}
 #[test]fn empty_process_list_cannot_move_unrelated_windows(){let mut follower=DisplayFollower::new(Rect{x:0,y:0,width:1920,height:1080});assert!(follower.update(&[]).is_empty());assert!(!follower.settled());}
}

use serde::Serialize;
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use tauri::{AppHandle,Emitter,Manager,State};

const XINPUT_SLOTS: usize = 4;
const JOYSTICK_SLOTS: usize = 16;
const WAKE_SLOTS: usize = XINPUT_SLOTS + JOYSTICK_SLOTS;

#[derive(Clone,Debug,Serialize,PartialEq)]pub struct Button{pressed:bool,touched:bool,value:f32}
#[derive(Clone,Debug,Serialize,PartialEq)]pub struct Pad{id:String,index:u32,connected:bool,axes:Vec<f32>,buttons:Vec<Button>}
#[derive(Clone,Debug,Serialize,PartialEq,Default)]pub struct Snapshot{available:bool,foreground:bool,wake_held:bool,pads:Vec<Pad>}
pub struct NativeGamepad(Mutex<Snapshot>);
fn axis(value:i16)->f32{if value<0{value as f32/32768.0}else{value as f32/32767.0}}
fn pad(index:u32,buttons:u16,left_trigger:u8,right_trigger:u8,x:i16,y:i16)->Pad{
    let masks=[0x1000,0x2000,0x4000,0x8000,0x100,0x200,0,0,0x20,0x10,0x40,0x80,1,2,4,8];
    let buttons=masks.into_iter().enumerate().map(|(i,mask)|{let value=match i{6=>left_trigger as f32/255.0,7=>right_trigger as f32/255.0,_=>if buttons&mask!=0{1.0}else{0.0}};Button{pressed:value>0.5,touched:value>0.0,value}}).collect();
    Pad{id:format!("Xbox controller {index} (XInput)"),index,connected:true,axes:vec![axis(x),-axis(y)],buttons}
}

#[derive(Default)]
struct WakeGesture {
    held: [bool; WAKE_SLOTS],
    claimed: [bool; WAKE_SLOTS],
}

impl WakeGesture {
    fn observe(&mut self, pads: &[Pad], joysticks: &[bool; JOYSTICK_SLOTS]) -> [bool; WAKE_SLOTS] {
        let mut down = [false; WAKE_SLOTS];
        for pad in pads {
            let index = pad.index as usize;
            if index < XINPUT_SLOTS {
                let state = self.held[index];
                let threshold = if state { 0.45 } else { 0.66 };
                down[index] = pad.axes.iter().any(|axis| axis.abs() > threshold)
                    || [0, 1, 2, 3, 4, 5, 8, 9, 12, 13, 14, 15].iter()
                        .any(|&i| pad.buttons.get(i).is_some_and(|button| button.pressed));
            }
        }
        down[XINPUT_SLOTS..].copy_from_slice(joysticks);
        let mut pressed = [false; WAKE_SLOTS];
        for (index, down) in down.into_iter().enumerate() {
            pressed[index] = down && !self.held[index];
            self.claimed[index] &= down;
            self.held[index] = down;
        }
        pressed
    }

    fn claim(&mut self, pressed: [bool; WAKE_SLOTS]) {
        for (claimed, newly_pressed) in self.claimed.iter_mut().zip(pressed) {
            *claimed |= newly_pressed;
        }
    }

    fn is_held(&self) -> bool { self.claimed.contains(&true) }
}

#[cfg(windows)]mod platform{
    use super::*;use std::ffi::c_void;
    #[repr(C)]#[derive(Default)]struct Gamepad{buttons:u16,left_trigger:u8,right_trigger:u8,x:i16,y:i16,right_x:i16,right_y:i16}
    #[repr(C)]#[derive(Default)]struct InputState{packet:u32,gamepad:Gamepad}
    type GetState=unsafe extern "system" fn(u32,*mut InputState)->u32;
    #[link(name="kernel32")]extern "system"{fn GetSystemDirectoryW(path:*mut u16,size:u32)->u32;fn LoadLibraryW(path:*const u16)->*mut c_void;fn GetProcAddress(module:*mut c_void,name:*const u8)->*mut c_void;fn FreeLibrary(module:*mut c_void)->i32;}
    #[link(name="user32")]extern "system"{fn GetForegroundWindow()->*mut c_void;}
    #[repr(C)]
    #[derive(Default)]
    struct JoystickPosition {
        size: u32,
        flags: u32,
        x: u32,
        y: u32,
        z: u32,
        r: u32,
        u: u32,
        v: u32,
        buttons: u32,
        button_number: u32,
        pov: u32,
        reserved1: u32,
        reserved2: u32,
    }
    #[link(name="winmm")]
    extern "system" {
        fn joyGetNumDevs() -> u32;
        fn joyGetPosEx(index: u32, position: *mut JoystickPosition) -> u32;
    }

    /// Detect button/D-pad wake gestures on WinMM joysticks. WebView maps their navigation.
    #[derive(Default)]
    pub struct Joysticks {
        connected: u32,
        last_scan: Option<std::time::Instant>,
    }

    impl Joysticks {
        pub fn buttons(&mut self) -> ([bool; JOYSTICK_SLOTS], bool) {
            let scan_all = self.last_scan.is_none_or(|scan| scan.elapsed().as_millis() >= 250);
            let candidates = if scan_all {
                self.last_scan = Some(std::time::Instant::now());
                let count = unsafe { joyGetNumDevs() }.min(JOYSTICK_SLOTS as u32);
                (1u32 << count) - 1
            } else {
                self.connected
            };
            let mut pressed = [false; JOYSTICK_SLOTS];
            self.connected = 0;
            for (index, down) in pressed.iter_mut().enumerate() {
                let slot = 1 << index;
                if candidates & slot == 0 { continue; }
                let mut position = JoystickPosition {
                    size: std::mem::size_of::<JoystickPosition>() as u32,
                    flags: 0x40 | 0x80, // JOY_RETURNPOV and JOY_RETURNBUTTONS
                    pov: 0xffff,
                    ..Default::default()
                };
                if unsafe { joyGetPosEx(index as u32, &mut position) } == 0 {
                    *down = position.buttons != 0 || position.pov < 36000;
                    self.connected |= slot;
                }
            }
            (pressed, self.connected != 0)
        }
    }
    pub struct Reader{module:usize,get:Option<GetState>}
    impl Reader{
        pub fn new()->Self{unsafe{
            let mut system=[0u16;32768];let size=GetSystemDirectoryW(system.as_mut_ptr(),system.len() as u32) as usize;
            if size==0||size>=system.len(){return Self{module:0,get:None};}
            for name in ["xinput1_4.dll","xinput9_1_0.dll"]{
                let path:Vec<u16>=system[..size].iter().copied().chain("\\".encode_utf16()).chain(name.encode_utf16()).chain(Some(0)).collect();
                let module=LoadLibraryW(path.as_ptr());if module.is_null(){continue;}
                let function=GetProcAddress(module,b"XInputGetState\0".as_ptr());if !function.is_null(){return Self{module:module as usize,get:Some(std::mem::transmute::<*mut c_void,GetState>(function))};}FreeLibrary(module);
            }Self{module:0,get:None}
        }}
        pub fn read(&self,window:usize)->Snapshot{unsafe{
            let foreground=GetForegroundWindow() as usize==window;
            let mut snapshot=Snapshot{available:self.get.is_some(),foreground,..Default::default()};
            // Connectivity is independent of focus. The frontend gates actions on
            // foreground, so gameplay never navigates the background lobby.
            if let Some(get)=self.get{for index in 0..XINPUT_SLOTS as u32{let mut state=InputState::default();if get(index,&mut state)==0{let g=state.gamepad;snapshot.pads.push(pad(index,g.buttons,g.left_trigger,g.right_trigger,g.x,g.y));}}}
            snapshot
        }}
    }
    impl Drop for Reader{fn drop(&mut self){if self.module!=0{unsafe{FreeLibrary(self.module as *mut c_void);}}}}
}

pub fn start(app:&AppHandle){
    app.manage(NativeGamepad(Mutex::new(Snapshot::default())));
    #[cfg(windows)]{
        let Some(window)=app.get_webview_window("main")else{return;};let Ok(handle)=window.hwnd()else{return;};let handle=handle.0 as usize;
        let stop=Arc::new(AtomicBool::new(false));let closed=stop.clone();window.on_window_event(move|event|{if matches!(event,tauri::WindowEvent::Destroyed){closed.store(true,Ordering::Release);}});
        let app=app.clone();std::thread::spawn(move||{
            let reader=platform::Reader::new();let mut last=Snapshot::default();let mut emitted=std::time::Instant::now();
            let mut gesture=WakeGesture::default();
            let mut joysticks=platform::Joysticks::default();
            while !stop.load(Ordering::Acquire){
                let mut snapshot=reader.read(handle);let foreground=snapshot.foreground;
                let (joystick_buttons,joystick_connected)=joysticks.buttons();
                let pressed=gesture.observe(&snapshot.pads,&joystick_buttons);
                let target=if !foreground&&pressed.contains(&true){crate::lobby_focus::gamepad_wake_target(handle)}else{None};
                if target.is_some(){gesture.claim(pressed);}
                snapshot.wake_held=gesture.is_held();
                if snapshot!=last||emitted.elapsed().as_millis()>=150{
                    if let Ok(mut saved)=app.state::<NativeGamepad>().0.lock(){*saved=snapshot.clone();}
                    let _=app.emit("native-gamepad-state",&snapshot);last=snapshot;emitted=std::time::Instant::now();
                }
                if let Some(target)=target {crate::lobby_focus::wake_from_gamepad(&app,target);}
                // A short tap must survive while another display has the focus.
                let delay=if foreground{16}else if !last.pads.is_empty()||joystick_connected{24}else{100};
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
        });
    }
}
#[tauri::command]pub fn native_gamepad_status(state:State<'_,NativeGamepad>)->Result<Snapshot,String>{state.0.lock().map(|snapshot|snapshot.clone()).map_err(|e|e.to_string())}

#[cfg(test)]mod tests{use super::*;
    #[test]
    fn a_button_held_when_focus_is_lost_does_not_request_another_wake() {
        let mut wake = WakeGesture::default();
        let no_joysticks = [false; JOYSTICK_SLOTS];
        let pressed = [pad(0, 0x1000, 0, 0, 0, 0)];
        assert!(wake.observe(&pressed, &no_joysticks)[0]);
        assert!(!wake.observe(&pressed, &no_joysticks)[0]);
        wake.observe(&[], &no_joysticks);
        let edge = wake.observe(&pressed, &no_joysticks);
        assert!(edge[0]);
        wake.claim(edge);
        assert!(wake.is_held());
        assert!(!wake.observe(&pressed, &no_joysticks)[0]);
        wake.observe(&[], &no_joysticks);
        assert!(!wake.is_held());
    }

    #[test]
    fn stick_drift_is_ignored_and_legacy_buttons_have_independent_edges() {
        let mut wake = WakeGesture::default();
        let mut joysticks = [false; JOYSTICK_SLOTS];
        assert!(!wake.observe(&[pad(0, 0, 0, 0, 17000, 0)], &joysticks).contains(&true));
        joysticks[2] = true;
        let edge = wake.observe(&[pad(0, 0, 0, 0, 24500, 0)], &joysticks);
        assert!(edge[0] && edge[XINPUT_SLOTS + 2]);
        wake.claim(edge);
        wake.observe(&[pad(0, 0, 0, 0, 16000, 0)], &joysticks);
        assert!(wake.is_held());
        wake.observe(&[], &[false; JOYSTICK_SLOTS]);
        assert!(!wake.is_held());
    }
    #[test]fn native_buttons_use_the_same_standard_indices_as_the_web_gamepad(){let p=pad(1,0x1000|0x20|4,0,255,0,0);assert!(p.buttons[0].pressed&&p.buttons[8].pressed&&p.buttons[14].pressed&&p.buttons[7].pressed);assert!(!p.buttons[1].pressed&&!p.buttons[6].pressed);}
    #[test]fn native_axes_are_normalized_and_up_is_negative(){let p=pad(0,0,0,0,-32768,32767);assert_eq!(p.axes,vec![-1.0,-1.0]);let p=pad(0,0,0,0,32767,-32768);assert_eq!(p.axes,vec![1.0,1.0]);}
}

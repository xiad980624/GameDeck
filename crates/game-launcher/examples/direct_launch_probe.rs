//! Launch an explicitly supplied installed executable and observe that exact child.
fn main(){
 let path=std::env::args().nth(1).expect("installed game executable");
 let target=game_core::LaunchTarget{game_id:"direct-probe".into(),provider:"manual".into(),external_id:String::new(),install_path:path};
 let game_launcher::LaunchResult::Direct(mut child)=game_launcher::launch(&target).expect("launch executable")else{panic!("not direct")};
 println!("spawned pid={}",child.id());
 for _ in 0..20{
  if let Some(status)=child.try_wait().unwrap(){println!("early_exit code={status}");std::process::exit(1);}
  std::thread::sleep(std::time::Duration::from_millis(500));
 }
 println!("game_process_alive_after_10s pid={}",child.id());
}

//! Explicit on-device smoke test. Observes the game and leaves it running.
fn main(){
 let args:Vec<_>=std::env::args().skip(1).collect();assert_eq!(args.len(),3,"identity install-directory launcher-executable");
 let root=std::path::Path::new(&args[1]);
 if !game_launcher::game_processes(root).expect("process inspection").is_empty(){println!("already_running");return;}
 let target=game_core::LaunchTarget{game_id:"epic-launch-probe".into(),provider:"epic".into(),external_id:args[0].clone(),install_path:args[1].clone()};
 game_launcher::launch_with_client(&target,Some(std::path::Path::new(&args[2]))).expect("Epic invocation");
 let start=std::time::Instant::now();
 while start.elapsed().as_secs()<90{
  let pids=game_launcher::game_processes(root).expect("process inspection");
  if !pids.is_empty(){println!("game_running elapsed_ms={} processes={:?}",start.elapsed().as_millis(),pids);return;}
  std::thread::sleep(std::time::Duration::from_millis(350));
 }
 eprintln!("game_not_observed_within_90s");std::process::exit(1);
}

use std::{path::PathBuf,sync::Mutex,time::Instant};
pub struct Trace {started:Instant,log:PathBuf,events:Mutex<Vec<serde_json::Value>>,pub probe:bool}
impl Trace {
 pub fn new(started:Instant,logs:PathBuf,probe:bool)->Self{let _=std::fs::create_dir_all(&logs);Self{started,log:logs.join(if probe{"startup-probe.json"}else{"startup-latest.json"}),events:Mutex::new(Vec::new()),probe}}
 pub fn has(&self,stage:&str)->bool{self.events.lock().unwrap().iter().any(|e|e["stage"]==stage)}
 pub fn mark(&self,stage:&str,frontend_ms:Option<f64>){let mut events=self.events.lock().unwrap();if events.iter().any(|e|e["stage"]==stage){return;}
  events.push(serde_json::json!({"stage":stage,"native_ms":self.started.elapsed().as_secs_f64()*1000.,"frontend_ms":frontend_ms}));
  let report=serde_json::json!({"pid":std::process::id(),"probe":self.probe,"events":*events});if let Ok(bytes)=serde_json::to_vec_pretty(&report){let _=std::fs::write(&self.log,bytes);}
 }
}

//! Verify a title inside the actual 16:9 background crop. No language ranking or compositing.
use std::{path::Path,hash::{Hash,Hasher},sync::Mutex};
use image::{DynamicImage,RgbImage,RgbaImage};
static WORKER:Mutex<()>=Mutex::new(());
fn distance(a:&[u8],b:&[u8])->u32{a.iter().zip(b).take(3).map(|(a,b)|a.abs_diff(*b) as u32).sum()}
pub fn screen_crop(image:&DynamicImage)->DynamicImage{
 let(w,h)=(image.width(),image.height());
 let(cw,ch)=if w as f64/h as f64>16./9.{((h as f64*16./9.).round() as u32,h)}else{(w,(w as f64*9./16.).round() as u32)};
 image.crop_imm((w-cw)/2,(h-ch)/2,cw.max(1),ch.max(1))
}
pub fn usable_logo(path:&Path)->bool{image::open(path).is_ok_and(|image|{let(w,h)=(image.width(),image.height());w>=100&&h>=20&&w<=6000&&h<=4000&&trim(&image.to_rgba8()).is_some()})}
fn trim(logo:&RgbaImage)->Option<RgbaImage>{
 let(mut l,mut t,mut r,mut b)=(logo.width(),logo.height(),0,0);let mut transparent=false;
 for(x,y,p)in logo.enumerate_pixels(){if p[3]>230{l=l.min(x);t=t.min(y);r=r.max(x);b=b.max(y);}else if p[3]<10{transparent=true;}}
 (transparent&&r>l&&b>t).then(||image::imageops::crop_imm(logo,l,t,r-l+1,b-t+1).to_image())
}
/// Conservative masked color-and-silhouette comparison; blank white areas cannot pass.
pub fn contains_logo(frame:&RgbImage,original:&RgbaImage)->bool{
 let Some(original)=trim(original)else{return false};
 if original.width()<32||original.height()<16{return false;}
 for w in (32..=frame.width().min(384)).step_by(4){
  let h=(original.height()as f64*w as f64/original.width()as f64).round()as u32;
  if h<16||h>=frame.height(){continue;}
  let logo=image::imageops::resize(&original,w,h,image::imageops::FilterType::Triangle);
  let mut solid=Vec::new();let mut edges=Vec::new();
  for y in 2..h-2{for x in 2..w-2{let p=logo.get_pixel(x,y);
   if p[3]>245&&[(x-1,y),(x+1,y),(x,y-1),(x,y+1)].iter().all(|&(a,b)|logo.get_pixel(a,b)[3]>245){solid.push((x,y,*p));}
   if p[3]>230{for(dx,dy)in[(-2,0),(2,0),(0,-2),(0,2)]{let nx=(x as i32+dx)as u32;let ny=(y as i32+dy)as u32;if logo.get_pixel(nx,ny)[3]<10{edges.push((x,y,nx,ny));break;}}}
  }}
  if solid.len()<40||edges.len()<12{continue;}
  let points:Vec<_>=(0..96).map(|i|solid[(i*7919)%solid.len()]).collect();
  let edge_points:Vec<_>=(0..32).map(|i|edges[(i*997)%edges.len()]).collect();
  for oy in (0..=frame.height()-h).step_by(2){for ox in (0..=frame.width()-w).step_by(2){
   let mut error=0;let mut bad=0;let mut rejected=false;
   for(i,&(x,y,p))in points.iter().enumerate(){let d=distance(&frame.get_pixel(ox+x,oy+y).0,&p.0);error+=d;if d>210{bad+=1;}
    if (i==11&&bad>3)||bad>18{rejected=true;break;}
   }
   if rejected||error>96*100{continue;}
   let contrast=edge_points.iter().filter(|&&(x,y,nx,ny)|distance(&frame.get_pixel(ox+x,oy+y).0,&frame.get_pixel(ox+nx,oy+ny).0)>65).count();
   if contrast>=18{return true;}
  }}
 }
 false
}
fn normalize(s:&str)->String{s.chars().filter(|c|c.is_alphanumeric()).flat_map(char::to_lowercase).collect()}
pub fn title_matches(text:&str,name:&str)->bool{
 let raw_name=name;let text=normalize(text);let title=super::artwork_search_name(name);let mut names=vec![normalize(name),normalize(&title)];
 for suffix in [" Definitive Edition"," Complete Edition"," Game of the Year Edition"," GOTY Edition"," Deluxe Edition"," Demo"]{if title.to_lowercase().ends_with(&suffix.to_lowercase()){names.push(normalize(&title[..title.len()-suffix.len()]));}}
 // Bilingual platform names can have a separate, fully written CJK title.
 for run in name.split(|c:char|!('\u{3400}'..='\u{9fff}').contains(&c)){if run.chars().count()>=2{names.push(normalize(run));}}
 names.into_iter().any(|name|{
  if name.is_empty()||!(name.chars().count()>=4||name.chars().all(|c|('\u{3400}'..='\u{9fff}').contains(&c))){return false;}
  let numbered=raw_name.split_whitespace().last().is_some_and(|word|word.chars().all(|c|c.is_ascii_digit()||"IVX".contains(c)));
  text.match_indices(&name).any(|(start,_)|!numbered||text[start+name.len()..].chars().next().is_none_or(|c|!c.is_ascii_digit()&&!"ivx".contains(c)))
 })
}
pub fn verify(path:&Path,name:&str,id:&str,root:&Path)->Result<String,String>{
 let _guard=WORKER.lock().map_err(|_|"标题校验任务锁异常")?;
 let meta=std::fs::metadata(path).map_err(|e|e.to_string())?;let mut hash=std::collections::hash_map::DefaultHasher::new();
 ("branded-screen-v1",path,meta.len(),meta.modified().ok(),name).hash(&mut hash);
 let mut logos:Vec<_>=std::fs::read_dir(root.join("logo")).into_iter().flatten().flatten().map(|e|e.path()).filter(|p|p.file_name().is_some_and(|n|n.to_string_lossy().starts_with(id))).collect();logos.sort();
 for logo in &logos{if let Ok(meta)=std::fs::metadata(logo){(logo,meta.len(),meta.modified().ok()).hash(&mut hash);}}
 let dir=root.join("title-checks");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
 let key=format!("{:016x}",hash.finish());let record=dir.join(format!("{key}.json"));
 if let Ok(bytes)=std::fs::read(&record){if let Ok(v)=serde_json::from_slice::<serde_json::Value>(&bytes){if let Some(proof)=v["proof"].as_str(){return Ok(proof.into());}if v["checked"]==true{return Err(v["reason"].as_str().unwrap_or("未能确认图内标题").into());}}}
 let image=screen_crop(&image::open(path).map_err(|e|e.to_string())?);let small=image.thumbnail(640,640).to_rgb8();
 for file in &logos{if let Ok(logo)=image::open(file){if contains_logo(&small,&logo.thumbnail(600,600).to_rgba8()){
  let proof="全屏裁切后仍包含本作透明标题 Logo（颜色及轮廓匹配）".to_string();let _=std::fs::write(&record,serde_json::json!({"checked":true,"proof":proof,"method":"logo-template"}).to_string());return Ok(proof);
 }}}
 let preview=dir.join(format!("{key}.jpg"));image.thumbnail(2000,2000).to_rgb8().save(&preview).map_err(|e|e.to_string())?;
 let ocr=recognize(&preview,&dir);
 let (texts,checked)=match ocr{Ok(texts)=>(texts,true),Err(reason)=>return Err(format!("图内标题未获确认；Logo 轮廓未匹配；文字识别不可用：{reason}"))};
 let matched=texts.iter().filter_map(|t|t["text"].as_str()).any(|text|title_matches(text,name));
 let proof="全屏裁切后可识别本作标题文字（保持原图语言）";
 let reason="未能确认全屏裁切后含本作标题/Logo；透明 Logo 轮廓未匹配，文字识别也未命中。艺术字可能漏识别，不能据此断言原图没有 Logo";
 let report=serde_json::json!({"checked":checked,"proof":matched.then_some(proof),"reason":reason,"texts":texts});let _=std::fs::write(&record,report.to_string());
 if matched{Ok(proof.into())}else{Err(reason.into())}
}
#[cfg(windows)]fn recognize(path:&Path,dir:&Path)->Result<Vec<serde_json::Value>,String>{
 use std::{io::Read,process::{Command,Stdio},time::{Duration,Instant}};use std::os::windows::process::CommandExt;
 let worker=dir.join("title-worker.ps1");std::fs::write(&worker,include_str!("branding.ps1")).map_err(|e|e.to_string())?;
 let output=path.with_extension("ocr.json");let mut child=Command::new("powershell.exe").creation_flags(0x08000000).args(["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File"]).arg(worker).arg("-ImagePath").arg(path).arg("-OutputPath").arg(&output).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
 let start=Instant::now();loop{if let Some(status)=child.try_wait().map_err(|e|e.to_string())?{if !status.success(){let mut error=String::new();if let Some(mut stderr)=child.stderr.take(){let _=stderr.read_to_string(&mut error);}return Err(error.chars().take(500).collect());}break;}if start.elapsed()>Duration::from_secs(20){let _=child.kill();let _=child.wait();return Err("识别超时（20 秒）".into());}std::thread::sleep(Duration::from_millis(50));}
 let bytes=std::fs::read(output).map_err(|e|e.to_string())?;serde_json::from_slice(&bytes).map_err(|e|e.to_string())
}
#[cfg(not(windows))]fn recognize(_: &Path,_:&Path)->Result<Vec<serde_json::Value>,String>{Err("当前系统没有 Windows 标题识别组件".into())}
#[cfg(test)]mod tests{use super::*;
 #[test]fn exact_game_title_required_without_language_preference(){assert!(title_matches("IT TAKES TWO","It Takes Two"));assert!(title_matches("双 人 成 行","双人成行"));assert!(!title_matches("Some other title 4K wallpaper","It Takes Two"));assert!(!title_matches("Dark Souls III","Dark Souls II"));}
 #[test]fn crop_removes_logos_that_would_be_outside_the_screen(){assert_eq!((screen_crop(&DynamicImage::new_rgb8(3840,1240)).width(),screen_crop(&DynamicImage::new_rgb8(3840,1240)).height()),(2204,1240));}
 #[test]fn matching_logo_passes_but_a_blank_area_does_not(){
  let mut logo=RgbaImage::new(80,40);for y in 4..36{for x in 4..76{if (x/8+y/8)%3!=0{logo.put_pixel(x,y,image::Rgba([230,160,25,255]));}}}
  let mut frame=RgbImage::from_pixel(200,120,image::Rgb([15,25,40]));for(x,y,p)in logo.enumerate_pixels(){if p[3]>0{frame.put_pixel(x+20,y+10,image::Rgb([p[0],p[1],p[2]]));}}
  assert!(contains_logo(&frame,&logo));assert!(!contains_logo(&RgbImage::from_pixel(200,120,image::Rgb([230,160,25])),&logo));
 }
}

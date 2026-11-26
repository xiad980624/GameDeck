//! Bounded original animation storage and metadata-based background classification.
use super::*;
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct MediaInfo {pub width:u32,pub height:u32,pub animated:bool,pub extension:String,pub duration_ms:u64,pub frames:u32}
pub fn inspect(bytes:&[u8])->Result<MediaInfo,String>{
 if bytes.len()>24_000_000{return Err("素材超过 24 MB".into());}
 if bytes.starts_with(b"GIF89a")||bytes.starts_with(b"GIF87a") {
  if bytes.len()<14{return Err("GIF 数据不完整".into());}
  let w=u16::from_le_bytes([bytes[6],bytes[7]]) as u32;let h=u16::from_le_bytes([bytes[8],bytes[9]]) as u32;
  let mut p=13usize;if bytes[10]&128!=0{p+=3*(1usize<<((bytes[10]&7)+1));}let mut frames=0;let mut duration_ms=0u64;let mut frame_delay=100u64;
  while p<bytes.len(){let marker=bytes[p];p+=1;match marker{
   0x3b=>{if frames==0{return Err("GIF 缺少图像帧".into());}return bounded(w,h,frames>1,"gif",duration_ms,frames);},
   0x21=>{if bytes.get(p)==Some(&0xf9)&&bytes.get(p+1)==Some(&4)&&p+7<=bytes.len(){let delay=u16::from_le_bytes([bytes[p+3],bytes[p+4]]) as u64*10;frame_delay=if delay==0{100}else{delay.max(10)};}p+=1;skip_blocks(bytes,&mut p)?;},
   0x2c=>{if p+9>bytes.len(){return Err("GIF 帧不完整".into());}let flags=bytes[p+8];p+=9;if flags&128!=0{p+=3*(1usize<<((flags&7)+1));}p+=1;skip_blocks(bytes,&mut p)?;frames+=1;duration_ms=duration_ms.saturating_add(frame_delay);frame_delay=100;if frames>1200{return Err("动画帧数过多".into());}},
   _=>return Err("GIF 结构无效".into()),
  }}return Err("GIF 缺少结束标记".into());
 }
 let (w,h)=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e|e.to_string())?.into_dimensions().map_err(|e|e.to_string())?;
 let webp=bytes.len()>20&&&bytes[0..4]==b"RIFF"&&&bytes[8..12]==b"WEBP";
 let mut animated=false;let mut duration_ms=0u64;let mut frames=0u32;
 if webp{let mut p=12usize;while p+8<=bytes.len(){let len=u32::from_le_bytes(bytes[p+4..p+8].try_into().unwrap()) as usize;if &bytes[p..p+4]==b"ANIM"{animated=true;}if &bytes[p..p+4]==b"ANMF"{if len<16||p+24>bytes.len(){return Err("WebP 动画帧不完整".into());}let delay=bytes[p+20] as u64|((bytes[p+21] as u64)<<8)|((bytes[p+22] as u64)<<16);duration_ms=duration_ms.saturating_add(if delay==0{100}else{delay.max(10)});frames+=1;}p=p.checked_add(8+len+(len%2)).ok_or("WebP 结构无效")?;if p>bytes.len(){return Err("WebP 数据不完整".into());}}}
 bounded(w,h,animated,if webp{"webp"}else{"png"},duration_ms,frames)
}
fn skip_blocks(bytes:&[u8],p:&mut usize)->Result<(),String>{loop{let n=*bytes.get(*p).ok_or("GIF 数据块不完整")? as usize;*p+=1;if n==0{return Ok(());}*p=p.checked_add(n).ok_or("GIF 数据块无效")?;if *p>bytes.len(){return Err("GIF 数据块不完整".into());}}}
fn bounded(w:u32,h:u32,animated:bool,ext:&str,duration_ms:u64,frames:u32)->Result<MediaInfo,String>{if w==0||h==0||w>10000||h>10000||w as u64*h as u64>40_000_000{return Err("素材像素尺寸超限".into());}if animated&&(frames==0||frames>1200){return Err("动画帧数无效".into());}if animated&&duration_ms>8000{return Err(format!("动画时长 {:.1} 秒，超过 8 秒短循环限制",duration_ms as f64/1000.0));}Ok(MediaInfo{width:w,height:h,animated,extension:ext.into(),duration_ms,frames})}
pub fn publish(bytes:&[u8],base:&Path)->Result<(PathBuf,MediaInfo),String>{
 let info=inspect(bytes)?;
 let path=if info.animated{base.with_extension(format!("motion.{}",info.extension))}else if info.extension=="gif"{base.with_extension("gif")}else{base.with_extension("png")};
 if info.animated||info.extension=="gif" {let pending=path.with_extension("pending");std::fs::write(&pending,bytes).map_err(|e|e.to_string())?;std::fs::rename(pending,&path).map_err(|e|e.to_string())?;}
 else{save_image(image::load_from_memory(bytes).map_err(|e|e.to_string())?,&path,10000)?;}
 Ok((path,info))
}
/// This is conservative label evidence, not a claim to recognize every HUD visually.
pub fn classify(text:&str)->(i32,&'static str){let t=text.to_lowercase();
 if ["logo","标志","徽标"].iter().any(|s|t.contains(s)){return (-100,"Logo / 标志");}
 if ["screenshot","screen shot","gameplay","in-game","ingame","实机","截图","游戏画面","hud","user interface","菜单"].iter().any(|s|t.contains(s)){return (-30,"疑似实机截图");}
 if ["key art","keyart","promotional","official artwork","concept art","illustration","主视觉","宣传","原画","概念图","插画"].iter().any(|s|t.contains(s)){return (30,"宣传图 / 原画");}
 (0,"壁纸 / 未分类")
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn animated_files_keep_every_original_byte(){
  let root=std::env::temp_dir().join(format!("gamedeck-motion-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&root).unwrap();
  for (index,bytes) in [include_bytes!("../tests/fixtures/two-frame.gif").as_slice(),include_bytes!("../tests/fixtures/two-frame.webp").as_slice()].iter().enumerate(){
   let info=inspect(bytes).unwrap();assert!(info.animated);assert_eq!((info.width,info.height),(8,6));
   let (path,info)=publish(bytes,&root.join(format!("{index}.png"))).unwrap();assert!(path.to_string_lossy().contains(".motion."));assert!(info.animated);assert_eq!(std::fs::read(&path).unwrap(),*bytes);std::fs::remove_file(path).unwrap();
  }
  assert!(!inspect(include_bytes!("../tests/fixtures/single-frame.gif")).unwrap().animated);std::fs::remove_dir(root).unwrap();
 }
 #[test]fn screenshots_lose_to_key_art_even_when_called_wallpaper(){assert!(classify("official key art").0>classify("8K gameplay screenshot wallpaper").0);assert!(classify("logo wallpaper").0<0);assert_eq!(classify("wallpaper").0,0);}
 #[test]fn truncated_animation_is_rejected(){assert!(inspect(b"GIF89a").is_err());assert!(inspect(b"GIF89a\x01\x00\x01\x00\x00\x00\x00\x3b").is_err());}
 #[test]fn frame_delays_enforce_eight_second_limit(){
  for original in [include_bytes!("../tests/fixtures/two-frame.gif").as_slice(),include_bytes!("../tests/fixtures/two-frame.webp").as_slice()] {
   let info=inspect(original).unwrap();assert_eq!(info.duration_ms,240);assert_eq!(info.frames,2);
   let mut bytes=original.to_vec();
   if bytes.starts_with(b"GIF"){let offsets:Vec<_>=bytes.windows(3).enumerate().filter(|(_,v)|*v==[0x21,0xf9,4]).map(|(i,_)|i).collect();for i in offsets{bytes[i+4]=0xf4;bytes[i+5]=1;}}
   else{let mut p=12;while p+8<=bytes.len(){let len=u32::from_le_bytes(bytes[p+4..p+8].try_into().unwrap()) as usize;if &bytes[p..p+4]==b"ANMF"{bytes[p+20]=0x88;bytes[p+21]=0x13;bytes[p+22]=0;}p+=8+len+len%2;}}
   assert!(inspect(&bytes).unwrap_err().contains("超过 8 秒"));
  }
 }
}

//! Screen-sized static display copies. Originals and database artwork stay intact.
use std::{path::{Path,PathBuf},hash::{Hash,Hasher}};
pub fn target_dimensions(w:u32,h:u32)->(u32,u32){
 if w==0||h==0{return (w,h);}
 let scale=(3840.0/w as f64).max(2160.0/h as f64).min(1.0);
 ((w as f64*scale).ceil() as u32,(h as f64*scale).ceil() as u32)
}
fn cache_target(source:&Path,artwork_root:&Path)->Result<(PathBuf,PathBuf),String>{
 let source=source.canonicalize().map_err(|_|"背景原图不存在")?;let root=artwork_root.canonicalize().map_err(|_|"素材目录不存在")?;
 if !source.starts_with(&root){return Err("背景不在素材目录内".into());}
 if source.to_string_lossy().contains(".motion."){return Err("大背景仅支持静态宣传图".into());}
 let metadata=std::fs::metadata(&source).map_err(|e|e.to_string())?;let mut hash=std::collections::hash_map::DefaultHasher::new();
 ("display-4k-jpeg94-v1",&source,metadata.len(),metadata.modified().ok()).hash(&mut hash);
 let path=artwork_root.join("display").join(format!("{:016x}.jpg",hash.finish()));Ok((source,path))
}
/// Read an existing display copy without decoding or generating any image.
pub fn cached_background(source:&Path,artwork_root:&Path)->Result<Option<PathBuf>,String>{let (_,path)=cache_target(source,artwork_root)?;Ok(path.is_file().then_some(path))}
pub fn background(source:&Path,artwork_root:&Path)->Result<PathBuf,String>{
 let (source,path)=cache_target(source,artwork_root)?;
 if path.is_file(){return Ok(path);}
 std::fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;
 let image=image::open(&source).map_err(|e|e.to_string())?;let (w,h)=target_dimensions(image.width(),image.height());
 let image=if image.width()!=w||image.height()!=h{image.resize_exact(w,h,image::imageops::FilterType::Lanczos3)}else{image};
 let mut matte=image::RgbaImage::from_pixel(w,h,image::Rgba([5,8,19,255]));image::imageops::overlay(&mut matte,&image.to_rgba8(),0,0);let rgb=image::DynamicImage::ImageRgba8(matte).into_rgb8();
 static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
 let pending=path.with_extension(format!("{}-{}.tmp",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
 let result=(||{let file=std::fs::File::create(&pending).map_err(|e|e.to_string())?;let mut writer=std::io::BufWriter::new(file);image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer,94).encode_image(&image::DynamicImage::ImageRgb8(rgb)).map_err(|e|e.to_string())?;use std::io::Write;writer.flush().map_err(|e|e.to_string())?;drop(writer);std::fs::rename(&pending,&path).map_err(|e|e.to_string())?;Ok(path)})();if result.is_err(){let _=std::fs::remove_file(pending);}result
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn cover_resolution_preserves_ratio_without_upscaling(){assert_eq!(target_dimensions(8192,4096),(4320,2160));assert_eq!(target_dimensions(7680,4320),(3840,2160));assert_eq!(target_dimensions(1920,620),(1920,620));}
 #[test]fn cache_lookup_never_generates_an_image(){let root=std::env::temp_dir().join(format!("gamedeck-cache-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&root).unwrap();let source=root.join("hero.png");image::RgbImage::new(160,90).save(&source).unwrap();assert!(cached_background(&source,&root).unwrap().is_none());assert!(!root.join("display").exists());let path=background(&source,&root).unwrap();assert_eq!(cached_background(&source,&root).unwrap(),Some(path.clone()));std::fs::remove_file(path).unwrap();std::fs::remove_file(source).unwrap();std::fs::remove_dir(root.join("display")).unwrap();std::fs::remove_dir(root).unwrap();}
 #[test]fn display_cache_reuses_file_and_leaves_original_untouched(){let root=std::env::temp_dir().join(format!("gamedeck-display-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&root).unwrap();let source=root.join("hero.png");image::RgbImage::from_pixel(160,90,image::Rgb([80,90,100])).save(&source).unwrap();let original=std::fs::read(&source).unwrap();let first=background(&source,&root).unwrap();let second=background(&source,&root).unwrap();assert_eq!(first,second);assert_eq!(std::fs::read(&source).unwrap(),original);assert_eq!(image::image_dimensions(&first).unwrap(),(160,90));std::fs::remove_file(first).unwrap();std::fs::remove_file(source).unwrap();std::fs::remove_dir(root.join("display")).unwrap();std::fs::remove_dir(root).unwrap();}
}

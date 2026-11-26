use std::path::Path;
#[repr(C)]struct Blob {len:u32,data:*mut u8}
#[link(name="crypt32")]extern "system" {
 fn CryptProtectData(i:*const Blob,d:*const u16,e:*const Blob,r:*mut std::ffi::c_void,p:*mut std::ffi::c_void,f:u32,o:*mut Blob)->i32;
 fn CryptUnprotectData(i:*const Blob,d:*mut *mut u16,e:*const Blob,r:*mut std::ffi::c_void,p:*mut std::ffi::c_void,f:u32,o:*mut Blob)->i32;
}
#[link(name="kernel32")]extern "system" {fn LocalFree(p:*mut std::ffi::c_void)->*mut std::ffi::c_void;}
pub(crate) fn crypt(bytes:&[u8],protect:bool)->Result<Vec<u8>,String>{
 let input=Blob{len:u32::try_from(bytes.len()).map_err(|_|"凭据过长")?,data:bytes.as_ptr() as *mut u8};let mut output=Blob{len:0,data:std::ptr::null_mut()};
 unsafe{let ok=if protect{CryptProtectData(&input,std::ptr::null(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null_mut(),1,&mut output)}else{CryptUnprotectData(&input,std::ptr::null_mut(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null_mut(),1,&mut output)};
 if ok==0{return Err("无法读取或加密 素材服务凭据，请重新保存 Key".into());}let result=std::slice::from_raw_parts(output.data,output.len as usize).to_vec();LocalFree(output.data.cast());Ok(result)}
}
fn load_named(root:&Path,name:&str)->Result<Option<String>,String>{let path=root.join("credentials").join(format!("{name}.key"));if !path.exists(){return Ok(None);}let bytes=std::fs::read(path).map_err(|_|"无法读取 素材服务凭据")?;Ok(Some(String::from_utf8(crypt(&bytes,false)?).map_err(|_|"素材服务凭据无效")?))}
fn save_named(root:&Path,key:&str,name:&str)->Result<(),String>{
 let dir=root.join("credentials");std::fs::create_dir_all(&dir).map_err(|_|"无法创建凭据目录")?;let path=dir.join(format!("{name}.key"));let key=key.trim();
 if key.is_empty(){if path.exists(){std::fs::remove_file(path).map_err(|_|"无法清除凭据")?;}return Ok(());}
 if key.len()>256||!key.chars().all(|c|c.is_ascii_alphanumeric()){return Err("素材服务 Key 格式无效".into());}
 let pending=dir.join(format!("{name}.pending"));std::fs::write(&pending,crypt(key.as_bytes(),true)?).map_err(|_|"无法保存凭据")?;std::fs::rename(pending,path).map_err(|_|"无法替换凭据")?;Ok(())
}
#[cfg(test)]mod tests{use super::*;#[test]fn dpapi_roundtrip_and_tamper(){let original=b"test-private-credential";let encrypted=crypt(original,true).unwrap();assert!(!encrypted.windows(original.len()).any(|w|w==original));assert_eq!(crypt(&encrypted,false).unwrap(),original);assert!(crypt(b"invalid",false).is_err());}}

pub fn load(root:&Path)->Result<Option<String>,String>{load_named(root,"alphacoders")}
pub fn save(root:&Path,key:&str)->Result<(),String>{save_named(root,key,"alphacoders")}
pub fn load_sgdb(root:&Path)->Result<Option<String>,String>{load_named(root,"steamgriddb")}
pub fn save_sgdb(root:&Path,key:&str)->Result<(),String>{save_named(root,key,"steamgriddb")}
#[cfg(test)]mod provider_tests {
 use super::*;
 #[test]fn credentials_stay_separate_and_support_replace_and_clear(){
  let root=std::env::temp_dir().join(format!("gamedeck-credentials-{}",uuid::Uuid::new_v4()));
  save(&root,"alphaTest").unwrap();save_sgdb(&root,"sgdbTest").unwrap();
  assert_eq!(load(&root).unwrap().as_deref(),Some("alphaTest"));
  save_sgdb(&root,"replacementTest").unwrap();assert_eq!(load_sgdb(&root).unwrap().as_deref(),Some("replacementTest"));
  save_sgdb(&root,"").unwrap();assert!(load_sgdb(&root).unwrap().is_none());assert_eq!(load(&root).unwrap().as_deref(),Some("alphaTest"));
  save(&root,"").unwrap();std::fs::remove_dir(root.join("credentials")).unwrap();std::fs::remove_dir(root).unwrap();
 }
}

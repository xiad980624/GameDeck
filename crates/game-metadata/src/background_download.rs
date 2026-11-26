//! Bounded original-image downloads, including slow CDN connections.
use std::time::Duration;
const MAX:usize=24_000_000;

fn safe_error(error:reqwest::Error)->String{
    if error.is_timeout(){"原图传输超时（已允许最多 180 秒）".into()}else{"原图连接或传输中断".into()}
}
fn validate(response:&reqwest::Response,host:&str)->Result<(),String>{
    if response.url().scheme()!="https"||!response.url().host_str().is_some_and(|final_host|final_host==host||(host.ends_with(".gamespress.com")&&(final_host.ends_with(".blob.core.windows.net")||final_host.ends_with(".gamespress.com")))){return Err("原图跳转来源未验证".into());}Ok(())
}
async fn limited(mut response:reqwest::Response,max:usize)->Result<Vec<u8>,String>{
    let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(safe_error)?{
        if bytes.len()+chunk.len()>max{return Err("原图数据超过允许大小".into());}bytes.extend_from_slice(&chunk);
    }Ok(bytes)
}
pub async fn original(url:&str,host:&str)->Result<Vec<u8>,String>{
    #[cfg(windows)]
    if host.ends_with(".steamstatic.com"){
        let (url,host)=(url.to_owned(),host.to_owned());
        return tokio::task::spawn_blocking(move||windows_original(&url,&host)).await.map_err(|_|"Windows 原图下载任务失败".to_string())?;
    }
    original_http(url,host).await
}
async fn original_http(url:&str,host:&str)->Result<Vec<u8>,String>{
    let client=reqwest::Client::builder().http1_only().redirect(reqwest::redirect::Policy::limited(3))
        .timeout(Duration::from_secs(180)).default_headers({let mut headers=reqwest::header::HeaderMap::new();headers.insert(reqwest::header::ACCEPT,reqwest::header::HeaderValue::from_static("image/png,image/jpeg,image/webp;q=0.9,*/*;q=0.1"));headers}).user_agent("GameDeck/1.0 (personal game library)").build().map_err(|_|"原图下载器初始化失败")?;
    let head=client.head(url).timeout(Duration::from_secs(12)).send().await;
    if let Ok(response)=head {
        validate(&response,host)?;
        if response.status().is_success(){
            if let Some(total)=response.content_length(){
                if total>MAX as u64{return Err("原图超过 24 MB".into());}
                let ranges=response.headers().get(reqwest::header::ACCEPT_RANGES).and_then(|h|h.to_str().ok()).is_some_and(|v|v.eq_ignore_ascii_case("bytes"));
                if ranges&&total>=262144{
                    // Small independent ranges finish on slow connections; never enlarge or transcode the original.
                    let mut tasks=tokio::task::JoinSet::new();let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(4));
                    for start in (0..total).step_by(131072){let end=(start+131071).min(total-1);let (client,url,host,limit)=(client.clone(),url.to_string(),host.to_string(),limit.clone());
                        tasks.spawn(async move {let _permit=limit.acquire_owned().await.map_err(|_|"分段下载被取消")?;
                            let r=client.get(url).header(reqwest::header::RANGE,format!("bytes={start}-{end}")).header(reqwest::header::ACCEPT_ENCODING,"identity").timeout(Duration::from_secs(45)).send().await.map_err(safe_error)?;
                            validate(&r,&host)?;let expected=format!("bytes {start}-{end}/{total}");
                            if r.status()!=reqwest::StatusCode::PARTIAL_CONTENT||r.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v|v.to_str().ok())!=Some(expected.as_str()){return Err("CDN 未返回正确的原始字节范围".into());}
                            let bytes=limited(r,(end-start+1) as usize).await?;if bytes.len()!=(end-start+1) as usize{return Err("原图分段长度不完整".into());}Ok::<_,String>((start,bytes))
                        });
                    }
                    let collect=async {let mut parts=std::collections::BTreeMap::new();while let Some(task)=tasks.join_next().await{let (start,bytes)=task.map_err(|_|"原图分段任务失败")??;parts.insert(start,bytes);}let bytes:Vec<_>=parts.into_values().flatten().collect();if bytes.len()!=total as usize{return Err("原图总长度不完整".into());}Ok::<_,String>(bytes)};
                    return tokio::time::timeout(Duration::from_secs(180),collect).await.map_err(|_|"原图分段传输超过 180 秒".to_string())?;
                }
            }
        }
    }
    let response=client.get(url).timeout(Duration::from_secs(180)).send().await.map_err(safe_error)?;validate(&response,host)?;
    if !response.status().is_success(){return Err(format!("原图 HTTP {}",response.status().as_u16()));}
    if response.content_length().is_some_and(|n|n>MAX as u64){return Err("原图超过 24 MB".into());}
    limited(response,MAX).await
}

/// Public images only; Windows validates the certificate chain and hostname.
/// Best-effort revocation permits certificates without a reachable revocation endpoint,
/// but never permits an invalid certificate or an untrusted issuer.
#[cfg(windows)]
fn windows_original(url:&str,host:&str)->Result<Vec<u8>,String>{
    use std::{process::{Command,Stdio},os::windows::process::CommandExt};
    let parsed=reqwest::Url::parse(url).map_err(|_|"原图地址无效")?;
    if parsed.scheme()!="https"||parsed.host_str()!=Some(host)||!host.ends_with(".steamstatic.com"){return Err("原图来源无效".into());}
    #[link(name="kernel32")]extern "system"{fn GetSystemDirectoryW(path:*mut u16,size:u32)->u32;}
    let mut path=[0u16;32768];let size=unsafe{GetSystemDirectoryW(path.as_mut_ptr(),path.len() as u32)} as usize;
    if size==0||size>=path.len(){return Err("无法定位 Windows 原生下载器".into());}
    let curl=std::path::PathBuf::from(String::from_utf16_lossy(&path[..size])).join("curl.exe");
    let temp=std::env::temp_dir().join(format!("gamedeck-public-image-{}-{}.tmp",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
    struct Temporary(std::path::PathBuf);impl Drop for Temporary{fn drop(&mut self){let _=std::fs::remove_file(&self.0);}}let _temporary=Temporary(temp.clone());
    let output=Command::new(curl).creation_flags(0x08000000).args(["--silent","--show-error","--fail","--location","--max-redirs","3","--proto","=https","--proto-redir","=https","--ssl-revoke-best-effort","--connect-timeout","10","--max-time","40","--max-filesize","24000000","--output"]).arg(&temp).args(["--write-out","%{http_code}\n%{url_effective}",url]).stdin(Stdio::null()).output().map_err(|_|"Windows 原生图片下载器不可用")?;
    let status=String::from_utf8_lossy(&output.stdout);let mut lines=status.lines();let http=lines.next().unwrap_or("000");let effective=lines.next().unwrap_or("");
    if !output.status.success(){return Err(if http!="000"{format!("原图 HTTP {http}")}else{format!("Windows 原图连接失败（下载器代码 {}）",output.status.code().unwrap_or(-1))});}
    let final_url=reqwest::Url::parse(effective).map_err(|_|"原图跳转地址无效")?;
    if final_url.scheme()!="https"||final_url.host_str()!=Some(host){return Err("原图跳转来源未验证".into());}
    let size=std::fs::metadata(&temp).map_err(|e|e.to_string())?.len();if size==0||size>MAX as u64{return Err("原图大小不符合限制".into());}
    std::fs::read(temp).map_err(|e|e.to_string())
}

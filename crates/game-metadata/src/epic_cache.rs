//! Import only public Epic CDN image entries from the launcher's Chromium cache.
//! No account, cookie or authentication databases are opened.
use crate::{Artwork, GameRef};
use std::{fs, path::{Path,PathBuf}};

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> { Some(u32::from_le_bytes(bytes.get(offset..offset+4)?.try_into().ok()?)) }
pub(crate) fn stream(root: &Path, address: u32, size: usize) -> Option<Vec<u8>> {
    if address & 0x8000_0000 == 0 || size == 0 || size > 16_000_000 { return None; }
    let kind = (address >> 28) & 7;
    if kind == 0 { return fs::read(root.join(format!("f_{:06x}",address & 0x0fff_ffff))).ok()?.get(..size).map(|s| s.to_vec()); }
    let block = match kind { 2 => 256, 3 => 1024, 4 => 4096, _ => return None };
    let count = ((address >> 24) & 3) as usize + 1;
    if size > block * count { return None; }
    let bytes = fs::read(root.join(format!("data_{}",(address >> 16) & 255))).ok()?;
    let start = 8192 + (address & 65535) as usize * block;
    bytes.get(start..start.checked_add(size)?).map(|s| s.to_vec())
}
pub(crate) fn dimensions(bytes: &[u8]) -> Option<(u32,u32,&'static str)> {
    if bytes.get(4..12) == Some(b"ftypavif") || bytes.get(4..12) == Some(b"ftypavis") {
        // AVIF's image spatial extents property contains the decoded width and height.
        for (index, _) in bytes.windows(4).enumerate().filter(|(_,s)| *s == b"ispe") {
            let width = u32::from_be_bytes(bytes.get(index+8..index+12)?.try_into().ok()?);
            let height = u32::from_be_bytes(bytes.get(index+12..index+16)?.try_into().ok()?);
            if width > 0 && height > 0 && width <= 10000 && height <= 10000 { return Some((width,height,"avif")); }
        }
        return None;
    }
    let image = image::load_from_memory(bytes).ok()?;
    let extension = match image::guess_format(bytes).ok()? { image::ImageFormat::Png => "png", image::ImageFormat::Jpeg => "jpg", image::ImageFormat::WebP => "webp", _ => return None };
    Some((image.width(),image.height(),extension))
}
pub fn cache_roots() -> Vec<PathBuf> {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else { return Vec::new() };
    let mut roots = vec![PathBuf::from(local).join("EpicGamesLauncher/Saved")];
    if let Some(profile) = std::env::var_os("USERPROFILE") { roots.push(PathBuf::from(profile).join("AppData/Local/EpicGamesLauncher/Saved")); }
    let mut caches: Vec<_> = roots.into_iter().flat_map(|root| fs::read_dir(root).into_iter().flatten().flatten())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("webcache"))
        .flat_map(|entry| [entry.path().join("Cache"),entry.path().join("Cache/Cache_Data")])
        .filter(|path| path.join("data_1").is_file()).collect();
    caches.sort(); caches.dedup(); caches
}
fn public_url(key: &[u8], namespace: &str) -> Option<String> {
    if namespace.is_empty() || !namespace.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') { return None; }
    let text = std::str::from_utf8(key).ok()?;
    let start = text.find("https://cdn")?;
    let url = text[start..].split(['\0',' ','\n','\r']).next()?;
    let host = url.strip_prefix("https://")?.split('/').next()?;
    if !["cdn1.epicgames.com","cdn2.epicgames.com","cdn.epicgames.com","cdn.unrealengine.com","cdn2.unrealengine.com"].contains(&host) { return None; }
    if !url.split('?').next()?.split('/').any(|part| part.eq_ignore_ascii_case(namespace)) { return None; }
    Some(url.to_string())
}
pub fn import(game: &GameRef, namespace: &str, caches: &[PathBuf], output: &Path) -> Result<Vec<Artwork>,String> {
    let mut images = Vec::new();
    for cache in caches {
        let Ok(bytes) = fs::read(cache.join("data_1")) else { continue };
        // EntryStore is allocated in 256-byte blocks after the block-file header.
        for offset in (8192..bytes.len().saturating_sub(96)).step_by(256) {
            if u32_at(&bytes,offset+20) != Some(0) { continue; } // only live entries
            let Some(key_len) = u32_at(&bytes,offset+32).filter(|n| *n > 0 && *n <= 4096) else { continue };
            if u32_at(&bytes,offset+36) != Some(0) { continue; } // external keys are not needed for images
            let Some(key) = bytes.get(offset+96..offset+96+key_len as usize) else { continue };
            let Some(url) = public_url(key,namespace) else { continue };
            let Some(data) = u32_at(&bytes,offset+44).zip(u32_at(&bytes,offset+60)).and_then(|(size,address)| stream(cache,address,size as usize)) else { continue };
            let Some((width,height,extension)) = dimensions(&data) else { continue };
            if width < 200 || height < 150 || width > 10000 || height > 10000 { continue; }
            images.push((width,height,extension,data,url));
        }
    }
    let mut assets = Vec::new();
    for kind in ["poster","hero","landscape"] {
        let best = images.iter().filter(|(w,h,_,_,_)| if kind == "poster" { h > w } else { *w as f32 / *h as f32 > 1.4 })
            .max_by_key(|(w,h,_,_,_)| *w as u64 * *h as u64);
        if let Some((width,height,extension,data,_)) = best {
            let dir = output.join(kind); fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            use std::hash::{Hash,Hasher};let mut hash=std::collections::hash_map::DefaultHasher::new();data.hash(&mut hash);
            let path = dir.join(format!("{}-epic-{:016x}.{extension}",game.id,hash.finish()));
            if !path.exists() { fs::write(&path,data).map_err(|e| e.to_string())?; }
            assets.push(Artwork { kind,path,source:"epic-local",width:*width,height:*height });
        }
    }
    Ok(assets)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn only_exact_public_namespace_matches() {
        assert!(public_url(b"https://cdn1.epicgames.com/catnip/item/art.jpg?w=360","catnip").is_some());
        assert!(public_url(b"https://cdn1.epicgames.com/catnip2/item/art.jpg","catnip").is_none());
        assert!(public_url(b"https://cdn1.epicgames.com.evil.test/catnip/a.jpg","catnip").is_none());
        assert!(public_url(b"https://account.epicgames.com/catnip","catnip").is_none());
    }
    #[test] fn rejects_uninitialized_or_unbounded_cache_streams() {
        assert!(stream(Path::new("."),0,100).is_none());
        assert!(stream(Path::new("."),0x80000001,20_000_000).is_none());
        assert!(stream(Path::new("."),0xa0010001,4096).is_none());
    }
}

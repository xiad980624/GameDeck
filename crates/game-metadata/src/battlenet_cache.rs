//! Public artwork only, from Battle.net's browser cache. No login/SQLite stores are read.
use crate::{Artwork,GameRef};
use std::{fs,path::{Path,PathBuf}};
fn number(bytes:&[u8],at:usize)->Option<u32>{Some(u32::from_le_bytes(bytes.get(at..at+4)?.try_into().ok()?))}
fn roots()->Vec<PathBuf>{
    let mut profiles=Vec::new();if let Some(p)=std::env::var_os("LOCALAPPDATA"){profiles.push(PathBuf::from(p));}
    if let Some(p)=std::env::var_os("USERPROFILE"){profiles.push(PathBuf::from(p).join("AppData/Local"));}
    let mut caches=Vec::new();
    for root in profiles {if let Ok(entries)=fs::read_dir(root.join("Battle.net/BrowserCaches")){for e in entries.flatten(){let p=e.path().join("Cache/Cache_Data");if p.join("data_1").is_file(){caches.push(p);}}}}
    caches.sort();caches.dedup();caches
}
fn matching_url(key:&[u8],id:&str)->Option<String>{
    let text=std::str::from_utf8(key).ok()?;let start=text.find("https://")?;
    let url=text[start..].split(['\0','\r','\n',' ']).next()?;
    let host=url.strip_prefix("https://")?.split('/').next()?;
    if !["bnetproduct-a.akamaihd.net","blz-contentstack-images.akamaized.net","contentstack-images.blzstatic.cn","blznav.akamaized.net"].contains(&host){return None;}
    let name=url.split('?').next()?.rsplit('/').next()?.to_ascii_lowercase();
    // Product artwork is stable. News, season banners and shop bundles must not
    // become a game's identity merely because their cached resolution is larger.
    let matches=match id {
        "diablo_iii"|"diablo_iii_cn"=>["diablo3.png","7627-11730.png","d3_logo_cn.png"].contains(&name.as_str()),
        "diablo_iv"=>name=="2022_09_diablo_d4_shop_product_asset_base_1920x1080_ts01.png"||name=="d4_logo.png"||name=="diablo-4-cn-logo.png",
        _=>false,
    };let background=id.starts_with("diablo_iii")&&name=="dia_d3_s38_bnet_keyart_1920x1080.png";
    (matches||background).then(||url.into())
}
pub fn product_reference_urls(game:&GameRef)->Vec<String>{
    if game.provider!="battlenet"{return Vec::new();}
    let mut urls=std::collections::BTreeSet::new();
    for root in roots(){let Ok(bytes)=fs::read(root.join("data_1"))else{continue};
        for at in (8192..bytes.len().saturating_sub(96)).step_by(256){
            if number(&bytes,at+20)!=Some(0)||number(&bytes,at+36)!=Some(0){continue;}
            let Some(length)=number(&bytes,at+32).filter(|n|*n>0&&*n<4096)else{continue};
            let Some(url)=bytes.get(at+96..at+96+length as usize).and_then(|key|matching_url(key,&game.external_id))else{continue};
            let name=url.split('?').next().unwrap_or(&url).rsplit('/').next().unwrap_or("").to_lowercase();
            let seasonal=name.split('_').any(|part|part.strip_prefix('s').is_some_and(|s|!s.is_empty()&&s.chars().all(|c|c.is_ascii_digit())));
            if !name.contains("logo")&&!seasonal{urls.insert(url);}
        }
    }
    urls.into_iter().collect()
}
pub fn import(game:&GameRef,output:&Path)->Result<Vec<Artwork>,String>{
    if game.provider!="battlenet"{return Ok(Vec::new());}let mut images=Vec::new();
    for root in roots(){let Ok(bytes)=fs::read(root.join("data_1"))else{continue};
        for at in (8192..bytes.len().saturating_sub(96)).step_by(256){
            if number(&bytes,at+20)!=Some(0)||number(&bytes,at+36)!=Some(0){continue;}
            let Some(length)=number(&bytes,at+32).filter(|n|*n>0&&*n<4096)else{continue};
            let Some(url)=bytes.get(at+96..at+96+length as usize).and_then(|key|matching_url(key,&game.external_id))else{continue};
            let Some(data)=number(&bytes,at+44).zip(number(&bytes,at+60)).and_then(|(size,address)|crate::epic_cache::stream(&root,address,size as usize))else{continue};
            let Some((w,h,extension))=crate::epic_cache::dimensions(&data)else{continue};
            if w>=200&&h>=100&&w<=10000&&h<=10000{images.push((w,h,extension,data,url.to_ascii_lowercase()));}
        }
    }
    let mut assets=Vec::new();
    for kind in ["poster","hero","landscape","logo"]{
        let best=images.iter().filter(|(w,h,_,_,url)|if kind=="logo"{url.contains("logo")}else{
                let seasonal=url.split('?').next().unwrap_or(url).ends_with("dia_d3_s38_bnet_keyart_1920x1080.png");
                !url.contains("logo")&&if kind=="hero"{*w as f32 / *h as f32>1.4}else{!seasonal}
            })
            .max_by_key(|(w,h,_,_,url)|((kind=="poster"&&h>w) as u8,url.contains("asset_base") as u8,*w as u64 * *h as u64));
        if let Some((width,height,extension,data,_))=best{
            let dir=output.join(kind);fs::create_dir_all(&dir).map_err(|e|e.to_string())?;// A new cache selection gets a new URL, so WebView cannot reuse the old bitmap.
            use std::hash::{Hash,Hasher};
            let mut hash=std::collections::hash_map::DefaultHasher::new();data.hash(&mut hash);
            let path=dir.join(format!("{}-battlenet-v2-{:016x}.{extension}",game.id,hash.finish()));
            if !path.exists(){fs::write(&path,data).map_err(|e|e.to_string())?;}
            assets.push(Artwork {kind,path,source:"battlenet-local",width:*width,height:*height});
        }
    }Ok(assets)
}
#[cfg(test)]mod tests{use super::*;#[test]fn excludes_unrelated_game_and_promotional_assets(){
    assert!(matching_url(b"https://contentstack-images.blzstatic.cn/x/DIABLO3.png","diablo_iii_cn").is_some());
    assert!(matching_url(b"https://contentstack-images.blzstatic.cn/x/DIABLO3.png","diablo_iv").is_none());
    assert!(matching_url(b"https://contentstack-images.blzstatic.cn/x/D4_Battle_Pass.jpg","diablo_iv").is_none());
    assert!(matching_url(b"https://evil.test/x/DIABLO3.png","diablo_iii").is_none());
}}

#[cfg(test)]mod product_art_tests {
    use super::*;
    #[test]fn rejects_season_and_store_banners_even_with_game_name(){
        for name in ["DIABLOIV_Desktop-1600x500.png","Diablo4_Standard_1920x1080_(1).png","DIA_D4_FamilyBundle_Refeature_BnetShop_BrowsingCard_1920x1080.png"]{
            let url=format!("https://contentstack-images.blzstatic.cn/v3/assets/x/{name}");
            assert!(matching_url(url.as_bytes(),"diablo_iii").is_none());
            assert!(matching_url(url.as_bytes(),"diablo_iv").is_none());
        }
    }
    #[test]fn recognizes_verified_product_covers_without_cross_matching(){
        let d3=b"https://contentstack-images.blzstatic.cn/v3/assets/x/7627-11730.png?image_process=resize,w_572";
        assert!(matching_url(d3,"diablo_iii").is_some());
        assert!(matching_url(d3,"diablo_iii_cn").is_some());
        assert!(matching_url(d3,"diablo_iv").is_none());
        let d4=b"https://contentstack-images.blzstatic.cn/v3/assets/x/2022_09_Diablo_D4_Shop_Product_Asset_Base_1920x1080_TS01.png";
        assert!(matching_url(d4,"diablo_iv").is_some());
        assert!(matching_url(d4,"diablo_iii").is_none());
    }
}

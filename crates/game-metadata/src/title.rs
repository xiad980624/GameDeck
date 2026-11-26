//! Independent title retrieval: never wait for background scraping to obtain a logo.
use super::*;

fn urls(value:&serde_json::Value,id:u64)->Vec<String>{
    let mut urls=Vec::new();
    if let Some(items)=value["response"]["store_items"].as_array(){for item in items{
        if item["id"].as_u64()!=Some(id){continue;}
        for assets in [&item["assets"],&item["library_assets"]]{
            let Some(format)=assets["asset_url_format"].as_str()else{continue};
            for field in ["library_logo_2x","library_logo","logo_2x","logo"]{
                let Some(file)=assets[field].as_str()else{continue};
                if file.contains("..")||file.contains(':')||!file.ends_with(".png"){continue;}
                let relative=format.replace("${FILENAME}",file);
                let url=if relative.starts_with("https://"){relative}else{format!("https://shared.fastly.steamstatic.com/{}{}",if relative.starts_with("steam/apps/"){"store_item_assets/"}else{""},relative)};
                if let Ok(parsed)=reqwest::Url::parse(&url){if parsed.scheme()=="https"&&parsed.host_str().is_some_and(|h|h.ends_with(".steamstatic.com"))&&parsed.path().contains(&format!("/apps/{id}/")){urls.push(url);}}
            }
        }
    }}urls
}
impl Scraper{
    pub async fn scrape_title(&self,game:&GameRef,root:&Path,confirmed_sgdb:Option<u64>,confirmed_steam:Option<u64>)->Result<Option<Artwork>,String>{
        tokio::time::timeout(Duration::from_secs(65),self.title_impl(game,root,confirmed_sgdb,confirmed_steam)).await.map_err(|_|"Logo 获取超时，稍后自动重试".to_string())?
    }
    async fn title_impl(&self,game:&GameRef,root:&Path,confirmed_sgdb:Option<u64>,confirmed_steam:Option<u64>)->Result<Option<Artwork>,String>{
        let official_error=match tokio::time::timeout(Duration::from_secs(15),self.official_title(game,root,confirmed_sgdb,confirmed_steam)).await{
            Ok(Ok(Some(asset)))=>return Ok(Some(asset)),Ok(Ok(None))=>None,Ok(Err(error))=>Some(error),Err(_)=>Some("Steam Logo 来源超时，备用来源也未返回可用图片".into())
        };
        let directory=root.join("logo");std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
        let path=directory.join(format!("{}-original.png",game.id));
        if let Some(key)=&self.key{
            let id=if confirmed_sgdb.is_some(){confirmed_sgdb}else{unique_exact(self.candidates(&game.name).await?)};
            if let Some(id)=id{
                let mut errors=Vec::new();
                for url in self.sgdb_asset_candidates(id,"logos",key,"logo").await?{
                    match self.fetch_sgdb_media(&url,&path,"logo").await{
                        Ok(Some(asset)) if branding::usable_logo(&asset.path)=>return Ok(Some(asset)),
                        Ok(Some(_))=>errors.push("候选不是可用的透明 Logo".into()),Ok(None)=>(),Err(error)=>errors.push(error),
                    }
                }
                if !errors.is_empty(){return Err(format!("Logo 候选均未成功：{}",errors.join("；")));}
            }
        }
        if let Some(error)=official_error{return Err(error);}
        Ok(None)
    }
    async fn official_title(&self,game:&GameRef,root:&Path,confirmed_sgdb:Option<u64>,confirmed_steam:Option<u64>)->Result<Option<Artwork>,String>{
        let steam=if game.provider=="steam"{game.external_id.parse().ok()}else if confirmed_steam.is_some(){confirmed_steam}else{self.steam_platform_identity(&game.name,confirmed_sgdb,root,&game.id).await?};
        let steam=if steam.is_some(){steam}else{unique_exact(self.steam_candidates(&artwork_search_name(&game.name)).await?)};
        let directory=root.join("logo");std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
        let path=directory.join(format!("{}-official.png",game.id));
        if let Some(id)=steam{
            for url in crate::platform_identity::cached_steam_assets(root,&game.id,&game.name,confirmed_sgdb,id,"logo"){
                if let Ok(Some((width,height)))=self.fetch_artwork(&url,&path,1280,"logo").await{if branding::usable_logo(&path){return Ok(Some(Artwork{kind:"logo",path,source:"title-logo-official",width,height}));}}
            }
            let input=serde_json::json!({"ids":[{"appid":id}],"context":{"language":"english","country_code":"US"},"data_request":{"include_assets":true}}).to_string();
            let mut sources=Vec::new();
            if let Ok(response)=self.client.get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/").query(&[("input_json",input)]).send().await{if let Ok(value)=response.json::<serde_json::Value>().await{sources.extend(urls(&value,id));}}
            for base in ["https://shared.fastly.steamstatic.com/store_item_assets/steam/apps","https://shared.fastly.steamstatic.com/steam/apps"]{for file in ["logo_2x.png","logo.png","library_logo.png"]{sources.push(format!("{base}/{id}/{file}"));}}
            let mut failure=None;
            for url in sources{match self.fetch_artwork(&url,&path,1280,"logo").await{
                Ok(Some((width,height))) if branding::usable_logo(&path)=>return Ok(Some(Artwork{kind:"logo",path,source:"title-logo-official",width,height})),
                Err(error)=>failure=Some(error),_=>()
            }}
            if let Some(error)=failure{return Err(format!("Steam Logo 下载失败：{error}"));}
        }
        Ok(None)
    }
}

/// Trim transparent padding for consistent left/bottom placement, preserving the source file.
pub fn display(asset:&Artwork)->Result<Artwork,String>{
    if asset.kind!="logo"{return Ok(asset.clone());}
    let bytes=std::fs::read(&asset.path).map_err(|e|e.to_string())?;
    let rgba=image::load_from_memory(&bytes).map_err(|e|e.to_string())?.to_rgba8();
    let (mut l,mut t,mut r,mut b)=(rgba.width(),rgba.height(),0,0);
    for(x,y,p)in rgba.enumerate_pixels(){if p[3]>16{l=l.min(x);t=t.min(y);r=r.max(x);b=b.max(y);}}
    if r<=l||b<=t{return Err("Logo 没有可见像素".into());}
    use std::hash::{Hash,Hasher};let mut hash=std::collections::hash_map::DefaultHasher::new();bytes.hash(&mut hash);
    let stem=asset.path.file_stem().unwrap_or_default().to_string_lossy();let stem=stem.split("-trimmed-").next().unwrap_or(&stem);
    let path=asset.path.with_file_name(format!("{stem}-trimmed-{:016x}.png",hash.finish()));
    if !path.is_file(){image::imageops::crop_imm(&rgba,l,t,r-l+1,b-t+1).to_image().save(&path).map_err(|e|e.to_string())?;}
    Ok(Artwork{path,width:r-l+1,height:b-t+1,..asset.clone()})
}
#[cfg(test)]mod tests{use super::*;
    #[test]fn metadata_cannot_return_a_different_games_logo(){
        let value=serde_json::json!({"response":{"store_items":[{"id":42,"assets":{"asset_url_format":"steam/apps/42/hash/${FILENAME}","library_logo":"logo.png"}}]}});
        assert_eq!(urls(&value,42),vec!["https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/42/hash/logo.png"]);
        assert!(urls(&value,43).is_empty());
    }
}

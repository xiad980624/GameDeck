use super::*;

fn mapped_id(value:&serde_json::Value,name:&str)->Option<u64>{
    let data=&value["data"];
    if value["success"]!=true||data["verified"]!=true||normalize(&artwork_search_name(data["name"].as_str()?))!=normalize(&artwork_search_name(name)){return None;}
    let mut ids:Vec<_>=data["external_platform_data"]["steam"].as_array()?.iter()
        .filter_map(|item|item["id"].as_str().and_then(|s|s.parse::<u64>().ok()).or_else(||item["id"].as_u64())).filter(|id|*id>0).collect();
    ids.sort_unstable();ids.dedup();if ids.len()==1{Some(ids[0])}else{None}
}
/// Reuse the verified platform's actual hashed asset paths, not guessed filenames.
pub(crate) fn cached_steam_assets(root:&Path,game_id:&str,name:&str,confirmed:Option<u64>,steam_id:u64,kind:&str)->Vec<String>{
    let load=||->Option<Vec<String>>{
        let bytes=std::fs::read(root.join("identity").join(format!("{game_id}-steam.json"))).ok()?;
        let cache:serde_json::Value=serde_json::from_slice(&bytes).ok()?;
        let response=&cache["response"];
        if cache["requested_name"]!=artwork_search_name(name)||confirmed.is_some_and(|id|response["data"]["id"].as_u64()!=Some(id))||mapped_id(response,name)!=Some(steam_id){return None;}
        let item=response["data"]["external_platform_data"]["steam"].as_array()?.iter().find(|item|item["id"].as_str().and_then(|s|s.parse::<u64>().ok()).or_else(||item["id"].as_u64())==Some(steam_id))?;
        let field=match kind{"logo"=>"library_logo","hero"=>"library_hero",_=>return None};let metadata=&item["metadata"];let full=&metadata[format!("{field}_full")];let mut files=Vec::new();
        for size in ["image2x","image"]{
            let values=&full[size];let file=values["english"].as_str().or_else(||values.as_object()?.values().find_map(|v|v.as_str()));
            if let Some(file)=file{files.push(file.to_owned());}
        }
        if files.is_empty()&&metadata[field].as_str().is_some_and(|language|!language.is_empty()){
            files.extend(if kind=="logo"{vec!["logo_2x.png".into(),"logo.png".into()]}else{vec!["library_hero_2x.jpg".into(),"library_hero.jpg".into()]});
        }
        let mut urls=Vec::new();
        for file in files{
            if file.starts_with('/')||file.contains([':', '?', '#', '\\'])||file.split('/').any(|part|part.is_empty()||part==".."||part=="."){continue;}
            let suffix=if kind=="logo"{file.ends_with("logo.png")||file.ends_with("logo_2x.png")}else{file.ends_with("library_hero.jpg")||file.ends_with("library_hero_2x.jpg")};if !suffix{continue;}
            for base in ["store_item_assets/steam/apps","steam/apps"]{let url=format!("https://shared.fastly.steamstatic.com/{base}/{steam_id}/{file}");if !urls.contains(&url){urls.push(url);}}
        }Some(urls)
    };load().unwrap_or_default()
}
impl Scraper{
    pub async fn steam_platform_identity(&self,name:&str,confirmed:Option<u64>,root:&Path,game_id:&str)->Result<Option<u64>,String>{
        let dir=root.join("identity");let path=dir.join(format!("{game_id}-steam.json"));
        if let Ok(bytes)=std::fs::read(&path){if let Ok(v)=serde_json::from_slice::<serde_json::Value>(&bytes){if v["requested_name"]==artwork_search_name(name)&&confirmed.map_or(true,|id|v["response"]["data"]["id"].as_u64()==Some(id)){if let Some(id)=mapped_id(&v["response"],name){return Ok(Some(id));}}}}
        let Some(key)=&self.key else{return Ok(None);};
        let sgdb=if let Some(id)=confirmed{Some(id)}else{unique_exact(self.candidates(name).await?)};let Some(sgdb)=sgdb else{return Ok(None);};
        let response=self.client.get(format!("https://www.steamgriddb.com/api/v2/games/id/{sgdb}")).query(&[("platformdata","steam")]).bearer_auth(key).send().await.map_err(|_|"平台编号映射连接失败")?;
        if !response.status().is_success(){return Err(format!("平台编号映射 HTTP {}",response.status().as_u16()));}
        let value=response.json::<serde_json::Value>().await.map_err(|_|"平台编号映射格式无效")?;
        let id=mapped_id(&value,name);
        if id.is_some(){std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;let snapshot=serde_json::json!({"requested_name":artwork_search_name(name),"response":value});std::fs::write(path,serde_json::to_vec(&snapshot).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;}
        Ok(id)
    }
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn hashed_platform_assets_require_exact_verified_identity_and_safe_paths(){
    let root=std::env::temp_dir().join(format!("gamedeck-platform-assets-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(root.join("identity")).unwrap();
    let mut value=serde_json::json!({"requested_name":"Base Game","response":{"success":true,"data":{"id":1,"verified":true,"name":"Base Game","external_platform_data":{"steam":[{"id":"42","metadata":{"library_logo_full":{"image":{"english":"abc123/logo.png"}}}}]}}}});
    let path=root.join("identity/test-steam.json");std::fs::write(&path,value.to_string()).unwrap();
    assert!(cached_steam_assets(&root,"test","Base Game",Some(1),42,"logo")[0].ends_with("/42/abc123/logo.png"));
    assert!(cached_steam_assets(&root,"test","Base Game Demo",Some(1),42,"logo").is_empty());assert!(cached_steam_assets(&root,"test","Base Game",Some(2),42,"logo").is_empty());assert!(cached_steam_assets(&root,"test","Base Game",Some(1),43,"logo").is_empty());
    value["response"]["data"]["external_platform_data"]["steam"][0]["metadata"]["library_logo_full"]["image"]["english"]="../other/logo.png".into();std::fs::write(path,value.to_string()).unwrap();assert!(cached_steam_assets(&root,"test","Base Game",Some(1),42,"logo").is_empty());std::fs::remove_dir_all(root).unwrap();
 }
 #[test]fn mapping_requires_verified_exact_game_and_unique_platform_id(){let mut v=serde_json::json!({"success":true,"data":{"verified":true,"name":"Base Game","external_platform_data":{"steam":[{"id":"42"}]}}});assert_eq!(mapped_id(&v,"Base Game"),Some(42));assert_eq!(mapped_id(&v,"Base Game Demo"),None);v["data"]["verified"]=false.into();assert_eq!(mapped_id(&v,"Base Game"),None);v["data"]["verified"]=true.into();v["data"]["external_platform_data"]["steam"]=serde_json::json!([{"id":"42"},{"id":"43"}]);assert_eq!(mapped_id(&v,"Base Game"),None);}
}

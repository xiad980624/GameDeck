use super::*;
use serde_json::{Value,json};

pub fn missing_artwork(game:&Game)->Vec<&'static str>{
    [("poster",&game.poster),("hero",&game.hero),("landscape",&game.landscape),("logo",&game.logo)].into_iter()
        .filter(|(_,path)|!path.as_ref().is_some_and(|p|Path::new(p).is_file())).map(|(kind,_)|kind).collect()
}
impl Storage{
    /// Resource-only backfill; never logs in, scans installations or changes game ownership.
    pub async fn repair_epic_artwork<F>(&self,scraper:game_metadata::Scraper,root:&Path,retry:bool,progress:F)->DbResult<Value>
    where F:Fn(Value)+Send+Sync+'static{
        let report_path=root.join("reports/epic-artwork-latest.json");
        let mut history:Value=std::fs::read(&report_path).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_else(||json!({"games":{}}));
        if !history.is_object()||!history["games"].is_object(){history=json!({"games":{}});}
        let progress=std::sync::Arc::new(progress);
        let now=Utc::now().timestamp();let mut tasks=tokio::task::JoinSet::new();let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(2));let mut skipped=0;
        for game in self.list_games().await?.into_iter().filter(|g|g.platform=="epic"&&!g.hidden){
            let kinds=missing_artwork(&game);if kinds.is_empty(){if !history["games"][&game.id].is_null(){history["games"][&game.id]["missing"]=json!([]);history["games"][&game.id]["errors"]=json!([]);}continue;}
            let previous=&history["games"][&game.id];
            if !retry&&previous["revision"]==2&&previous["attempted_at"].as_i64().is_some_and(|t|now-t<6*3600)&&previous["missing"]==json!(kinds){skipped+=1;continue;}
            let(db,scraper,root,limit)=(self.clone(),scraper.clone(),root.to_owned(),limit.clone());
            let updates=progress.clone();
            tasks.spawn(async move{
                let _permit=limit.acquire_owned().await.map_err(|e|e.to_string())?;
                let reference=game_metadata::GameRef{id:game.id.clone(),name:game.original_name.clone(),provider:"epic".into(),external_id:game.external_id.clone()};
                let sgdb=db.confirmed_sgdb_id(&game.id).await?;let steam=db.confirmed_steam_artwork(&game.id).await?;
                let mut saved=0;let mut errors=Vec::new();
                // Save title first, so a slow background search cannot hold a ready logo back.
                for requested in [vec!["logo"],vec!["poster","landscape","hero"]]{
                    let requested:Vec<_>=requested.into_iter().filter(|k|kinds.contains(k)).collect();if requested.is_empty(){continue;}
                    let result=tokio::time::timeout(std::time::Duration::from_secs(150),scraper.scrape_kinds(&reference,&root,false,sgdb,steam,&requested)).await;
                    match result{
                        Ok((assets,issues))=>{errors.extend(issues);for asset in assets{
                            if !kinds.contains(&asset.kind){continue;}
                            match db.save_scraped_asset(&game.id,&asset).await{Ok(true)=>{saved+=1;updates(json!({"stage":"asset","game_id":game.id,"kind":asset.kind}));if asset.kind=="hero"{let(path,root)=(asset.path.clone(),root.clone());let _=tokio::task::spawn_blocking(move||game_metadata::display::background(&path,&root)).await;}},Ok(false)=>(),Err(e)=>errors.push(e)}
                        }},Err(_)=>errors.push("素材请求超时，已保留原有图片，可稍后重试".into())
                    }
                }
                let fresh=db.list_games().await?.into_iter().find(|g|g.id==game.id);let missing=fresh.as_ref().map(missing_artwork).unwrap_or_default();
                Ok::<_,String>((game.id,json!({"revision":2,"game":game.name,"attempted_at":Utc::now().timestamp(),"saved":saved,"missing":missing,"errors":errors})))
            });
        }
        let total=tasks.len();let(mut processed,mut saved)=(0,0);let mut errors=Vec::new();
        while let Some(result)=tasks.join_next().await{
            processed+=1;match result{Ok(Ok((id,entry)))=>{saved+=entry["saved"].as_u64().unwrap_or(0) as usize;history["games"][id]=entry;},Ok(Err(e))=>errors.push(e),Err(e)=>errors.push(e.to_string())}
            history["processed"]=json!(processed);history["total"]=json!(total);history["saved"]=json!(saved);history["skipped"]=json!(skipped);history["errors"]=json!(errors);
            std::fs::create_dir_all(report_path.parent().unwrap()).map_err(|e|e.to_string())?;
            let pending=report_path.with_extension("pending");std::fs::write(&pending,serde_json::to_vec_pretty(&history).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;std::fs::rename(pending,&report_path).map_err(|e|e.to_string())?;
            progress(json!({"processed":processed,"total":total,"saved":saved}));
        }
        history["processed"]=json!(processed);history["total"]=json!(total);history["saved"]=json!(saved);history["skipped"]=json!(skipped);history["errors"]=json!(errors);
        std::fs::create_dir_all(report_path.parent().unwrap()).map_err(|e|e.to_string())?;
        let pending=report_path.with_extension("pending");std::fs::write(&pending,serde_json::to_vec_pretty(&history).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;std::fs::rename(pending,&report_path).map_err(|e|e.to_string())?;
        Ok(json!({"processed":processed,"total":total,"saved":saved,"skipped":skipped,"errors":errors}))
    }
}

#[cfg(test)]mod tests{use super::*;
 #[test]fn automatic_backfill_respects_retry_cooldown_and_clears_resolved_failures(){tokio::runtime::Runtime::new().unwrap().block_on(async{
    let root=std::env::temp_dir().join(format!("epic-cooldown-{}",Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let db=Storage::open(&root.join("test.db")).await.unwrap();
    db.upsert_discovered(vec![DiscoveredGame{provider:"epic".into(),external_id:"A".into(),name:"Fixture".into(),install_path:root.to_string_lossy().into()}],vec![]).await.unwrap();let game=db.list_games().await.unwrap().remove(0);
    let image=root.join("existing-artwork.png");std::fs::write(&image,b"existing file fixture").unwrap();
    db.save_local_assets(["poster","landscape"].into_iter().map(|kind|(game.id.clone(),game_metadata::Artwork{kind,path:image.clone(),source:"fixture",width:1920,height:1080})).collect()).await.unwrap();
    let art=root.join("artwork");std::fs::create_dir_all(art.join("reports")).unwrap();let report=art.join("reports/epic-artwork-latest.json");
    std::fs::write(&report,serde_json::to_vec(&json!({"games":{game.id.clone():{"revision":2,"game":"Fixture","attempted_at":Utc::now().timestamp(),"saved":0,"missing":["hero","logo"],"errors":["previous network error"]}}})).unwrap()).unwrap();
    let result=db.repair_epic_artwork(game_metadata::Scraper::new(None).unwrap(),&art,false,|_|panic!("cooldown must not run a network job")).await.unwrap();assert_eq!(result["processed"],0);assert_eq!(result["skipped"],1);
    for kind in ["hero","logo"]{sqlx::query("INSERT INTO game_assets(id,game_id,asset_type,path,source,priority,width,height) VALUES(?,?,?,?,?,10,1920,1080)").bind(Uuid::new_v4().to_string()).bind(&game.id).bind(kind).bind(image.to_string_lossy().as_ref()).bind(if kind=="logo"{"title-logo-original"}else{"fixture"}).execute(&db.pool).await.unwrap();}
    let result=db.repair_epic_artwork(game_metadata::Scraper::new(None).unwrap(),&art,false,|_|panic!("complete artwork must not schedule a job")).await.unwrap();assert_eq!(result["processed"],0);
    let report:Value=serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();assert_eq!(report["games"][&game.id]["missing"],json!([]));assert_eq!(report["games"][&game.id]["errors"],json!([]));
    db.pool.close().await;std::fs::remove_dir_all(root).unwrap();
 });}
 #[test]fn epic_account_covers_do_not_count_as_a_complete_artwork_set(){tokio::runtime::Runtime::new().unwrap().block_on(async{
    let root=std::env::temp_dir().join(format!("epic-artwork-{}",Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let db=Storage::open(&root.join("test.db")).await.unwrap();
    db.upsert_discovered(vec![DiscoveredGame{provider:"epic".into(),external_id:"A".into(),name:"My Game".into(),install_path:root.to_string_lossy().into()}],vec![]).await.unwrap();
    let mut game=db.list_games().await.unwrap().remove(0);let poster=root.join("poster.png");let card=root.join("card.png");std::fs::write(&poster,b"fixture").unwrap();std::fs::write(&card,b"fixture").unwrap();game.poster=Some(poster.to_string_lossy().into());game.landscape=Some(card.to_string_lossy().into());
    assert_eq!(missing_artwork(&game),vec!["hero","logo"]);game.hero=game.poster.clone();game.logo=game.landscape.clone();assert!(missing_artwork(&game).is_empty());std::fs::remove_file(poster).unwrap();assert_eq!(missing_artwork(&game),vec!["poster","hero"]);
    db.pool.close().await;std::fs::remove_dir_all(root).unwrap();
 });}
}

use super::*;

impl Storage{
    pub async fn normalize_titles(&self)->DbResult<usize>{
        let rows=sqlx::query("SELECT a.id,a.path,a.width,a.height FROM game_assets a JOIN games g ON g.id=a.game_id WHERE a.asset_type='logo' AND g.hidden=0 AND a.priority<100").fetch_all(&self.pool).await.map_err(|e|e.to_string())?;
        let mut count=0;
        for row in rows{
            let path: String=row.get("path");if path.contains("-trimmed-"){continue;}
            let asset=game_metadata::Artwork{kind:"logo",path:path.into(),source:"cache",width:row.get::<Option<i64>,_>("width").unwrap_or(0) as u32,height:row.get::<Option<i64>,_>("height").unwrap_or(0) as u32};
            let Ok(asset)=game_metadata::title::display(&asset)else{continue};
            sqlx::query("UPDATE game_assets SET path=?,width=?,height=? WHERE id=?").bind(asset.path.to_string_lossy().as_ref()).bind(asset.width as i64).bind(asset.height as i64).bind(row.get::<String,_>("id")).execute(&self.pool).await.map_err(|e|e.to_string())?;count+=1;
        }Ok(count)
    }
    pub async fn repair_titles(&self,scraper:game_metadata::Scraper,root:&Path)->DbResult<serde_json::Value>{
        let normalized=self.normalize_titles().await?;
        let current=self.list_games().await?;let refs=self.public_games_for_scraping().await?;
        let mut tasks=tokio::task::JoinSet::new();let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(3));
        for game in refs{
            // Epic artwork is completed by its own bounded, per-asset backfill queue.
            if game.provider=="epic"{continue;}
            if current.iter().find(|g|g.id==game.id).and_then(|g|g.logo.as_ref()).is_some_and(|path|Path::new(path).is_file()){continue;}
            let (db,scraper,root,limit)=(self.clone(),scraper.clone(),root.to_owned(),limit.clone());
            tasks.spawn(async move{
                let _permit=limit.acquire_owned().await.map_err(|e|e.to_string())?;
                let sgdb=db.confirmed_sgdb_id(&game.id).await?;let steam=db.confirmed_steam_artwork(&game.id).await?;
                let result=scraper.scrape_title(&game,&root,sgdb,steam).await;
                match result{Ok(Some(asset))=>{db.save_scraped_asset(&game.id,&asset).await?;Ok::<_,String>(serde_json::json!({"game":game.name,"saved":true}))},Ok(None)=>Ok(serde_json::json!({"game":game.name,"saved":false,"reason":"没有可用的透明 Logo"})),Err(e)=>Ok(serde_json::json!({"game":game.name,"saved":false,"reason":e}))}
            });
        }
        let mut entries=Vec::new();while let Some(result)=tasks.join_next().await{match result{Ok(Ok(row))=>entries.push(row),Ok(Err(e))=>entries.push(serde_json::json!({"error":e})),Err(e)=>entries.push(serde_json::json!({"error":e.to_string()}))}}
        Ok(serde_json::json!({"normalized":normalized,"entries":entries}))
    }
}

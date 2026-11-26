use super::*;
use game_metadata::{GameRef, Scraper};
use serde_json::{json, Value};

impl Storage {
    /// Clear only automatic backgrounds; manual overrides and card artwork are untouched.
    pub async fn clear_auto_background(&self, id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM game_assets WHERE game_id=? AND asset_type='hero' AND priority<100")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn refresh_promo_game(&self, scraper: &Scraper, game: &GameRef, root: &Path) -> DbResult<Value> {
        if self.custom_hero(&game.id).await? {
            return Ok(json!({"id":game.id,"game":game.name,"status":"manual-preserved"}));
        }
        let sgdb = self.confirmed_sgdb_id(&game.id).await?;
        let steam = self.confirmed_steam_artwork(&game.id).await?;
        match scraper.scrape_background(game, root, sgdb, steam, true).await {
            Ok(asset) => {
                self.save_scraped_asset(&game.id, &asset).await?;
                // Warm exactly the display cache used by Home/Details, before reporting completion.
                let display = game_metadata::display::background(&asset.path, root);
                let audit = std::fs::read(root.join("hero").join(format!("{}-promo-report.json",game.id)))
                    .ok().and_then(|s|serde_json::from_slice::<Value>(&s).ok());
                Ok(json!({"id":game.id,"game":game.name,"status":"updated","width":asset.width,
                    "height":asset.height,"path":asset.path,"source":asset.source,
                    "selection":audit.as_ref().and_then(|v|v.get("selected")),
                    "audit":audit,
                    "display_cache":display.as_ref().ok(),"display_error":display.err()}))
            }
            Err(reason) => {
                let audit=std::fs::read(root.join("hero").join(format!("{}-promo-report.json",game.id))).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok());
                Ok(json!({"id":game.id,"game":game.name,"status":"no-verified-resource","reason":reason,"audit":audit}))
            }
        }
    }

    /// Shared by the desktop command and the native maintenance executable.
    /// Every visible library entry is audited, including old 4K caches.
    pub async fn refresh_promo_backgrounds<F>(&self, scraper: Scraper, root: &Path, progress: F) -> DbResult<Value>
    where F: Fn(Value) + Send + Sync {
        let games = self.public_games_for_scraping().await?;
        let total = games.len();
        let stamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let reports = root.join("reports");
        std::fs::create_dir_all(&reports).map_err(|e|e.to_string())?;
        let before: Vec<_> = self.list_games().await?.into_iter().filter(|game|!game.hidden).collect();
        std::fs::write(reports.join(format!("background-before-{stamp}.json")),serde_json::to_vec_pretty(&before).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let limit = std::sync::Arc::new(tokio::sync::Semaphore::new(3));
        let mut tasks = tokio::task::JoinSet::new();
        for game in games {
            let (db,scraper,root,limit)=(self.clone(),scraper.clone(),root.to_owned(),limit.clone());
            tasks.spawn(async move {
                let _permit=limit.acquire_owned().await.map_err(|e|e.to_string())?;
                db.refresh_promo_game(&scraper,&game,&root).await
                    .map_err(|e|format!("{}: {e}",game.name))
            });
        }
        let mut entries=Vec::new(); let mut errors=Vec::new(); let mut processed=0;
        while let Some(result)=tasks.join_next().await {
            processed+=1;
            match result { Ok(Ok(entry))=>entries.push(entry), Ok(Err(e))=>errors.push(e), Err(e)=>errors.push(e.to_string()) }
            let saved=entries.iter().filter(|e|e["status"]=="updated").count();
            let latest=json!({"policy":game_metadata::promo::POLICY,"total":total,"processed":processed,
                "saved":saved,"entries":entries,"errors":errors});
            let bytes=serde_json::to_vec_pretty(&latest).map_err(|e|e.to_string())?;
            std::fs::write(reports.join("background-refresh-latest.json"),&bytes).map_err(|e|e.to_string())?;
            std::fs::write(reports.join(format!("background-refresh-{stamp}.json")),&bytes).map_err(|e|e.to_string())?;
            progress(json!({"processed":processed,"total":total,"saved":saved,"last":entries.last()}));
        }
        let mut markdown=format!("# 整库宣传背景核验\n\n规则版本：{}。已处理 {processed}/{total} 款。\n\n| 游戏 | 采用尺寸 | 结果 | 未采用原因（本轮检索） |\n|---|---|---|---|\n",game_metadata::promo::POLICY);
        for entry in &entries {
            let name=before.iter().find(|g|Some(g.id.as_str())==entry["id"].as_str()).map(|g|g.name.as_str()).unwrap_or_else(||entry["game"].as_str().unwrap_or(""));
            let mut reasons=std::collections::BTreeSet::new();
            if let Some(candidates)=entry["audit"]["candidates"].as_array(){for c in candidates {if c["decision"]=="rejected" {if let Some(r)=c["reason"].as_str(){reasons.insert(r.to_string());}}}}
            if let Some(issues)=entry["audit"]["issues"].as_array(){for issue in issues {if let Some(r)=issue.as_str(){reasons.insert(r.to_string());}}}
            if let Some(reason)=entry["reason"].as_str(){reasons.insert(reason.into());}
            let status=entry["selection"]["tier"].as_str().unwrap_or_else(||entry["status"].as_str().unwrap_or(""));
            let dimensions=match(entry["width"].as_u64(),entry["height"].as_u64()){(Some(w),Some(h))=>format!("{w}×{h}"),_=>"—".into()};
            markdown.push_str(&format!("| {} | {dimensions} | {status} | {} |\n",name.replace('|',"/"),reasons.into_iter().collect::<Vec<_>>().join("；").replace('|',"/")));
        }
        std::fs::write(reports.join("background-refresh-latest.md"),markdown).map_err(|e|e.to_string())?;
        Ok(json!({"policy":game_metadata::promo::POLICY,"total":total,"processed":processed,
            "saved":entries.iter().filter(|e|e["status"]=="updated").count(),"entries":entries,"errors":errors}))
    }
}

//! SQLx + SQLite 数据层：Game / GameSource / Installation 分层持久化。
use chrono::Utc;
use game_core::{DiscoveredGame, Game, LaunchTarget, ScanReport};
use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, Row, SqlitePool};
use std::path::Path;
use uuid::Uuid;
mod promo_refresh;
mod title_repair;
mod epic_library;
mod epic_artwork;
pub mod manual;

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_import_survives_restart_and_scan_and_preserves_existing_records() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let dir=std::env::temp_dir().join(format!("gamedeck-hidden-{}",Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            let path=dir.join("Game.EXE");std::fs::write(&path,b"test-only").unwrap();
            let db=Storage::open(&dir.join("test.db")).await.unwrap();
            let id=db.import_manual(&path,Some("我的游戏"),true).await.unwrap();
            db.set_favorite(&id,true).await.unwrap();
            assert_eq!(db.import_manual(&path,None,true).await.unwrap(),id);
            assert!(db.add_manual(&path).await.is_err());
            assert!(db.import_manual(&dir,None,true).await.is_err());
            db.pool.close().await;
            let db=Storage::open(&dir.join("test.db")).await.unwrap();
            let game=db.list_games().await.unwrap().remove(0);
            assert!(game.hidden&&game.favorite);assert_eq!(game.name,"我的游戏");
            assert!(db.public_games_for_scraping().await.unwrap().is_empty());
            let launch=db.launch_target(game.launch_installation_id.as_deref().unwrap()).await.unwrap();
            assert_eq!(Path::new(&launch.install_path),path.canonicalize().unwrap());
            let entry=DiscoveredGame{provider:"steam".into(),external_id:"hidden-test".into(),name:"Steam Game".into(),install_path:dir.to_string_lossy().into()};
            db.upsert_discovered(vec![entry.clone()],vec![]).await.unwrap();
            let platform=db.list_games().await.unwrap().into_iter().find(|g|g.platform=="steam").unwrap();
            db.set_hidden(&platform.id,true).await.unwrap();db.upsert_discovered(vec![entry],vec![]).await.unwrap();
            assert!(db.list_games().await.unwrap().into_iter().find(|g|g.id==platform.id).unwrap().hidden);
            db.set_hidden(&id,false).await.unwrap();
            let restored=db.list_games().await.unwrap().into_iter().find(|g|g.id==id).unwrap();
            assert!(!restored.hidden&&restored.favorite);assert_eq!(restored.name,"我的游戏");
            assert!(db.set_hidden("missing",true).await.is_err());
            db.pool.close().await;std::fs::remove_dir_all(dir).unwrap();
        });
    }

    #[test]
    fn manual_override_survives_scraping_and_reset_restores_cached_artwork() {
      tokio::runtime::Runtime::new().unwrap().block_on(async {
        let dir = std::env::temp_dir().join(format!("gamedeck-storage-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Storage::open(&dir.join("test.db")).await.unwrap();
        let entry = DiscoveredGame { provider: "steam".into(), external_id: "570".into(), name: "Dota 2".into(), install_path: dir.to_string_lossy().into() };
        assert_eq!(db.upsert_discovered(vec![entry.clone()], vec![]).await.unwrap().inserted, 1);
        let game = db.list_games().await.unwrap().remove(0);
        db.save_chinese_titles(&[(game.id.clone(),"刀塔2".into())]).await.unwrap();
        assert_eq!(db.list_games().await.unwrap()[0].name,"刀塔2");
        assert_eq!(db.list_games().await.unwrap()[0].original_name,"Dota 2");
        assert_eq!(db.games_for_scraping().await.unwrap()[0].name,"Dota 2");
        let manual = dir.join("manual.png");
        db.save_manual_asset(&game.id, "poster", &manual, 600, 900).await.unwrap();
        let automatic = game_metadata::Artwork { kind: "poster", path: dir.join("auto.png"), source: "steam", width: 600, height: 900 };
        assert!(db.save_scraped_asset(&game.id, &automatic).await.unwrap());
        assert_eq!(db.upsert_discovered(vec![entry], vec![]).await.unwrap().updated, 1);
        assert_eq!(db.list_games().await.unwrap()[0].poster.as_deref(), manual.to_str());
        let choices=db.artwork_choices(&game.id).await.unwrap();
        assert!(choices.iter().any(|choice|choice["kind"]=="poster"&&choice["manual"]==true&&choice["width"]==600));
        db.reset_manual_asset(&game.id, "poster").await.unwrap();
        assert!(db.artwork_choices(&game.id).await.unwrap().iter().any(|choice|choice["kind"]=="poster"&&choice["manual"]==false));
        assert_eq!(db.list_games().await.unwrap()[0].poster.as_deref(), automatic.path.to_str());
        db.confirm_sgdb(&game.id, 123).await.unwrap();
        assert_eq!(db.confirmed_sgdb_id(&game.id).await.unwrap(), Some(123));
        // Repeated platform sync must replace totals, never add them to local sessions.
        db.set_favorite(&game.id,true).await.unwrap();
        sqlx::query("UPDATE games SET total_minutes=15 WHERE id=?").bind(&game.id).execute(&db.pool).await.unwrap();
        let records = vec![("570".to_string(),10612,Some(1771310231))];
        assert_eq!(db.sync_steam_playtime("account-a",&records).await.unwrap(),1);
        db.sync_steam_playtime("account-a",&records).await.unwrap();
        let synced = db.list_games().await.unwrap().remove(0);
        assert_eq!(synced.total_minutes,10612);
        assert_eq!(synced.local_minutes,15);
        assert_eq!(synced.playtime_source,"steam-local");
        assert!(synced.favorite);
        db.sync_steam_playtime("account-b",&[("570".into(),60,None)]).await.unwrap();
        assert_eq!(db.list_games().await.unwrap()[0].total_minutes,10612);
        db.select_steam_account("account-b").await.unwrap();
        assert_eq!(db.list_games().await.unwrap()[0].total_minutes,60);
        db.import_steam_catalogue("account-b", &[("730".into(),"Counter-Strike 2".into())], "cache").await.unwrap();
        assert_eq!(db.list_games().await.unwrap().len(),2);
        assert!(!db.list_games().await.unwrap().iter().find(|g| g.external_id=="730").unwrap().installed);
        db.select_steam_account("account-a").await.unwrap();
        assert_eq!(db.list_games().await.unwrap().len(),1);
        assert_eq!(db.list_games().await.unwrap()[0].total_minutes,10612);
        let reviewed=game_metadata::Artwork{kind:"poster",path:dir.join("reviewed.webp"),source:"reviewed-promo-motion",width:600,height:900};
        std::fs::write(&reviewed.path,b"reviewed-test-asset").unwrap();db.save_scraped_asset(&game.id,&reviewed).await.unwrap();
        let local=game_metadata::Artwork{kind:"poster",path:dir.join("native.png"),source:"battlenet-local",width:1200,height:1800};std::fs::write(&local.path,b"native-test-asset").unwrap();
        db.save_local_assets(vec![(game.id.clone(),local.clone())]).await.unwrap();assert_eq!(db.list_games().await.unwrap()[0].poster.as_deref(),reviewed.path.to_str());
        std::fs::remove_file(&reviewed.path).unwrap();db.save_local_assets(vec![(game.id.clone(),local.clone())]).await.unwrap();assert_eq!(db.list_games().await.unwrap()[0].poster.as_deref(),local.path.to_str());
        db.save_manual_asset(&game.id,"hero",&manual,3840,2160).await.unwrap();
        let old_hero=game_metadata::Artwork{kind:"hero",path:dir.join("old-wallpaper.png"),source:"background-alphacoders",width:8192,height:4096};
        db.save_scraped_asset(&game.id,&old_hero).await.unwrap();
        db.clear_auto_background(&game.id).await.unwrap();
        let preserved=db.list_games().await.unwrap().into_iter().find(|g|g.id==game.id).unwrap();
        assert_eq!(preserved.hero.as_deref(),manual.to_str());
        db.reset_manual_asset(&game.id,"hero").await.unwrap();
        assert!(db.list_games().await.unwrap().into_iter().find(|g|g.id==game.id).unwrap().hero.is_none());
        db.pool.close().await;
        std::fs::remove_dir_all(&dir).unwrap();
      });
    }
}

type DbResult<T> = Result<T, String>;

impl Storage {
    pub async fn open(path: &Path) -> DbResult<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .busy_timeout(std::time::Duration::from_secs(10))
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(4)
            .connect_with(options).await.map_err(|e| e.to_string())?;
        sqlx::migrate!("./migrations").run(&pool).await.map_err(|e| e.to_string())?;
        Ok(Self { pool })
    }

    pub async fn asset_path(&self,id:&str,kind:&str)->DbResult<Option<String>>{
        sqlx::query_scalar("SELECT path FROM game_assets WHERE game_id=? AND asset_type=? ORDER BY priority DESC LIMIT 1").bind(id).bind(kind).fetch_optional(&self.pool).await.map_err(|e|e.to_string())
    }
    pub async fn hero_path(&self,id:&str)->DbResult<Option<String>>{
        sqlx::query_scalar("SELECT path FROM game_assets WHERE game_id=? AND asset_type='hero' ORDER BY priority DESC LIMIT 1").bind(id).fetch_optional(&self.pool).await.map_err(|e|e.to_string())
    }
    pub async fn list_games(&self) -> DbResult<Vec<Game>> {
        let rows = sqlx::query(r#"
            SELECT g.id, COALESCE(NULLIF(g.name_zh,''),g.name) AS name, g.name AS original_name, g.description, g.genre, g.favorite, g.hidden,
                   COALESCE(p.minutes,g.total_minutes) AS total_minutes, g.total_minutes AS local_minutes,
                   COALESCE(p.last_played_at,g.last_played_at) AS last_played_at, g.created_at,
                   CASE WHEN p.source_id IS NOT NULL THEN 'steam-local' ELSE 'gamedeck' END AS playtime_source,
                   p.synced_at AS playtime_synced_at,
                   COALESCE(s.provider,'manual') AS provider,
                   COALESCE(s.external_id,'') AS external_id,
                   i.id AS installation_id,
                   COALESCE(i.installed,0) AS installed,
                   CASE WHEN COALESCE(i.installed,0)=0 THEN 'not_installed' ELSE COALESCE(i.update_status,'unknown') END AS update_status,
                   i.installed_version,i.update_detail,i.update_checked_at,
                   (SELECT path FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='poster' ORDER BY priority DESC LIMIT 1) AS poster,
                   (SELECT path FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='hero' ORDER BY priority DESC LIMIT 1) AS hero,
                   (SELECT width FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='hero' ORDER BY priority DESC LIMIT 1) AS hero_width,
                   (SELECT height FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='hero' ORDER BY priority DESC LIMIT 1) AS hero_height,
                   (SELECT source FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='hero' ORDER BY priority DESC LIMIT 1) AS hero_source,
                   (SELECT path FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='landscape' ORDER BY priority DESC LIMIT 1) AS landscape,
                   (SELECT path FROM game_assets a WHERE a.game_id=g.id AND a.asset_type='logo' AND (priority>=100 OR source IN ('title-logo-official','title-logo-original','steam-local','battlenet-local','epic-local')) ORDER BY priority DESC,CASE WHEN source IN ('title-logo-official','steam-local') THEN 0 ELSE 1 END LIMIT 1) AS logo
            FROM games g
            LEFT JOIN game_sources s ON s.id = (
                SELECT id FROM game_sources WHERE game_id=g.id ORDER BY CASE provider WHEN 'steam' THEN 0 ELSE 1 END,id LIMIT 1
            )
            LEFT JOIN installations i ON i.id = (
                SELECT id FROM installations WHERE source_id=s.id ORDER BY installed DESC,id LIMIT 1
            )
            LEFT JOIN platform_playtime p ON p.source_id=s.id AND p.account_id=(SELECT value_json FROM settings WHERE key='steam_active_account')
            WHERE NOT (s.provider='steam' AND s.external_id='228980') AND (COALESCE(i.installed,0)=1 OR s.provider<>'steam' OR EXISTS (
                SELECT 1 FROM account_games ag WHERE ag.source_id=s.id AND ag.account_id=(SELECT value_json FROM settings WHERE key='steam_active_account')
            ))
            AND (s.provider<>'epic' OR COALESCE(i.installed,0)=1 OR EXISTS (SELECT 1 FROM account_games eg WHERE eg.source_id=s.id AND eg.provider='epic' AND eg.account_id=(SELECT value_json FROM settings WHERE key='epic_active_account')))
            ORDER BY COALESCE(p.last_played_at,g.last_played_at,g.created_at) DESC, g.name ASC
        "#).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        rows.into_iter().map(|row| {
            Ok(Game {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                original_name: row.try_get("original_name").map_err(|e| e.to_string())?,
                description: row.try_get("description").map_err(|e| e.to_string())?,
                genre: row.try_get("genre").map_err(|e| e.to_string())?,
                platform: row.try_get("provider").map_err(|e| e.to_string())?,
                external_id: row.try_get("external_id").map_err(|e| e.to_string())?,
                launch_installation_id: row.try_get("installation_id").map_err(|e| e.to_string())?,
                installed: row.try_get::<i64,_>("installed").map_err(|e| e.to_string())? != 0,
                update_status: row.try_get("update_status").map_err(|e|e.to_string())?,
                installed_version: row.try_get("installed_version").map_err(|e|e.to_string())?,
                update_detail: row.try_get("update_detail").map_err(|e|e.to_string())?,
                update_checked_at: row.try_get("update_checked_at").map_err(|e|e.to_string())?,
                favorite: row.try_get::<i64,_>("favorite").map_err(|e| e.to_string())? != 0,
                hidden: row.try_get::<i64,_>("hidden").map_err(|e| e.to_string())? != 0,
                total_minutes: row.try_get("total_minutes").map_err(|e| e.to_string())?,
                last_played_at: row.try_get("last_played_at").map_err(|e| e.to_string())?,
                added_at: row.try_get("created_at").map_err(|e| e.to_string())?,
                poster: row.try_get("poster").map_err(|e| e.to_string())?,
                hero: row.try_get("hero").map_err(|e| e.to_string())?,
                hero_width: row.try_get("hero_width").map_err(|e|e.to_string())?,
                hero_height: row.try_get("hero_height").map_err(|e|e.to_string())?,
                hero_source: row.try_get("hero_source").map_err(|e|e.to_string())?,
                landscape: row.try_get("landscape").map_err(|e| e.to_string())?,
                logo:row.try_get("logo").map_err(|e|e.to_string())?,
                local_minutes: row.try_get("local_minutes").map_err(|e| e.to_string())?,
                playtime_source: row.try_get("playtime_source").map_err(|e| e.to_string())?,
                playtime_synced_at: row.try_get("playtime_synced_at").map_err(|e| e.to_string())?,
            })
        }).collect()
    }

    pub async fn upsert_discovered(&self, discovered: Vec<DiscoveredGame>, errors: Vec<String>) -> DbResult<ScanReport> {
        let mut result = ScanReport { found: discovered.len(), inserted: 0, updated: 0, errors };
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        for entry in discovered {
            let existing = sqlx::query("SELECT id, game_id FROM game_sources WHERE provider=? AND external_id=?")
                .bind(&entry.provider).bind(&entry.external_id)
                .fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            let source_id = if let Some(row) = existing {
                result.updated += 1;
                row.try_get::<String,_>("id").map_err(|e| e.to_string())?
            } else {
                result.inserted += 1;
                let game_id = Uuid::new_v4().to_string();
                let source_id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO games(id,name,normalized_name,created_at) VALUES(?,?,?,?)")
                    .bind(&game_id).bind(&entry.name).bind(entry.name.trim().to_lowercase()).bind(&now)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
                sqlx::query("INSERT INTO game_sources(id,game_id,provider,external_id,source_name) VALUES(?,?,?,?,?)")
                    .bind(&source_id).bind(&game_id).bind(&entry.provider).bind(&entry.external_id).bind(&entry.name)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
                source_id
            };
            let installation = sqlx::query("SELECT id FROM installations WHERE source_id=? ORDER BY id LIMIT 1")
                .bind(&source_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            if let Some(row) = installation {
                let id: String = row.try_get("id").map_err(|e| e.to_string())?;
                sqlx::query("UPDATE installations SET install_path=?, installed=1,discovered_at=? WHERE id=?")
                    .bind(&entry.install_path).bind(&now).bind(&id)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            } else {
                sqlx::query("INSERT INTO installations(id,source_id,install_path,installed,discovered_at) VALUES(?,?,?,1,?)")
                    .bind(Uuid::new_v4().to_string()).bind(&source_id).bind(&entry.install_path).bind(&now)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub async fn add_manual(&self, file: &Path) -> DbResult<()> {
        self.import_manual(file, None, false).await.map(|_| ())
    }

    pub async fn set_hidden(&self, game_id: &str, hidden: bool) -> DbResult<()> {
        let result = sqlx::query("UPDATE games SET hidden=? WHERE id=?")
            .bind(hidden).bind(game_id).execute(&self.pool).await.map_err(|e|e.to_string())?;
        if result.rows_affected() == 0 { return Err("未找到游戏".into()); }
        Ok(())
    }

    pub async fn import_manual(&self, file: &Path, name: Option<&str>, hidden: bool) -> DbResult<String> {
        let canonical = file.canonicalize().map_err(|e| format!("文件不存在：{e}"))?;
        if !canonical.is_file() || !canonical.extension().is_some_and(|ext| ext.to_str().is_some_and(|name| name.eq_ignore_ascii_case("exe"))) {
            return Err("仅允许导入现有 .exe 文件".into());
        }
        let target = canonical.to_string_lossy().to_string();
        let mut tx = self.pool.begin().await.map_err(|e|e.to_string())?;
        let existing = sqlx::query("SELECT s.game_id FROM installations i JOIN game_sources s ON s.id=i.source_id WHERE i.install_path=?")
            .bind(&target).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
        if let Some(row) = existing {
            if !hidden { return Err("这个游戏已经导入过了".into()); }
            let id: String = row.try_get("game_id").map_err(|e|e.to_string())?;
            sqlx::query("UPDATE games SET hidden=1 WHERE id=?").bind(&id).execute(&mut *tx).await.map_err(|e|e.to_string())?;
            tx.commit().await.map_err(|e|e.to_string())?;
            return Ok(id);
        }
        let name = name.map(str::trim).filter(|s|!s.is_empty()).unwrap_or_else(||canonical.file_stem().and_then(|x|x.to_str()).unwrap_or("本地游戏")).to_string();
        if name.chars().count()>120 { return Err("游戏名称最多 120 个字".into()); }
        let game_id = Uuid::new_v4().to_string();
        let source_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO games(id,name,normalized_name,created_at,hidden) VALUES(?,?,?,?,?)")
            .bind(&game_id).bind(&name).bind(name.to_lowercase()).bind(&now).bind(hidden)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO game_sources(id,game_id,provider,external_id,source_name) VALUES(?,?,?,?,?)")
            .bind(&source_id).bind(&game_id).bind("manual").bind(&target).bind(&name)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO installations(id,source_id,install_path,installed,discovered_at) VALUES(?,?,?,1,?)")
            .bind(Uuid::new_v4().to_string()).bind(&source_id).bind(&target).bind(&now)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(game_id)
    }

    pub async fn games_for_scraping(&self) -> DbResult<Vec<game_metadata::GameRef>> {
        Ok(self.list_games().await?.into_iter().map(|game| game_metadata::GameRef {
            id: game.id, name: game.original_name, provider: game.platform, external_id: game.external_id,
        }).collect())
    }
    pub async fn public_games_for_scraping(&self) -> DbResult<Vec<game_metadata::GameRef>> {
        Ok(self.list_games().await?.into_iter().filter(|game|!game.hidden).map(|game|game_metadata::GameRef {
            id:game.id,name:game.original_name,provider:game.platform,external_id:game.external_id,
        }).collect())
    }
    pub async fn installation_for_status(&self,game_id:&str)->DbResult<Option<DiscoveredGame>>{
        let row=sqlx::query("SELECT s.provider,s.external_id,g.name,i.install_path FROM installations i JOIN game_sources s ON s.id=i.source_id JOIN games g ON g.id=s.game_id WHERE g.id=? AND i.installed=1 ORDER BY CASE s.provider WHEN 'steam' THEN 0 ELSE 1 END,s.id,i.id LIMIT 1").bind(game_id).fetch_optional(&self.pool).await.map_err(|e|e.to_string())?;
        Ok(row.map(|r|DiscoveredGame{provider:r.get("provider"),external_id:r.get("external_id"),name:r.get("name"),install_path:r.get("install_path")}))
    }
    pub async fn installed_games_for_status(&self)->DbResult<Vec<DiscoveredGame>>{
        let rows=sqlx::query("SELECT s.provider,s.external_id,g.name,i.install_path FROM installations i JOIN game_sources s ON s.id=i.source_id JOIN games g ON g.id=s.game_id WHERE i.installed=1").fetch_all(&self.pool).await.map_err(|e|e.to_string())?;
        Ok(rows.into_iter().map(|r|DiscoveredGame{provider:r.get("provider"),external_id:r.get("external_id"),name:r.get("name"),install_path:r.get("install_path")}).collect())
    }

    pub async fn sync_steam_playtime(&self, account_id: &str, records: &[(String,i64,Option<i64>)]) -> DbResult<usize> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        // Replace one selected account snapshot; never sum accounts or add platform totals to local sessions.
        sqlx::query("INSERT OR IGNORE INTO settings(key,value_json) VALUES('steam_active_account',?)")
            .bind(account_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        let mut updated = 0;
        for (app_id,minutes,last_played) in records {
            if *minutes < 0 { continue; }
            let played = last_played.and_then(|seconds| chrono::DateTime::from_timestamp(seconds,0)).map(|date| date.to_rfc3339());
            let result = sqlx::query("INSERT INTO platform_playtime(source_id,account_id,minutes,last_played_at,synced_at) SELECT id,?,?,?,? FROM game_sources WHERE provider='steam' AND external_id=? ON CONFLICT(source_id,account_id) DO UPDATE SET minutes=excluded.minutes,last_played_at=excluded.last_played_at,synced_at=excluded.synced_at")
                .bind(account_id).bind(minutes).bind(played).bind(&now).bind(app_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
            updated += result.rows_affected() as usize;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(updated)
    }

    pub async fn save_chinese_titles(&self, titles: &[(String,String)]) -> DbResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for (id,name) in titles {
            if name.trim().is_empty() { continue; }
            sqlx::query("UPDATE games SET name_zh=? WHERE id=?").bind(name).bind(id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?; Ok(())
    }
    pub async fn save_installation_status(&self,provider:&str,external_id:&str,status:&str,version:Option<&str>,detail:&str)->DbResult<()> {
        sqlx::query("UPDATE installations SET update_status=?,installed_version=?,update_detail=?,update_checked_at=? WHERE source_id IN (SELECT id FROM game_sources WHERE provider=? AND external_id=?)")
            .bind(status).bind(version).bind(detail).bind(Utc::now().to_rfc3339()).bind(provider).bind(external_id).execute(&self.pool).await.map_err(|e|e.to_string())?;Ok(())
    }
    pub async fn reconcile_battlenet(&self,found:&[String])->DbResult<()> {
        let rows=sqlx::query("SELECT i.id,s.external_id FROM installations i JOIN game_sources s ON s.id=i.source_id WHERE s.provider='battlenet'").fetch_all(&self.pool).await.map_err(|e|e.to_string())?;
        for row in rows {let external:String=row.get("external_id");if !found.contains(&external){sqlx::query("UPDATE installations SET installed=0,update_status='unknown' WHERE id=?").bind(row.get::<String,_>("id")).execute(&self.pool).await.map_err(|e|e.to_string())?;}}
        Ok(())
    }

    pub async fn active_steam_account(&self) -> DbResult<Option<String>> {
        sqlx::query_scalar("SELECT value_json FROM settings WHERE key='steam_active_account'").fetch_optional(&self.pool).await.map_err(|e| e.to_string())
    }
    pub async fn confirmed_steam_artwork(&self, game_id: &str) -> DbResult<Option<u64>> {
        let id: Option<String> = sqlx::query_scalar("SELECT external_id FROM metadata_matches WHERE game_id=? AND provider='steam-artwork' AND confirmed=1")
            .bind(game_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id.and_then(|s| s.parse().ok()))
    }
    pub async fn confirm_steam_artwork(&self, game_id: &str, id: u64) -> DbResult<()> {
        sqlx::query("INSERT INTO metadata_matches(game_id,provider,external_id,confirmed) VALUES(?,'steam-artwork',?,1) ON CONFLICT(game_id,provider) DO UPDATE SET external_id=excluded.external_id,confirmed=1")
            .bind(game_id).bind(id.to_string()).execute(&self.pool).await.map_err(|e| e.to_string())?; Ok(())
    }
    pub async fn select_steam_account(&self, id: &str) -> DbResult<()> {
        sqlx::query("INSERT INTO settings(key,value_json) VALUES('steam_active_account',?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json")
            .bind(id).execute(&self.pool).await.map_err(|e| e.to_string())?; Ok(())
    }
    /// Catalogue imports create no installation and never replace a source's identity.
    pub async fn import_steam_catalogue(&self, account: &str, games: &[(String,String)], scope: &str) -> DbResult<usize> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        let mut inserted = 0;
        for (app_id,name) in games {
            if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) || name.trim().is_empty() || app_id == "228980" { continue; }
            let existing: Option<String> = sqlx::query_scalar("SELECT id FROM game_sources WHERE provider='steam' AND external_id=?")
                .bind(app_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
            let source = if let Some(id) = existing { id } else {
                inserted += 1;
                let game = Uuid::new_v4().to_string(); let source = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO games(id,name,normalized_name,created_at) VALUES(?,?,?,?)").bind(&game).bind(name).bind(name.to_lowercase()).bind(&now)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
                sqlx::query("INSERT INTO game_sources(id,game_id,provider,external_id,source_name) VALUES(?,?,'steam',?,?)").bind(&source).bind(&game).bind(app_id).bind(name)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
                source
            };
            sqlx::query("INSERT INTO account_games(provider,account_id,source_id,scope) VALUES('steam',?,?,?) ON CONFLICT(provider,account_id,source_id) DO UPDATE SET scope=CASE WHEN excluded.scope='owned' THEN 'owned' ELSE account_games.scope END")
                .bind(account).bind(source).bind(scope).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?; Ok(inserted)
    }
    pub async fn steam_library_scope(&self, account: &str) -> DbResult<String> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM account_games WHERE provider='steam' AND account_id=? AND scope='owned'").bind(account)
            .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(if count > 0 { "owned" } else { "cache" }.into())
    }

    pub async fn has_asset(&self, game_id: &str, kind: &str) -> DbResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_assets WHERE game_id=? AND asset_type=? AND priority>=100")
            .bind(game_id).bind(kind).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(count > 0)
    }

    pub async fn has_automatic_asset(&self, game_id: &str, kind: &str) -> DbResult<bool> {
        let paths: Vec<String> = sqlx::query_scalar("SELECT path FROM game_assets WHERE game_id=? AND asset_type=? AND priority<100")
            .bind(game_id).bind(kind).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(paths.iter().any(|path| Path::new(path).is_file()))
    }

    pub async fn custom_hero(&self,game_id:&str)->DbResult<bool>{
        let n:i64=sqlx::query_scalar("SELECT COUNT(*) FROM game_assets WHERE game_id=? AND asset_type='hero' AND priority>=100")
            .bind(game_id).fetch_one(&self.pool).await.map_err(|e|e.to_string())?;Ok(n>0)
    }
    pub async fn save_local_assets(&self, assets: Vec<(String,game_metadata::Artwork)>) -> DbResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for (game_id,asset) in assets {
            let asset=if asset.kind=="logo"{game_metadata::title::display(&asset)?}else{asset};
            let existing = sqlx::query("SELECT path,width,height,source FROM game_assets WHERE game_id=? AND asset_type=? AND priority<100")
                .bind(&game_id).bind(asset.kind).fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
            if existing.iter().any(|row|{let source=row.get::<String,_>("source");(source=="reviewed-promo-motion"||source.starts_with("background-promo-"))&&Path::new(&row.get::<String,_>("path")).is_file()}) {continue;}
            if asset.source != "battlenet-local" && existing.iter().any(|row| {
                let path: String = row.get("path");
                let width: Option<i64> = row.get("width");
                let height: Option<i64> = row.get("height");
                Path::new(&path).is_file() && if asset.source.starts_with("background-") {
                    (width.unwrap_or(0)*1080).min(height.unwrap_or(0)*1920)>=(asset.width as i64*1080).min(asset.height as i64*1920)
                }else{width.unwrap_or(0)*height.unwrap_or(0)>=asset.width as i64*asset.height as i64}
            }) { continue; }
            sqlx::query("DELETE FROM game_assets WHERE game_id=? AND asset_type=? AND priority<100")
                .bind(&game_id).bind(asset.kind).execute(&mut *tx).await.map_err(|e| e.to_string())?;
            sqlx::query("INSERT INTO game_assets(id,game_id,asset_type,path,source,priority,width,height) VALUES(?,?,?,?,?,10,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&game_id).bind(asset.kind)
                .bind(asset.path.to_string_lossy().to_string()).bind(asset.source)
                .bind(asset.width as i64).bind(asset.height as i64).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn save_scraped_asset(&self, game_id: &str, asset: &game_metadata::Artwork) -> DbResult<bool> {
        let prepared;if asset.kind=="logo"{prepared=game_metadata::title::display(asset)?;}else{prepared=asset.clone();}let asset=&prepared;
        if asset.kind=="logo" {
            if !game_metadata::branding::usable_logo(&asset.path){return Ok(false);}
            if !matches!(asset.source,"steam-local"|"title-logo-official"){
                let official:Vec<String>=sqlx::query_scalar("SELECT path FROM game_assets WHERE game_id=? AND asset_type='logo' AND source IN ('steam-local','title-logo-official') AND priority<100").bind(game_id).fetch_all(&self.pool).await.map_err(|e|e.to_string())?;
                if official.iter().any(|path|Path::new(path).is_file()){return Ok(false);}
            }
        }
        let source=if asset.kind=="logo"{if matches!(asset.source,"steam-local"|"title-logo-official"){"title-logo-official"}else{"title-logo-original"}}else{asset.source};
        // Keep the automatic fallback alongside manual overrides. Selection is by priority,
        // so manual artwork remains visible and resetting it can reveal cached automatic art.
        for attempt in 0..3 {
            let result=async {
                let mut tx=self.pool.begin().await.map_err(|e|e.to_string())?;
                sqlx::query("DELETE FROM game_assets WHERE game_id=? AND asset_type=? AND priority<100")
                    .bind(game_id).bind(asset.kind).execute(&mut *tx).await.map_err(|e|e.to_string())?;
                sqlx::query("INSERT INTO game_assets(id,game_id,asset_type,path,source,priority,width,height) VALUES(?,?,?,?,?,10,?,?)")
                    .bind(Uuid::new_v4().to_string()).bind(game_id).bind(asset.kind)
                    .bind(asset.path.to_string_lossy().to_string()).bind(source)
                    .bind(asset.width as i64).bind(asset.height as i64).execute(&mut *tx).await.map_err(|e|e.to_string())?;
                tx.commit().await.map_err(|e|e.to_string())?;Ok::<_,String>(true)
            }.await;
            match result {Err(e) if attempt<2&&e.contains("locked")=>tokio::time::sleep(std::time::Duration::from_millis(250*(attempt+1))).await,other=>return other}
        }
        Err("素材写入重试失败".into())
    }

    pub async fn screenshots(&self, game_id: &str) -> DbResult<Vec<String>> {
        sqlx::query_scalar("SELECT path FROM game_assets WHERE game_id=? AND asset_type LIKE 'screenshot_%' ORDER BY asset_type")
            .bind(game_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }
    pub async fn confirmed_sgdb_id(&self, game_id: &str) -> DbResult<Option<u64>> {
        let id: Option<String> = sqlx::query_scalar("SELECT external_id FROM metadata_matches WHERE game_id=? AND provider='steamgriddb' AND confirmed=1")
            .bind(game_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(id.and_then(|s| s.parse().ok()))
    }
    pub async fn confirm_sgdb(&self, game_id: &str, id: u64) -> DbResult<()> {
        sqlx::query("INSERT INTO metadata_matches(game_id,provider,external_id,confirmed) VALUES(?,'steamgriddb',?,1) ON CONFLICT(game_id,provider) DO UPDATE SET external_id=excluded.external_id,confirmed=1")
            .bind(game_id).bind(id.to_string()).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }
    pub async fn save_metadata(&self, game_id: &str, data: &game_metadata::GameMetadata) -> DbResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        if let Some(description) = &data.description {
            sqlx::query("UPDATE games SET description=? WHERE id=?")
                .bind(description).bind(game_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        if let Some(genre) = data.genres.first() {
            sqlx::query("UPDATE games SET genre=? WHERE id=?")
                .bind(genre).bind(game_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        for name in &data.genres {
            sqlx::query("INSERT OR IGNORE INTO tags(id,name) VALUES(?,?)")
                .bind(Uuid::new_v4().to_string()).bind(name).execute(&mut *tx).await.map_err(|e| e.to_string())?;
            sqlx::query("INSERT OR IGNORE INTO game_tags(game_id,tag_id) SELECT ?,id FROM tags WHERE name=?")
                .bind(game_id).bind(name).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    pub async fn artwork_choices(&self,game_id:&str)->DbResult<Vec<serde_json::Value>>{
        let mut choices=Vec::new();for kind in ["hero","poster","landscape","logo"]{
            if let Some(row)=sqlx::query("SELECT path,width,height,priority FROM game_assets WHERE game_id=? AND asset_type=? ORDER BY priority DESC LIMIT 1").bind(game_id).bind(kind).fetch_optional(&self.pool).await.map_err(|e|e.to_string())?{
                choices.push(serde_json::json!({"kind":kind,"path":row.get::<String,_>("path"),"width":row.get::<Option<i64>,_>("width"),"height":row.get::<Option<i64>,_>("height"),"manual":row.get::<i64,_>("priority")>=100}));
            }
        }Ok(choices)
    }
    pub async fn save_manual_asset(&self, game_id: &str, kind: &str, path: &Path, width: u32, height: u32) -> DbResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM game_assets WHERE game_id=? AND asset_type=? AND priority=100")
            .bind(game_id).bind(kind).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO game_assets(id,game_id,asset_type,path,source,priority,width,height) VALUES(?,?,?,?, 'manual',100,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(game_id).bind(kind).bind(path.to_string_lossy().to_string())
            .bind(width as i64).bind(height as i64).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    pub async fn reset_manual_asset(&self, game_id: &str, kind: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM game_assets WHERE game_id=? AND asset_type=? AND priority=100")
            .bind(game_id).bind(kind).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }
    pub async fn set_favorite(&self, game_id: &str, favorite: bool) -> DbResult<()> {
        let changed = sqlx::query("UPDATE games SET favorite=? WHERE id=?")
            .bind(favorite).bind(game_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        if changed.rows_affected() == 0 { return Err("游戏不存在".into()); }
        Ok(())
    }

    pub async fn launch_target(&self, installation_id: &str) -> DbResult<LaunchTarget> {
        let row = sqlx::query(r#"
            SELECT g.id as game_id,s.provider,s.external_id,i.install_path
            FROM installations i JOIN game_sources s ON s.id=i.source_id
            JOIN games g ON g.id=s.game_id
            WHERE i.id=? AND i.installed=1
        "#).bind(installation_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?
            .ok_or("游戏安装实例不存在或未安装")?;
        Ok(LaunchTarget {
            game_id: row.try_get("game_id").map_err(|e| e.to_string())?,
            provider: row.try_get("provider").map_err(|e| e.to_string())?,
            external_id: row.try_get("external_id").map_err(|e| e.to_string())?,
            install_path: row.try_get("install_path").map_err(|e| e.to_string())?,
        })
    }

    pub async fn start_session(&self, session_id: &str, game_id: &str, installation_id: &str, started_at: &str) -> DbResult<()> {
        sqlx::query("INSERT INTO play_sessions(id,game_id,installation_id,started_at,launch_state) VALUES(?,?,?,?,?)")
            .bind(session_id).bind(game_id).bind(installation_id).bind(started_at).bind("running")
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn finish_session(&self, session_id: &str, game_id: &str, duration_seconds: i64, exit_reason: &str) -> DbResult<()> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx::query("UPDATE play_sessions SET ended_at=?,duration_seconds=?,launch_state='exited',exit_reason=? WHERE id=?")
            .bind(&now).bind(duration_seconds).bind(exit_reason).bind(session_id)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        sqlx::query("UPDATE games SET total_minutes=total_minutes + ?,last_played_at=? WHERE id=?")
            .bind(duration_seconds / 60).bind(&now).bind(game_id)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

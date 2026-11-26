use super::*;
use game_metadata::epic::{OwnedGame,Snapshot};

impl Storage{
    pub async fn activate_epic_account(&self,account:&str)->DbResult<()>{
        let mut tx=self.pool.begin().await.map_err(|e|e.to_string())?;
        let previous:Option<String>=sqlx::query_scalar("SELECT value_json FROM settings WHERE key='epic_active_account'").fetch_optional(&mut *tx).await.map_err(|e|e.to_string())?;
        if previous.as_deref()!=Some(account){sqlx::query("DELETE FROM settings WHERE key='epic_last_synced_at'").execute(&mut *tx).await.map_err(|e|e.to_string())?;}
        sqlx::query("INSERT INTO settings(key,value_json) VALUES('epic_active_account',?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json").bind(account).execute(&mut *tx).await.map_err(|e|e.to_string())?;
        tx.commit().await.map_err(|e|e.to_string())?;Ok(())
    }
    pub async fn import_epic_library(&self,account:&str,snapshot:&Snapshot)->DbResult<usize>{
        if account.is_empty(){return Err("Epic 账号无效".into());}
        let mut tx=self.pool.begin().await.map_err(|e|e.to_string())?;let now=Utc::now().to_rfc3339();let mut inserted=0;
        // A partial metadata response adds confirmed entries and preserves all older entitlements.
        if snapshot.unresolved==0{sqlx::query("DELETE FROM account_games WHERE provider='epic' AND account_id=?").bind(account).execute(&mut *tx).await.map_err(|e|e.to_string())?;}
        for game in &snapshot.games{
            let existing=sqlx::query("SELECT id,game_id FROM game_sources WHERE provider='epic' AND external_id=?").bind(&game.app_name).fetch_optional(&mut *tx).await.map_err(|e|e.to_string())?;
            let(source,id)=if let Some(row)=existing{(row.get::<String,_>("id"),row.get::<String,_>("game_id"))}else{
                let(source,id)=(Uuid::new_v4().to_string(),Uuid::new_v4().to_string());
                sqlx::query("INSERT INTO games(id,name,normalized_name,created_at) VALUES(?,?,?,?)").bind(&id).bind(&game.name).bind(game.name.to_lowercase()).bind(&now).execute(&mut *tx).await.map_err(|e|e.to_string())?;
                sqlx::query("INSERT INTO game_sources(id,game_id,provider,external_id,source_name) VALUES(?,?,'epic',?,?)").bind(&source).bind(&id).bind(&game.app_name).bind(&game.name).execute(&mut *tx).await.map_err(|e|e.to_string())?;inserted+=1;(source,id)
            };
            sqlx::query("INSERT OR REPLACE INTO account_games(provider,account_id,source_id,scope) VALUES('epic',?,?,'owned')").bind(account).bind(&source).execute(&mut *tx).await.map_err(|e|e.to_string())?;
            sqlx::query("INSERT INTO metadata_matches(game_id,provider,external_id,confirmed) VALUES(?,'epic-identity',?,1) ON CONFLICT(game_id,provider) DO UPDATE SET external_id=excluded.external_id,confirmed=1").bind(&id).bind(format!("{}:{}:{}",game.namespace,game.catalog_id,game.app_name)).execute(&mut *tx).await.map_err(|e|e.to_string())?;
            if let Some(description)=&game.description{sqlx::query("UPDATE games SET description=? WHERE id=? AND (description IS NULL OR description='')").bind(description).bind(&id).execute(&mut *tx).await.map_err(|e|e.to_string())?;}
        }
        sqlx::query("INSERT INTO settings(key,value_json) VALUES('epic_active_account',?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json").bind(account).execute(&mut *tx).await.map_err(|e|e.to_string())?;
        sqlx::query("INSERT INTO settings(key,value_json) VALUES('epic_last_synced_at',?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json").bind(&now).execute(&mut *tx).await.map_err(|e|e.to_string())?;
        tx.commit().await.map_err(|e|e.to_string())?;Ok(inserted)
    }
    pub async fn epic_library_status(&self)->DbResult<serde_json::Value>{
        let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM account_games WHERE provider='epic' AND account_id=(SELECT value_json FROM settings WHERE key='epic_active_account')").fetch_one(&self.pool).await.map_err(|e|e.to_string())?;
        let synced:Option<String>=sqlx::query_scalar("SELECT value_json FROM settings WHERE key='epic_last_synced_at'").fetch_optional(&self.pool).await.map_err(|e|e.to_string())?;
        Ok(serde_json::json!({"count":count,"synced_at":synced}))
    }
    pub async fn disconnect_epic_library(&self)->DbResult<()>{sqlx::query("DELETE FROM settings WHERE key IN ('epic_active_account','epic_last_synced_at')").execute(&self.pool).await.map_err(|e|e.to_string())?;Ok(())}
    pub async fn epic_install_identity(&self,id:&str)->DbResult<String>{
        sqlx::query_scalar("SELECT COALESCE((SELECT external_id FROM metadata_matches WHERE game_id=s.game_id AND provider='epic-identity' AND confirmed=1),s.external_id) FROM game_sources s WHERE s.game_id=? AND s.provider='epic' AND EXISTS(SELECT 1 FROM account_games a WHERE a.source_id=s.id AND a.provider='epic' AND a.account_id=(SELECT value_json FROM settings WHERE key='epic_active_account'))")
            .bind(id).fetch_optional(&self.pool).await.map_err(|e|e.to_string())?.ok_or("此游戏未关联当前 Epic 账号，请先同步账号库".into())
    }
    pub async fn cache_epic_covers(&self,scraper:game_metadata::Scraper,games:Vec<OwnedGame>,root:&Path)->DbResult<usize>{
        let current=self.list_games().await?;let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(4));let mut tasks=tokio::task::JoinSet::new();
        for game in games{let Some(target)=current.iter().find(|g|g.platform=="epic"&&g.external_id==game.app_name)else{continue};
            if target.poster.is_some()&&target.landscape.is_some(){continue;}
            let(id,root,scraper,limit)=(target.id.clone(),root.to_owned(),scraper.clone(),limit.clone());
            tasks.spawn(async move{let _permit=limit.acquire_owned().await.map_err(|e|e.to_string())?;Ok::<_,String>(scraper.epic_covers(&game,&id,&root).await.into_iter().map(|a|(id.clone(),a)).collect::<Vec<_>>())});
        }
        let mut assets=Vec::new();while let Some(task)=tasks.join_next().await{if let Ok(Ok(rows))=task{assets.extend(rows);}}let count=assets.len();self.save_local_assets(assets).await?;Ok(count)
    }
}
#[cfg(test)]mod tests{use super::*;
 fn game(app:&str)->OwnedGame{OwnedGame{app_name:app.into(),namespace:"space".into(),catalog_id:format!("item-{app}"),name:format!("Game {app}"),description:None,poster:None,landscape:None}}
 #[test]fn account_library_imports_uninstalled_without_duplicating_installations_or_other_accounts(){tokio::runtime::Runtime::new().unwrap().block_on(async{
  let root=std::env::temp_dir().join(format!("gamedeck-epic-{}",Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();let db=Storage::open(&root.join("test.db")).await.unwrap();
  db.upsert_discovered(vec![DiscoveredGame{provider:"epic".into(),external_id:"A".into(),name:"Game A".into(),install_path:root.to_string_lossy().into()}],vec![]).await.unwrap();
  let snapshot=Snapshot{games:vec![game("A"),game("B")],total_records:2,skipped:0,unresolved:0};assert_eq!(db.import_epic_library("one",&snapshot).await.unwrap(),1);assert_eq!(db.import_epic_library("one",&snapshot).await.unwrap(),0);
  let games=db.list_games().await.unwrap();assert_eq!(games.len(),2);let b=games.iter().find(|g|g.external_id=="B").unwrap();assert!(!b.installed&&b.launch_installation_id.is_none());assert_eq!(db.epic_install_identity(&b.id).await.unwrap(),"space:item-B:B");let id=b.id.clone();db.set_favorite(&id,true).await.unwrap();
  db.import_epic_library("two",&Snapshot{games:vec![game("C")],total_records:1,skipped:0,unresolved:0}).await.unwrap();let games=db.list_games().await.unwrap();assert_eq!(games.len(),2);assert!(games.iter().all(|g|g.external_id!="B"));
  db.import_epic_library("one",&snapshot).await.unwrap();assert!(db.list_games().await.unwrap().iter().find(|g|g.id==id).unwrap().favorite);
  db.import_epic_library("one",&Snapshot{games:vec![],total_records:2,skipped:0,unresolved:2}).await.unwrap();assert_eq!(db.list_games().await.unwrap().len(),2);
  db.disconnect_epic_library().await.unwrap();assert_eq!(db.list_games().await.unwrap().len(),1);db.pool.close().await;std::fs::remove_dir_all(root).unwrap();
 });}
}

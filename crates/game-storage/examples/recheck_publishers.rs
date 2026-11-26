//! Refresh the publisher adapter after a completed whole-library pass, then verify every selection.
#[allow(dead_code)]
#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"] mod credentials;
pub fn main(){tokio::runtime::Runtime::new().unwrap().block_on(async {
    let data=std::env::current_dir().unwrap().join("release/data");
    let artwork=data.join("artwork");let reports=artwork.join("reports");
    let db=game_storage::Storage::open(&data.join("database/gamedeck.db")).await.unwrap();
    let mut report:serde_json::Value=serde_json::from_slice(&std::fs::read(reports.join("background-refresh-latest.json")).unwrap()).unwrap();
    assert_eq!(report["processed"],report["total"],"Wait for the whole-library pass to finish");
    let recovered_errors=report["errors"].clone();
    let mut tasks=tokio::task::JoinSet::new();let limit=std::sync::Arc::new(tokio::sync::Semaphore::new(3));
    let scraper=game_metadata::Scraper::new(credentials::load_sgdb(&data).unwrap()).unwrap().with_alpha_key(credentials::load(&data).unwrap());
    for game in db.games_for_scraping().await.unwrap() {
        let audit=std::fs::read(artwork.join("hero").join(format!("{}-promo-report.json",game.id))).ok().and_then(|b|serde_json::from_slice::<serde_json::Value>(&b).ok()).unwrap_or_default();
        let transport_failed=audit["candidates"].as_array().is_some_and(|cs|cs.iter().any(|c|c["reason"].as_str().is_some_and(|r|r.contains("原图读取失败")||r.contains("原图请求失败/超时"))));
        let press_failure=audit["candidates"].as_array().is_some_and(|cs|cs.iter().any(|c|c["source"]=="publisher-press"&&c["reason"].as_str().is_some_and(|r|r.contains("HTTP 403"))));
        if std::env::var("GAMEDECK_RECHECK_PRESS_FAILURES_ONLY").as_deref()==Ok("1")&&!press_failure{continue;}
        let missing=!report["entries"].as_array().unwrap().iter().any(|e|e["id"]==game.id);
        let provider_only=std::env::var("GAMEDECK_RECHECK_PROVIDER_ONLY").as_deref()==Ok("1");
        if game.provider!="battlenet"&&(provider_only||(!transport_failed&&!missing)){continue;}
        let (db,scraper,artwork,limit)=(db.clone(),scraper.clone(),artwork.clone(),limit.clone());
        tasks.spawn(async move {let _permit=limit.acquire_owned().await.unwrap();db.refresh_promo_game(&scraper,&game,&artwork).await});
    }
    while let Some(result)=tasks.join_next().await { let result=result.unwrap().unwrap();

        println!("Program recheck: {} {}",result["game"],result["status"]);
        let entries=report["entries"].as_array_mut().unwrap();
        if let Some(old)=entries.iter_mut().find(|e|e["id"]==result["id"]){*old=result;}else{entries.push(result);}
    }
    report["recovered_errors"]=recovered_errors;report["errors"]=serde_json::json!([]);
    assert_eq!(report["entries"].as_array().unwrap().len() as u64,report["total"].as_u64().unwrap());
    report["saved"]=report["entries"].as_array().unwrap().iter().filter(|e|e["status"]=="updated").count().into();
    let games=db.list_games().await.unwrap();let mut verified=Vec::new();
    for entry in report["entries"].as_array().unwrap(){
        let game=games.iter().find(|g|Some(g.id.as_str())==entry["id"].as_str()).unwrap();
        let ok=match entry["status"].as_str().unwrap_or("") {
            "updated"=>game.hero.as_deref()==entry["path"].as_str()&&game.hero.as_ref().is_some_and(|p|std::path::Path::new(p).is_file())&&entry["display_cache"].as_str().is_some_and(|p|std::path::Path::new(p).is_file()),
            "manual-preserved"=>db.custom_hero(&game.id).await.unwrap(),
            "no-verified-resource"=>game.hero.is_none(),_=>false,
        };
        assert!(ok,"Persisted artwork differs from report: {}",game.name);
        verified.push(serde_json::json!({"id":game.id,"game":game.name,"verified":ok,"hero":game.hero,"source":game.hero_source}));
    }
    std::fs::write(reports.join("background-verification.json"),serde_json::to_vec_pretty(&verified).unwrap()).unwrap();
    std::fs::write(reports.join("background-refresh-latest.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("Verified all {} persisted selections and display caches",verified.len());
});}

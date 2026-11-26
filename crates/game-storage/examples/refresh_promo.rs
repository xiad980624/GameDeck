#[allow(dead_code)]
#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"] mod credentials;
fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let root=std::env::current_dir().unwrap().join("release/data");
        let db=game_storage::Storage::open(&root.join("database/gamedeck.db")).await.unwrap();
        let scraper=game_metadata::Scraper::new(credentials::load_sgdb(&root).unwrap()).unwrap()
            .with_alpha_key(credentials::load(&root).unwrap());
        println!("Starting programmatic library refresh (policy {})",game_metadata::promo::POLICY);
        let report=db.refresh_promo_backgrounds(scraper,&root.join("artwork"),|p| {
            println!("{}/{} {} {}",p["processed"],p["total"],p["last"]["game"],p["last"]["status"]);
        }).await.unwrap();
        let games=db.list_games().await.unwrap();
        let mut verification=Vec::new();
        for entry in report["entries"].as_array().unwrap() {
            let game=games.iter().find(|g|Some(g.id.as_str())==entry["id"].as_str()).unwrap();
            let ok=match entry["status"].as_str().unwrap_or("") {
                "updated"=>game.hero.as_deref()==entry["path"].as_str()
                    && game.hero.as_ref().is_some_and(|p|std::path::Path::new(p).is_file())
                    && entry["display_cache"].as_str().is_some_and(|p|std::path::Path::new(p).is_file()),
                "manual-preserved"=>db.custom_hero(&game.id).await.unwrap(),
                "no-verified-resource"=>game.hero.is_none(),
                _=>false,
            };
            verification.push(serde_json::json!({"id":game.id,"game":game.name,"verified":ok,"hero":game.hero,"source":game.hero_source}));
        }
        assert_eq!(verification.len() as u64, report["total"].as_u64().unwrap());
        let verified=verification.iter().filter(|v|v["verified"]==true).count();
        std::fs::write(root.join("artwork/reports/background-verification.json"),serde_json::to_vec_pretty(&verification).unwrap()).unwrap();
        println!("Verified persisted selections and display caches: {verified}/{}",verification.len());
        assert_eq!(verified,verification.len(),"A persisted selection did not match the program report");
        assert_eq!(report["processed"],report["total"]);
        assert!(report["errors"].as_array().unwrap().is_empty());
        println!("Completed: processed={} saved={} errors={}",report["processed"],report["saved"],report["errors"]);
    });
}

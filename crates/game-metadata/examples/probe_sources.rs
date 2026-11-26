#[allow(dead_code)]
#[path="../../../apps/desktop/src-tauri/src/alpha_credentials.rs"] mod credentials;
fn main(){tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    let root=std::env::current_dir().unwrap();let data=root.join("release/data");let reports=data.join("artwork/reports");
    let summary:serde_json::Value=serde_json::from_slice(&std::fs::read(reports.join("background-refresh-summary.json")).unwrap()).unwrap();
    let mut snapshots:Vec<_>=std::fs::read_dir(&reports).unwrap().flatten().map(|e|e.path()).filter(|p|p.file_name().unwrap().to_string_lossy().starts_with("background-before-")).collect();snapshots.sort();
    let games:serde_json::Value=serde_json::from_slice(&std::fs::read(snapshots.last().unwrap()).unwrap()).unwrap();
    let key=credentials::load_sgdb(&data).unwrap();let scraper=game_metadata::Scraper::new(key.clone()).unwrap();
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build().unwrap();let mut results=Vec::new();
    for game in games.as_array().unwrap().iter().filter(|g|summary["missing"].as_array().unwrap().iter().any(|m|m["game"]==g["name"])){
        let name=game["original_name"].as_str().unwrap();let mut row=serde_json::json!({"id":game["id"],"game":game["name"],"original_name":name,"provider":game["platform"]});
        if game["platform"]!="steam"{
            let term=if game["external_id"]=="diablo_iii_cn"{"Diablo III"}else{name};
            match scraper.candidates(term).await {
                Ok(hits)=>{let exact:Vec<_>=hits.into_iter().filter(|h|h.exact).collect();row["sgdb_matches"]=serde_json::to_value(&exact).unwrap();
                    if exact.len()==1{let r=client.get(format!("https://www.steamgriddb.com/api/v2/games/id/{}",exact[0].id)).query(&[("platformdata","steam")]).bearer_auth(key.as_deref().unwrap_or("")).send().await.unwrap();row["sgdb_status"]=r.status().as_u16().into();row["sgdb_data"]=r.json::<serde_json::Value>().await.unwrap_or_default();}
                },Err(_)=>row["error"]="SGDB lookup failed".into(),
            }
        }else{
            let id=game["external_id"].as_str().unwrap().parse::<u64>().unwrap();let request=serde_json::json!({"ids":[{"appid":id}],"context":{"language":"english","country_code":"US"},"data_request":{"include_assets":true}}).to_string();
            if let Ok(r)=client.get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/").query(&[("input_json",request)]).send().await{row["store_data"]=r.json::<serde_json::Value>().await.unwrap_or_default();}
        }
        println!("Probed {}",game["name"]);results.push(row);
    }
    std::fs::create_dir_all(root.join("docs/background-followup")).unwrap();std::fs::write(root.join("docs/background-followup/probe.json"),serde_json::to_vec_pretty(&results).unwrap()).unwrap();
});}

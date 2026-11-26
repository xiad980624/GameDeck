fn main() {
    let catalogue = game_discovery::appinfo::catalogue();
    println!("appinfo names: {}",catalogue.len());
    let (installed,_) = game_discovery::steam::scan();
    for game in installed { if let Some(app) = catalogue.get(&game.external_id) { println!("Title {}: {} | Chinese: {}",app.id,app.name,app.name_zh.as_deref().unwrap_or("-")); } }
    match game_discovery::playtime::read_steam_playtime() {
        Ok(snapshot) => println!("Steam cached history: {}, named games: {}",snapshot.records.len(),snapshot.games.len()),
        Err(error) => println!("Steam cache: {error}"),
    }
    let (games,errors) = game_discovery::epic::scan();
    for game in games { println!("Epic: {} | qualified identity: {}",game.name,game_discovery::epic::identity(&game.external_id).is_some()); }
    println!("Epic scan errors: {}",errors.len());
}

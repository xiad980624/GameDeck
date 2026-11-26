fn main() {
    let output = std::env::temp_dir().join("gamedeck-epic-cache-probe");
    let caches = game_metadata::epic_cache::cache_roots();
    println!("Public image cache roots: {}",caches.len());
    for (name,namespace) in [("Borderlands 3","catnip"),("Cyberpunk 2077","77f2b98e2cef40c8a7437518bf420e47"),("Death Stranding","f4a904fcef2447439c35c4e6457f3027"),("Surviving Mars","d759128018124dcabb1fbee9bb28e178")] {
        let game = game_metadata::GameRef {id:name.replace(' ',"-"),name:name.into(),provider:"epic".into(),external_id:String::new()};
        match game_metadata::epic_cache::import(&game,namespace,&caches,&output) {
            Ok(assets) => for asset in assets { println!("{name}: {} {}x{} ({})",asset.kind,asset.width,asset.height,asset.path.extension().unwrap().to_string_lossy()); },
            Err(error) => println!("{name}: {error}"),
        }
    }
    for (id,name) in [("diablo_iv","Diablo IV"),("diablo_iii","Diablo III"),("diablo_iii_cn","Diablo III (China)")] {
        let game=game_metadata::GameRef{id:id.into(),name:name.into(),provider:"battlenet".into(),external_id:id.into()};
        match game_metadata::battlenet_cache::import(&game,&output) {Ok(assets)=>{println!("Battle.net {name}: {} images",assets.len());for a in assets{println!("  {} {}x{}",a.kind,a.width,a.height);}},Err(e)=>println!("{name}: {e}")}
    }
}

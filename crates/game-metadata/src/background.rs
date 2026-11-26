//! Background-only scraping: identity -> semantic source -> decoded quality -> immutable cache.
use super::*;
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct BackgroundRecord {pub game_id:String,pub source:String,pub url:String,pub width:u32,pub height:u32,pub grade:String}
#[derive(Clone,Serialize,Deserialize)]
pub struct BackgroundChoice {pub id:String,pub path:PathBuf,pub width:u32,pub height:u32,pub animated:bool,#[serde(default)]pub duration_ms:u64,pub title:String,pub source:String,pub score:i32,pub reason:String,#[serde(default)]pub policy_version:u32,#[serde(default)]pub proof:String,#[serde(default)]pub title_detected:bool}
pub fn choices(root:&Path,game_id:&str)->Vec<BackgroundChoice>{
 let mut result=Vec::new();if let Ok(entries)=std::fs::read_dir(root.join("hero")){for e in entries.flatten(){if e.file_name().to_string_lossy().starts_with(&format!("{game_id}-background-"))&&e.file_name().to_string_lossy().ends_with(".candidate.json"){if let Ok(bytes)=std::fs::read(e.path()){if let Ok(c)=serde_json::from_slice::<BackgroundChoice>(&bytes){if c.policy_version==crate::promo::POLICY&&!c.proof.is_empty()&&!c.animated&&c.path.is_file()&&c.path.parent()==Some(root.join("hero").as_path()){result.push(c);}}}}}}
 result.sort_by_key(|c|std::cmp::Reverse(quality(c.width,c.height,c.score,c.animated&&(3000..=6000).contains(&c.duration_ms))));result
}
pub fn suitable(width:u32,height:u32)->bool{
    width>=2560&&height>=1440&&width<=10000&&height<=10000&&(1.5..=3.6).contains(&(width as f64/height as f64))
}
fn quality(width:u32,height:u32,score:i32,animated:bool)->(bool,i32,u64,u64,bool){let r=rank(width,height);(width>=3840&&height>=2160,score,r.0,r.1,animated)}
fn rank(width:u32,height:u32)->(u64,u64){((width as u64*1080).min(height as u64*1920),width as u64*height as u64)}
#[cfg(test)]
fn grade(width:u32,height:u32)->String{if width>=3840&&height>=2160{"4K"}else{"HD"}.into()}
#[cfg(test)]
fn game_wallpaper_tags(tags:&serde_json::Value,term:&str)->bool{
 let Some(tags)=tags.as_array()else{return false;};
 let exact=tags.iter().any(|t|t.get("name").and_then(|n|n.as_str()).is_some_and(|n|normalize(n)==normalize(term)));
 let game=tags.iter().any(|t|t.get("id").and_then(|id|id.as_u64())==Some(961)||t.get("name").and_then(|n|n.as_str()).is_some_and(|n|["videogame","videogames","电子游戏"].contains(&normalize(n).as_str())));
 exact&&game
}
#[cfg(test)]
fn trusted(url:&str,provider:&str,steam_id:Option<u64>)->bool{
    let Ok(url)=reqwest::Url::parse(url)else{return false};if url.scheme()!="https"{return false;}
    let host=url.host_str().unwrap_or("");
    match provider{
        "background-steam"=>steam_id.is_some_and(|id|(host.ends_with(".steamstatic.com")||host=="steamcdn-a.akamaihd.net")&&url.path().contains(&format!("/apps/{id}/"))&&url.path().contains("library_hero")&&!url.path().contains("blur")),
        "background-steamgriddb"=>host=="cdn2.steamgriddb.com"||host=="cdn.steamgriddb.com",
        "background-blizzard"=>host=="bnetproduct-a.akamaihd.net",
        "background-alphacoders"=>host.strip_suffix(".alphacoders.com").is_some_and(|s|s=="images"||s.strip_prefix("images").is_some_and(|n|!n.is_empty()&&n.chars().all(|c|c.is_ascii_digit())))&&!url.path().contains("thumb-")&&url.query().is_none(),
        _=>false,
    }
}
impl Scraper{
    pub async fn scrape_background(&self,game:&GameRef,root:&Path,confirmed_sgdb:Option<u64>,confirmed_steam:Option<u64>,force:bool)->Result<Artwork,String>{
        self.scrape_promo_background(game,root,confirmed_sgdb,confirmed_steam,force).await
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn generic_word_alone_does_not_identify_a_game(){let fruit=serde_json::json!([{"name":"banana","id":27},{"name":"food"}]);assert!(!game_wallpaper_tags(&fruit,"Banana"));let game=serde_json::json!([{"name":"Banana"},{"id":961,"name":"video game"}]);assert!(game_wallpaper_tags(&game,"Banana"));assert!(!game_wallpaper_tags(&game,"Other Game"));}
    #[test]fn four_k_beats_low_resolution_animation_and_screenshot_loses_at_same_quality(){assert!(quality(3840,2160,0,false)>quality(2560,1440,30,true));assert!(quality(3840,2160,30,false)>quality(7680,4320,-30,true));assert!(quality(3840,2160,0,true)>quality(3840,2160,0,false));}
    #[test]fn alpha_accepts_only_original_images_on_image_hosts(){
        assert!(trusted("https://images8.alphacoders.com/136/1362741.jpeg","background-alphacoders",None));
        for url in ["http://images8.alphacoders.com/136/1362741.jpeg","https://images8.alphacoders.com.evil.test/1.jpg","https://wall.alphacoders.com/big.php?i=1","https://images.alphacoders.com/136/thumb-1920-1.jpg","https://images.evil.alphacoders.com/1.jpg","https://images.alphacoders.com/1.jpg?auth=secret"]{assert!(!trusted(url,"background-alphacoders",None),"{url}");}
    }
    #[test]fn rejects_thumbnails_posters_and_short_banners(){for (w,h) in [(460,215),(1920,620),(600,900),(3840,500),(1200,1200)]{assert!(!suitable(w,h));}assert!(!suitable(1920,1080));assert!(!suitable(3840,1240));assert!(suitable(2560,1440));assert!(suitable(3840,2160));}
    #[test]fn scores_pixels_that_survive_screen_cropping(){assert!(rank(3840,2160)>rank(3840,1240));assert!(rank(2560,1440)>rank(3840,1080));assert_eq!(grade(3840,1240),"HD");}
    #[test]fn refuses_screenshots_even_on_trusted_steam_cdn(){assert!(!trusted("https://shared.fastly.steamstatic.com/steam/apps/570/ss_1.jpg","background-steam",Some(570)));assert!(trusted("https://shared.fastly.steamstatic.com/steam/apps/570/library_hero.jpg","background-steam",Some(570)));assert!(!trusted("https://evil.steamstatic.com.fake/steam/apps/570/library_hero.jpg","background-steam",Some(570)));}
}

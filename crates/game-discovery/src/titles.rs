use std::collections::HashMap;
use super::appinfo::AppInfo;

// Conventional Chinese titles for entries whose store cache still uses an English name.
// Steam matches by AppID; other platforms require an exact normalized original title.
const TITLES: &[(&str,&str,&str)] = &[
    ("413150","Stardew Valley","星露谷物语"),
    ("588650","Dead Cells","死亡细胞"),
    ("646570","Slay the Spire","杀戮尖塔"),
    ("105600","Terraria","泰拉瑞亚"),
    ("219990","Grim Dawn","恐怖黎明"),
    ("1948280","Stacklands","堆叠大陆"),
    ("1145360","Hades","哈迪斯"),
    ("1145350","Hades II","哈迪斯Ⅱ"),
    ("1091500","Cyberpunk 2077","赛博朋克2077"),
    ("397540","Borderlands 3","无主之地3"),
    ("1190460","Death Stranding","死亡搁浅"),
    ("464920","Surviving Mars","火星求生"),
    ("2344520","Diablo IV","暗黑破坏神Ⅳ"),
    ("367520","Hollow Knight","空洞骑士"),
    ("1245620","ELDEN RING","艾尔登法环"),
    ("2358720","Black Myth: Wukong","黑神话：悟空"),
    ("1086940","Baldur's Gate 3","博德之门3"),
    ("1174180","Red Dead Redemption 2","荒野大镖客：救赎2"),
    ("1551360","Forza Horizon 5","极限竞速：地平线5"),
    ("292030","The Witcher 3: Wild Hunt","巫师3：狂猎"),
    ("814380","Sekiro: Shadows Die Twice","只狼：影逝二度"),
];

fn han(c: char) -> bool { matches!(c as u32,0x3400..=0x9fff | 0x20000..=0x323af) }
fn normalized(name: &str) -> String { name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect() }

/// Prefer the Chinese part of a bilingual store title without translating unknown names.
fn chinese_part(name: &str) -> Option<String> {
    let first = name.char_indices().find(|(_,c)| han(*c))?.0;
    let (last,c) = name.char_indices().rev().find(|(_,c)| han(*c))?;
    let chinese_end = last+c.len_utf8();
    let tail = name[chinese_end..].trim();
    // Keep sequel numbers, including II/IV; discard an English subtitle repeated after Chinese.
    let roman_or_number = !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit() || " IVXLCDMivxlcdmⅡⅢⅣⅤⅥⅦⅧⅨⅩ：:-·®™()（）".contains(c));
    let end = if tail.chars().any(|c| c.is_ascii_alphabetic()) && !roman_or_number { chinese_end } else { name.len() };
    Some(name[first..end].trim().trim_matches(|c:char| c.is_whitespace() || "-·()（）".contains(c)).to_string())
}

pub fn chinese_title(provider: &str, id: &str, original: &str, catalogue: &HashMap<String,AppInfo>) -> Option<String> {
    if provider=="battlenet" { if let Some(title)=match id {"diablo_iv"=>Some("暗黑破坏神Ⅳ"),"diablo_iii"=>Some("暗黑破坏神Ⅲ · 国际服"),"diablo_iii_cn"=>Some("暗黑破坏神Ⅲ · 国服"),"world_of_warcraft"=>Some("魔兽世界"),"hearthstone"=>Some("炉石传说"),"overwatch"=>Some("守望先锋"),"starcraft_ii"=>Some("星际争霸Ⅱ"),"starcraft"=>Some("星际争霸"),"heroes_of_the_storm"=>Some("风暴英雄"),"warcraft_iii"=>Some("魔兽争霸Ⅲ"),"diablo_ii_resurrected"=>Some("暗黑破坏神Ⅱ：重制版"),_=>None} {return Some(title.into());} }
    let app = if provider == "steam" { catalogue.get(id) } else { catalogue.values().find(|app| normalized(&app.name)==normalized(original) && app.kind=="game") };
    if let Some(title) = app.and_then(|a| a.name_zh.as_deref()).and_then(chinese_part) { return Some(title); }
    let known = TITLES.iter().find(|(app_id,name,_)| if provider == "steam" { *app_id==id } else { normalized(name)==normalized(original) });
    known.map(|(_,_,title)| (*title).to_string()).or_else(|| chinese_part(original))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn preserves_sequel_numbers_and_removes_repeated_english() {
        assert_eq!(chinese_part("Vampire Survivors - 吸血鬼幸存者").as_deref(),Some("吸血鬼幸存者"));
        assert_eq!(chinese_part("潜水员戴夫 DAVE THE DIVER").as_deref(),Some("潜水员戴夫"));
        assert_eq!(chinese_part("土豆兄弟(Brotato)").as_deref(),Some("土豆兄弟"));
        assert_eq!(chinese_part("无主之地®4").as_deref(),Some("无主之地®4"));
        assert_eq!(chinese_part("暗黑破坏神 IV").as_deref(),Some("暗黑破坏神 IV"));
        assert!(chinese_part("Unknown Game").is_none());
    }
    #[test] fn title_alias_never_changes_an_unrelated_platform_id() {
        let catalogue = HashMap::new();
        assert_eq!(chinese_title("steam","413150","Stardew Valley",&catalogue).as_deref(),Some("星露谷物语"));
        assert!(chinese_title("steam","999999","Stardew Valley",&catalogue).is_none());
        assert_eq!(chinese_title("epic","Ginger","Cyberpunk 2077",&catalogue).as_deref(),Some("赛博朋克2077"));
    }
}

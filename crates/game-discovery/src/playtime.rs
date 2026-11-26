use serde_json::{Map, Value};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PlaytimeRecord { pub app_id: String, pub minutes: i64, pub last_played: Option<i64> }
#[derive(Debug)]
pub struct SteamPlaytime { pub account_id: String, pub records: Vec<PlaytimeRecord>, pub games: Vec<super::appinfo::AppInfo> }
#[derive(Debug, Clone, serde::Serialize)]
pub struct SteamAccount { pub id: String, pub name: String, pub most_recent: bool }

// VDF contains nested objects; matching a flat regex across applications mixes their data.
fn parse_vdf(input: &str) -> Result<Value, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() || chars[i] == '\u{feff}' { i += 1; continue; }
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' { i += 1; }
            continue;
        }
        if chars[i] == '{' || chars[i] == '}' { tokens.push(chars[i].to_string()); i += 1; continue; }
        let quoted = chars[i] == '"';
        if quoted { i += 1; }
        let mut text = String::new();
        while i < chars.len() {
            let c = chars[i];
            if quoted && c == '"' { i += 1; break; }
            if !quoted && (c.is_whitespace() || c == '{' || c == '}') { break; }
            if quoted && c == '\\' && matches!(chars.get(i + 1), Some('"' | '\\')) { i += 1; }
            text.push(chars[i]); i += 1;
        }
        tokens.push(text);
    }
    fn object(tokens: &[String], index: &mut usize, depth: usize) -> Result<Value, String> {
        if depth > 64 { return Err("Steam 配置嵌套过深".into()); }
        let mut result = Map::new();
        while *index < tokens.len() {
            if tokens[*index] == "}" {
                if depth == 0 { return Err("Steam 配置括号不匹配".into()); }
                *index += 1;
                return Ok(Value::Object(result));
            }
            let key = tokens[*index].clone(); *index += 1;
            let token = tokens.get(*index).ok_or("Steam 配置缺少值")?; *index += 1;
            let value = if token == "{" { object(tokens,index,depth+1)? } else { Value::String(token.clone()) };
            result.insert(key,value);
        }
        if depth != 0 { return Err("Steam 配置不完整".into()); }
        Ok(Value::Object(result))
    }
    object(&tokens, &mut 0, 0)
}
fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.as_object()?.iter().find(|(key,_)| key.eq_ignore_ascii_case(name)).map(|(_,value)| value)
}
fn records(input: &str) -> Result<Vec<PlaytimeRecord>, String> {
    let value = parse_vdf(input)?;
    let mut node = &value;
    for name in ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"] {
        node = field(node,name).ok_or("Steam 配置中没有游戏时长")?;
    }
    Ok(node.as_object().ok_or("Steam 游戏时长格式错误")?.iter().filter_map(|(app_id,app)| {
        if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) { return None; }
        let minutes = field(app,"Playtime")?.as_str()?.parse::<i64>().ok()?;
        if minutes < 0 { return None; }
        let last_played = field(app,"LastPlayed").and_then(Value::as_str).and_then(|s| s.parse().ok()).filter(|n| *n > 0);
        Some(PlaytimeRecord { app_id: app_id.clone(), minutes, last_played })
    }).collect())
}

pub fn steam_accounts() -> Vec<SteamAccount> {
    let mut accounts = std::collections::BTreeMap::new();
    for root in super::steam::steam_roots() {
        if let Ok(text) = std::fs::read_to_string(root.join("config/loginusers.vdf")) {
            if let Ok(value) = parse_vdf(&text) {
                if let Some(users) = field(&value,"users").and_then(Value::as_object) {
                    for (steam_id, user) in users {
                        let Ok(steam_id) = steam_id.parse::<u64>() else { continue };
                        let id = (steam_id & 0xffff_ffff).to_string();
                        if !root.join("userdata").join(&id).join("config/localconfig.vdf").is_file() { continue; }
                        let name = field(user,"PersonaName").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or(&id).to_string();
                        let most_recent = field(user,"MostRecent").and_then(Value::as_str) == Some("1");
                        accounts.insert(id.clone(), SteamAccount { id, name, most_recent });
                    }
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(root.join("userdata")) {
            for entry in entries.flatten() {
                let id = entry.file_name().to_string_lossy().to_string();
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) && entry.path().join("config/localconfig.vdf").is_file() {
                    accounts.entry(id.clone()).or_insert(SteamAccount { name: format!("Steam {id}"), id, most_recent: false });
                }
            }
        }
    }
    accounts.into_values().collect()
}

pub fn read_steam_playtime_for(selected: Option<&str>) -> Result<SteamPlaytime, String> {
    let accounts = steam_accounts();
    let account = match selected.filter(|s| !s.is_empty()) {
        Some(id) => accounts.iter().find(|a| a.id == id).ok_or("选定 Steam 账号的本地缓存不可用，请重新选择账号")?,
        None => accounts.iter().find(|a| a.most_recent).or_else(|| if accounts.len() == 1 { accounts.first() } else { None }).ok_or("请在设置中选择要同步的 Steam 账号")?,
    };
    for root in super::steam::steam_roots() {
        let path = root.join("userdata").join(&account.id).join(Path::new("config/localconfig.vdf"));
        if !path.is_file() { continue; }
        let text = std::fs::read_to_string(path).map_err(|e| format!("读取当前 Steam 账号的时长失败：{e}"))?;
        let records = records(&text)?;
        let mut catalogue = super::appinfo::catalogue();
        let games = records.iter().filter_map(|r| catalogue.remove(&r.app_id)).filter(|app| ["game","demo","mod","application"].contains(&app.kind.as_str())).collect();
        return Ok(SteamPlaytime { account_id: account.id.clone(), records, games });
    }
    Err("未找到 Steam 本地时长记录".into())
}
pub fn read_steam_playtime() -> Result<SteamPlaytime, String> { read_steam_playtime_for(None) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_nested_apps_without_mixing_accounts_or_minutes() {
        let text = r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
            "570" { "LastPlayed" "1771310231" "Playtime" "10612" "cloud" { "Playtime" "999" } }
            "730" { "Playtime" "60" }
            "123" { "Playtime" "-1" }
        } } } } }"#;
        let entries = records(text).unwrap();
        assert_eq!(entries.len(),2);
        assert_eq!(entries.iter().find(|r| r.app_id == "570").unwrap().minutes,10612);
        assert_eq!(entries.iter().find(|r| r.app_id == "730").unwrap().last_played,None);
    }
    #[test]
    fn rejects_truncated_config_instead_of_erasing_valid_playtime() {
        assert!(records(r#""UserLocalConfigStore" { "Software" {"#).is_err());
    }
}

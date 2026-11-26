//! Read names/types from Steam's version 39–41 public application cache.
//! This file is a catalogue, not an ownership list.
use serde_json::{Map, Value};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone)]
pub struct AppInfo { pub id: String, pub name: String, pub name_zh: Option<String>, pub kind: String }
struct Reader<'a> { bytes: &'a [u8], at: usize }
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).ok_or("appinfo offset overflow")?;
        let value = self.bytes.get(self.at..end).ok_or("appinfo truncated")?;
        self.at = end; Ok(value)
    }
    fn u32(&mut self) -> Result<u32, String> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn string(&mut self) -> Result<String, String> {
        let n = self.bytes.get(self.at..).ok_or("appinfo string")?.iter().position(|b| *b == 0).ok_or("appinfo unterminated string")?;
        let s = String::from_utf8_lossy(self.take(n)?).into_owned(); self.take(1)?; Ok(s)
    }
    fn object(&mut self, strings: Option<&[String]>, depth: usize) -> Result<Value, String> {
        if depth > 64 { return Err("appinfo nesting".into()); }
        let mut map = Map::new();
        loop {
            let kind = self.take(1)?[0];
            if kind == 8 || kind == 11 { break; }
            let key = match strings { Some(table) => table.get(self.u32()? as usize).ok_or("appinfo string index")?.clone(), None => self.string()? };
            let value = match kind {
                0 => self.object(strings, depth + 1)?,
                1 => Value::String(self.string()?),
                2 | 3 | 4 | 6 => { self.take(4)?; Value::Null },
                7 | 10 => { self.take(8)?; Value::Null },
                _ => return Err(format!("appinfo unsupported value {kind}")),
            };
            map.insert(key, value);
        }
        Ok(Value::Object(map))
    }
}
pub fn parse(bytes: &[u8]) -> Result<Vec<AppInfo>, String> {
    let mut reader = Reader { bytes, at: 0 };
    let magic = reader.u32()?;
    if ![0x07564427, 0x07564428, 0x07564429].contains(&magic) { return Err("未知 Steam appinfo 版本".into()); }
    reader.u32()?;
    let strings = if magic == 0x07564429 {
        let offset = u64::from_le_bytes(reader.take(8)?.try_into().unwrap());
        let mut table = Reader { bytes, at: usize::try_from(offset).map_err(|e| e.to_string())? };
        let count = table.u32()? as usize;
        if count > 1_000_000 { return Err("appinfo string table too large".into()); }
        let mut values = Vec::with_capacity(count);
        for _ in 0..count { values.push(table.string()?); }
        Some(values)
    } else { None };
    let mut apps = Vec::new();
    loop {
        let id = reader.u32()?;
        if id == 0 { break; }
        let size = reader.u32()? as usize;
        let entry = reader.take(size)?;
        let header = if magic >= 0x07564428 { 60 } else { 40 };
        let mut blob = Reader { bytes: entry, at: header };
        // A corrupt entry is isolated; the next app still starts at its declared boundary.
        if let Ok(value) = blob.object(strings.as_deref(), 0) {
            let common = &value["appinfo"]["common"];
            if let Some(name) = common["name"].as_str().filter(|s| !s.trim().is_empty()) {
                let name_zh = ["schinese","tchinese"].into_iter().find_map(|language| common["name_localized"][language].as_str().filter(|s| !s.trim().is_empty()).map(str::to_string));
                apps.push(AppInfo { id: id.to_string(), name: name.into(), name_zh, kind: common["type"].as_str().unwrap_or("").to_lowercase() });
            }
        }
    }
    Ok(apps)
}
pub fn catalogue() -> HashMap<String, AppInfo> {
    let mut apps = HashMap::new();
    for root in super::steam::steam_roots() {
        let path = root.join(Path::new("appcache/appinfo.vdf"));
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(entries) = parse(&bytes) { for app in entries { apps.entry(app.id.clone()).or_insert(app); } }
        }
    }
    apps
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn rejects_truncated_and_unknown_cache() { assert!(parse(&[1,2,3]).is_err()); assert!(parse(&[0;16]).is_err()); }
    #[test] fn reads_modern_string_table() {
        let mut blob = Vec::new();
        for index in [0u32,1] { blob.push(0); blob.extend(index.to_le_bytes()); }
        for (index, value) in [(2u32,"Example"),(3,"Game")] { blob.push(1); blob.extend(index.to_le_bytes()); blob.extend(value.as_bytes()); blob.push(0); }
        blob.extend([8,8,8]);
        let offset = 16 + 8 + 60 + blob.len() + 4;
        let mut bytes = Vec::new(); bytes.extend(0x07564429u32.to_le_bytes()); bytes.extend(1u32.to_le_bytes()); bytes.extend((offset as u64).to_le_bytes());
        bytes.extend(570u32.to_le_bytes()); bytes.extend(((60+blob.len()) as u32).to_le_bytes()); bytes.extend([0;60]); bytes.extend(blob); bytes.extend(0u32.to_le_bytes());
        bytes.extend(4u32.to_le_bytes()); bytes.extend(b"appinfo\0common\0name\0type\0");
        let apps = parse(&bytes).unwrap(); assert_eq!(apps.len(),1); assert_eq!(apps[0].name,"Example"); assert_eq!(apps[0].kind,"game");
    }
}

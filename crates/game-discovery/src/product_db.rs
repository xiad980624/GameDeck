//! Bounded decoder for the install-only subset of Battle.net's protobuf database.
use std::path::PathBuf;
#[derive(Debug)] pub struct Product { pub code: String, pub path: PathBuf }
fn varint(bytes: &[u8], at: &mut usize) -> Result<u64,String> {
    let mut value=0u64;
    for shift in (0..70).step_by(7) {
        let byte=*bytes.get(*at).ok_or("truncated product.db varint")?; *at+=1;
        if shift==63 && byte>1 { return Err("product.db varint overflow".into()); }
        value|=((byte&127) as u64)<<shift;
        if byte&128==0 { return Ok(value); }
    }
    Err("product.db varint overflow".into())
}
fn fields(bytes: &[u8]) -> Result<Vec<(u64,&[u8])>,String> {
    let mut at=0; let mut result=Vec::new();
    while at<bytes.len() {
        let tag=varint(bytes,&mut at)?; if tag>>3==0 { return Err("invalid product.db field".into()); }
        let n=match tag&7 { 0=>{varint(bytes,&mut at)?;continue}, 1=>8, 2=>usize::try_from(varint(bytes,&mut at)?).map_err(|e|e.to_string())?, 5=>4, _=>return Err("unsupported product.db wire type".into()) };
        let end=at.checked_add(n).ok_or("product.db overflow")?;
        let value=bytes.get(at..end).ok_or("truncated product.db field")?;at=end;
        if tag&7==2 { result.push((tag>>3,value)); }
    }
    Ok(result)
}
pub fn parse(bytes:&[u8])->Result<Vec<Product>,String> {
    if bytes.len()>16_000_000 { return Err("product.db too large".into()); }
    let mut products=Vec::new();
    for (_,entry) in fields(bytes)?.into_iter().filter(|(n,_)|*n==1) {
        let entry=fields(entry)?;
        let code=entry.iter().find(|(n,_)|*n==2).and_then(|(_,b)|std::str::from_utf8(b).ok());
        let settings=entry.iter().find(|(n,_)|*n==3).map(|(_,b)|fields(b)).transpose()?;
        let path=settings.as_ref().and_then(|f|f.iter().find(|(n,_)|*n==1)).and_then(|(_,b)|std::str::from_utf8(b).ok());
        if let (Some(code),Some(path))=(code,path) { if !code.is_empty() && !path.is_empty() { products.push(Product {code:code.into(),path:PathBuf::from(path)}); } }
    }
    Ok(products)
}
#[cfg(test)] mod tests {
    use super::*;
    fn field(tag:u8,value:&[u8])->Vec<u8>{let mut b=vec![tag<<3|2,value.len() as u8];b.extend(value);b}
    #[test] fn reads_unicode_path_and_ignores_launcher_fields() {
        let mut entry=field(2,b"d3cn"); entry.extend(field(3,&field(1,"F:/暗黑破坏神III".as_bytes())));
        let db=field(1,&entry);let p=parse(&db).unwrap();assert_eq!(p[0].code,"d3cn");assert_eq!(p[0].path,PathBuf::from("F:/暗黑破坏神III"));
        assert!(parse(&db[..db.len()-1]).is_err());assert!(parse(&[0xff;12]).is_err());
    }
}

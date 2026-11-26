//! Refresh one installed game from its local client manifest, without discovery or artwork work.
use game_storage::Storage;
fn main()->Result<(),String>{tauri::async_runtime::block_on(async{
 let mut args=std::env::args().skip(1);
 let db=args.next().ok_or("database path required")?;let external_id=args.next().ok_or("platform external id required")?;
 let storage=Storage::open(std::path::Path::new(&db)).await?;
 let game=storage.list_games().await?.into_iter().find(|g|g.external_id==external_id).ok_or("game not found")?;
 let target=storage.installation_for_status(&game.id).await?.ok_or("installed target not found")?;
 for status in game_discovery::status::collect(&[target]){
  storage.save_installation_status(&status.provider,&status.external_id,&status.status,status.version.as_deref(),&status.detail).await?;
  println!("{} {} {}",status.external_id,status.status,status.version.unwrap_or_default());
 }
 Ok(())
})}

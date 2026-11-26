//! Platform adapters return the same domain model. Add new providers here without UI changes.
pub mod steam;
pub mod epic;
pub mod battlenet;
pub mod playtime;
pub mod appinfo;
pub mod titles;
mod product_db;
pub mod status;
use game_core::DiscoveredGame;

pub struct ProviderResult {
    pub provider: &'static str,
    pub games: Vec<DiscoveredGame>,
    pub errors: Vec<String>,
}

/// Each provider is isolated: one broken manifest must not abort other scans.
pub fn scan_all() -> Vec<ProviderResult> {
    let adapters: [(&str, fn() -> (Vec<DiscoveredGame>, Vec<String>)); 3] = [
        ("steam", steam::scan),
        ("epic", epic::scan),
        ("battlenet", battlenet::scan),
    ];
    adapters.into_iter().map(|(provider, scan)| {
        let (games, errors) = scan();
        ProviderResult { provider, games, errors }
    }).collect()
}

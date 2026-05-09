//! Legitimate Steam client `UserGameStats_<userid>_<appid>.bin` adapter (Phase 3 stub — Plan 03-00).
//!
//! Plan 03-01 populates the body. This stub provides:
//! 1. The struct shape `SteamLegitPaths` returned from `discover_paths`.
//! 2. The function signature `pub fn discover_paths(steam_install: Option<&Path>) -> SteamLegitPaths`
//!    so Plan 03-00's `paths::discover()` can call it without compile errors.
//! 3. A `SteamLegitAdapter` placeholder struct + `new` constructor returning a stub
//!    that implements `SourceAdapter` returning empty/no-op for every method,
//!    so Plan 03-04's `lib.rs::run()` adapter Vec wiring compiles.
//!
//! All methods log `tracing::warn!("steam_legit stub — Plan 03-01 will populate")` exactly once.

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

/// Paths discovered for the Steam-legit adapter at startup.
#[derive(Debug, Clone, Default)]
pub struct SteamLegitPaths {
    /// `<SteamPath>\appcache\stats` if it exists; None otherwise.
    pub appcache_stats: Option<PathBuf>,
    /// Steam user IDs enumerated from `HKCU\Software\Valve\Steam\Users` registry. Empty if Steam not detected.
    pub user_ids: Vec<u64>,
}

/// Discover Steam-legit paths from an optional Steam install root. Plan 03-01 populates the body.
pub fn discover_paths(_steam_install: Option<&Path>) -> SteamLegitPaths {
    // Plan 03-01: read HKCU\Software\Valve\Steam\Users for user_ids;
    //              join steam_install with appcache\stats and check exists.
    SteamLegitPaths::default()
}

/// Adapter for legitimate Steam client. Plan 03-01 populates the body.
pub struct SteamLegitAdapter {
    cached_watch_paths: Vec<PathBuf>,
    #[allow(dead_code)]
    user_ids: Vec<u64>,
}

impl SteamLegitAdapter {
    /// `appcache_stats` is the single watch root; `user_ids` are the Steam user IDs whose files we accept.
    pub fn new(appcache_stats: Option<PathBuf>, user_ids: Vec<u64>) -> Self {
        let cached: Vec<PathBuf> = appcache_stats.into_iter().filter(|p| p.exists()).collect();
        Self { cached_watch_paths: cached, user_ids }
    }
}

#[async_trait::async_trait]
impl SourceAdapter for SteamLegitAdapter {
    fn name(&self) -> &str { "steam_legit" }
    fn kind(&self) -> SourceKind { SourceKind::SteamLegit }

    fn watch_paths(&self) -> Vec<PathBuf> { self.cached_watch_paths.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        tracing::warn!("steam_legit::seed_baseline stub — Plan 03-01 will populate");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        _path: PathBuf,
        _tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        tracing::trace!("steam_legit::on_file_changed stub — Plan 03-01 will populate");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_paths_returns_default_in_stub() {
        let p = discover_paths(None);
        assert!(p.appcache_stats.is_none());
        assert!(p.user_ids.is_empty());
    }

    #[test]
    fn adapter_kind_is_steam_legit() {
        let a = SteamLegitAdapter::new(None, vec![]);
        assert_eq!(a.name(), "steam_legit");
        assert_eq!(a.kind(), SourceKind::SteamLegit);
        assert!(a.watch_paths().is_empty());
    }
}

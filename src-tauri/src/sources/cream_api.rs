//! CreamAPI `<appid>\stats\CreamAPI.Achievements.cfg` adapter (Phase 3 stub — Plan 03-00).
//!
//! Plan 03-02 populates the body. This stub provides the same surface as
//! `steam_legit.rs`: a paths-data struct, a `discover_paths()` returning empty,
//! and a `CreamApiAdapter` whose SourceAdapter methods are no-ops.

use std::path::PathBuf;
use tokio::sync::mpsc;

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

/// Per-appid CreamAPI directories discovered at startup.
#[derive(Debug, Clone, Default)]
pub struct CreamApiPaths {
    /// `%APPDATA%\CreamAPI\<appid>\` directories that exist on disk.
    pub appid_dirs: Vec<PathBuf>,
}

/// Discover CreamAPI paths. Plan 03-02 populates the body.
pub fn discover_paths() -> CreamApiPaths {
    // Plan 03-02: enumerate %APPDATA%\CreamAPI\* numeric subdirs; for each one
    //              with a stats\CreamAPI.Achievements.cfg, push the appid dir.
    //              Plan 03-02 will also honor `HALLMARK_CREAMAPI_ROOT_OVERRIDE` env
    //              var to redirect the lookup root for SC2 integration testing
    //              (parallels Phase 1's HALLMARK_GOLDBERG_ROOT_OVERRIDE; see B-2 fix).
    CreamApiPaths::default()
}

/// Adapter for CreamAPI emulator. Plan 03-02 populates the body.
pub struct CreamApiAdapter {
    cached_watch_paths: Vec<PathBuf>,
}

impl CreamApiAdapter {
    pub fn new(appid_dirs: Vec<PathBuf>) -> Self {
        let cached: Vec<PathBuf> = appid_dirs.into_iter().filter(|p| p.exists()).collect();
        Self { cached_watch_paths: cached }
    }
}

#[async_trait::async_trait]
impl SourceAdapter for CreamApiAdapter {
    fn name(&self) -> &str { "cream_api" }
    fn kind(&self) -> SourceKind { SourceKind::CreamApi }
    fn watch_paths(&self) -> Vec<PathBuf> { self.cached_watch_paths.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        tracing::warn!("cream_api::seed_baseline stub — Plan 03-02 will populate");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        _path: PathBuf,
        _tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        tracing::trace!("cream_api::on_file_changed stub — Plan 03-02 will populate");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_paths_returns_default_in_stub() {
        let p = discover_paths();
        assert!(p.appid_dirs.is_empty());
    }

    #[test]
    fn adapter_kind_is_cream_api() {
        let a = CreamApiAdapter::new(vec![]);
        assert_eq!(a.name(), "cream_api");
        assert_eq!(a.kind(), SourceKind::CreamApi);
    }
}

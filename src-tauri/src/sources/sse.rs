//! SmartSteamEmu `<appid>\stats.bin` adapter (Phase 3 stub — Plan 03-00).
//!
//! Plan 03-03 populates the body. v1 ships only the `stats.bin` variant per
//! Achievement-Watcher's canonical sse.js parser. The alternate `User\Achievements.ini`
//! variant referenced by Hydra is logged as warn and skipped.

use std::path::PathBuf;
use tokio::sync::mpsc;

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

/// Per-appid SSE directories discovered at startup.
#[derive(Debug, Clone, Default)]
pub struct SsePaths {
    /// `%APPDATA%\SmartSteamEmu\<appid>\` directories with a `stats.bin` file present.
    pub appid_dirs: Vec<PathBuf>,
}

/// Discover SSE paths. Plan 03-03 populates the body.
pub fn discover_paths() -> SsePaths {
    // Plan 03-03: enumerate %APPDATA%\SmartSteamEmu\* numeric subdirs; include those
    //              with stats.bin; for those with only User\Achievements.ini, log warn + skip.
    //              Plan 03-03 will also honor `HALLMARK_SSE_ROOT_OVERRIDE` env var to
    //              redirect the lookup root for SC2 integration testing (parallels Phase 1's
    //              HALLMARK_GOLDBERG_ROOT_OVERRIDE; see B-2 fix).
    SsePaths::default()
}

/// Adapter for SmartSteamEmu emulator. Plan 03-03 populates the body.
pub struct SseAdapter {
    cached_watch_paths: Vec<PathBuf>,
}

impl SseAdapter {
    pub fn new(appid_dirs: Vec<PathBuf>) -> Self {
        let cached: Vec<PathBuf> = appid_dirs.into_iter().filter(|p| p.exists()).collect();
        Self { cached_watch_paths: cached }
    }
}

#[async_trait::async_trait]
impl SourceAdapter for SseAdapter {
    fn name(&self) -> &str { "smartsteamemu" }
    fn kind(&self) -> SourceKind { SourceKind::SmartSteamEmu }
    fn watch_paths(&self) -> Vec<PathBuf> { self.cached_watch_paths.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        tracing::warn!("sse::seed_baseline stub — Plan 03-03 will populate");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        _path: PathBuf,
        _tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        tracing::trace!("sse::on_file_changed stub — Plan 03-03 will populate");
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
    fn adapter_kind_is_smartsteamemu() {
        let a = SseAdapter::new(vec![]);
        assert_eq!(a.name(), "smartsteamemu");
        assert_eq!(a.kind(), SourceKind::SmartSteamEmu);
    }
}

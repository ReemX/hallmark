//! Source adapter trait + event types — the contract every detection source implements.
//!
//! # Phase 1 scope
//!
//! Only Goldberg is implemented (Plan 04 — `goldberg.rs`). Phase 3 adds Steam-legit
//! (binary VDF), CreamAPI, and SmartSteamEmu by adding additional `pub mod`
//! declarations here and new `SourceKind` variants below.
//!
//! # Why no `start()` method on the trait
//!
//! ARCHITECTURE.md's original spec had each adapter own its own watcher. In Phase 1
//! we centralized to a single `notify-debouncer-full` instance in `WatcherCore`
//! (Plan 04) so the 500ms debounce is uniform across all adapters. Adapters now only
//! declare the paths they care about (`watch_paths()`) and react to events on those
//! paths (`on_file_changed()`). The trait is intentionally smaller as a result.
//!
//! # Why `Send + Sync + 'static`
//!
//! Adapters are owned by `Arc<dyn SourceAdapter>` and shared with tokio tasks; all
//! three bounds are required for that. `'static` is satisfied because adapters carry
//! no borrowed data — only owned `Vec<PathBuf>` and `Arc<RwLock<...>>` interiors.

pub mod goldberg;

use std::path::PathBuf;
use tokio::sync::mpsc;

/// A raw unlock event emitted by a source adapter before any cross-source dedup or
/// schema enrichment. Phase 1's pipeline emits these directly to stdout via the CLI
/// test harness (Plan 05). Phase 2 will route them through schema resolution and
/// the popup queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawUnlockEvent {
    /// Steam app ID. Parsed from the `<appid>/achievements.json` directory name.
    pub app_id: u64,
    /// Steam achievement API name (e.g. `ACH_WIN_ONE_GAME`). NOT the human display name.
    pub ach_api_name: String,
    /// Source-reported unix timestamp in seconds. May be `0` when the source did not
    /// record a time (Goldberg's `earned_time = 0` for "earned but timestamp unknown",
    /// per PITFALLS.md #15). DO NOT use `timestamp > 0` as a freshness signal —
    /// the boolean `earned` transition false→true is the only valid unlock signal.
    pub timestamp: u64,
    /// Which adapter produced this event. Used for dedup, logging, and future UI badges.
    pub source: SourceKind,
}

/// Identifies which adapter produced an event. The `as_str()` form is what gets
/// stored in the `unlock_history.source` SQLite column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// Goldberg / gbe_fork emulator. Watches `%APPDATA%\Goldberg SteamEmu Saves\`,
    /// `%APPDATA%\GSE Saves\`, and `local_save.txt` redirects.
    Goldberg,
    // Phase 3 will add: SteamLegit, CreamApi, SmartSteamEmu
    // Future community plug-ins will add: Community(String)
}

impl SourceKind {
    /// Stable string representation for SQLite TEXT storage and log spans.
    /// Stable across versions — schema migrations rely on this string being lossless.
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Goldberg => "goldberg",
        }
    }
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Trait every source adapter implements. Plan 04 provides the only Phase 1
/// implementation (`goldberg::GoldbergAdapter`). The shape is locked here; adding
/// new methods is a breaking change to all adapters.
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync + 'static {
    /// Human-readable adapter name for log spans (`"goldberg"`, etc.).
    fn name(&self) -> &str;

    /// Stable enum identifier — what gets persisted as `unlock_history.source`.
    fn kind(&self) -> SourceKind;

    /// Filesystem roots this adapter watches (recursively). `WatcherCore` registers
    /// each path with the shared notify-debouncer-full instance and dispatches events
    /// back to the matching adapter via prefix-match.
    ///
    /// Adapter MUST filter out non-existent paths before returning — `notify::Watcher::watch`
    /// errors `PathNotFound` for missing dirs (PITFALLS.md, RESEARCH.md Pitfall #5).
    fn watch_paths(&self) -> Vec<PathBuf>;

    /// Read all current state files and populate the adapter's in-memory baseline
    /// (`HashMap<(appid, ach_api_name), bool>`). MUST run BEFORE the watcher attaches —
    /// `WatcherCore` enforces this ordering.
    ///
    /// Implements REQ DETECT-05 (no spam of historic unlocks on first run).
    async fn seed_baseline(&self) -> anyhow::Result<()>;

    /// Called by `WatcherCore` when a debounced file event lands on a path returned
    /// by this adapter's `watch_paths()`. Adapter parses the file, diffs against the
    /// baseline, and emits `RawUnlockEvent`s for any `false → true` transitions.
    ///
    /// Adapter is responsible for the per-file content-hash check (REQ DETECT-06,
    /// content-hash equality layer); the 500ms debounce is handled centrally.
    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_kind_as_str_is_stable_lowercase() {
        assert_eq!(SourceKind::Goldberg.as_str(), "goldberg");
        assert_eq!(SourceKind::Goldberg.to_string(), "goldberg");
    }

    #[test]
    fn raw_unlock_event_eq_ignores_timestamp_only_for_clone() {
        // Two events with same fields are equal (PartialEq derived).
        let a = RawUnlockEvent {
            app_id: 480,
            ach_api_name: "ACH_X".into(),
            timestamp: 0,
            source: SourceKind::Goldberg,
        };
        let b = a.clone();
        assert_eq!(a, b);
        // Differing timestamp DOES matter — derived Eq is field-by-field.
        let c = RawUnlockEvent {
            timestamp: 1,
            ..a.clone()
        };
        assert_ne!(a, c);
    }
}

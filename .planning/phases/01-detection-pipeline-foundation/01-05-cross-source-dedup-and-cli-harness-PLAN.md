---
phase: 01-detection-pipeline-foundation
plan: 05
type: execute
wave: 4
depends_on: [01-01, 01-02, 01-03, 01-04]
files_modified:
  - src-tauri/src/watcher/dedup.rs
  - src-tauri/src/watcher/mod.rs
  - src-tauri/src/store/mod.rs
  - src-tauri/src/paths.rs
  - src-tauri/src/bin/hallmark-cli.rs
  - src-tauri/Cargo.toml
  - src-tauri/tests/integration_phase1.rs
autonomous: true
requirements: [DETECT-01, DETECT-05, DETECT-06, DETECT-07, DETECT-08]
must_haves:
  truths:
    - "`CrossSourceDedup` struct holds `HashMap<(u64, String), Instant>` with TTL (default 10s) and `is_duplicate(app_id, ach_api_name)` returns `true` when the key was inserted within TTL"
    - "`hallmark-cli` binary builds via `cargo run --bin hallmark-cli` and accepts `--override-goldberg-root <PATH>` (env var `HALLMARK_GOLDBERG_ROOT_OVERRIDE` also supported) so integration tests run against fixture dirs without polluting `%APPDATA%`"
    - "`hallmark-cli` orchestrates: paths::discover() → adapters list → spawn run_watcher → consume RawUnlockEvent → cross-source dedup → SqliteStore::record_unlock → println!/tracing line per kept event"
    - "`SqliteStore::with_conn<F, T>(&self, f: F)` exposes the underlying connection to in-crate consumers (CLI binary + queries module) without leaking the Mutex visibility"
    - "Integration test `src-tauri/tests/integration_phase1.rs` exercises ROADMAP Success Criteria #1 (single event in <1s, no duplicates for 5s), #2 (zero historical events on populated startup), #3 (local_save.txt redirect end-to-end pipeline), #4 (cross-source dedup collapses two real adapters to one event), #5 (paths logged at startup)"
    - "SC3 builds a real on-disk fixture (steam_api64.dll + local_save.txt + redirect target + appmanifest_*.acf), invokes paths::scan_local_save_redirects (or its equivalent via paths::discover with HALLMARK_STEAM_LIBRARIES_OVERRIDE), constructs a real GoldbergAdapter from the discovery output, runs the full pipeline, writes achievements.json to the redirect target, and asserts exactly one RawUnlockEvent arrives at the sink with the appid resolved from the appmanifest"
    - "SC4 uses a real test-only MockAdapter implementing SourceAdapter that emits via a file-event-driven path (mirrors GoldbergAdapter's contract) — NOT direct raw_tx injection"
    - "`run_watcher` (Plan 04) is left unchanged in shape; this plan ADDS a wiring function `run_pipeline(adapters, raw_rx, store, session_id, sink, dedup_ttl)` in `watcher/mod.rs` that consumes the RawUnlockEvent stream and applies dedup + persistence"
  artifacts:
    - path: "src-tauri/src/watcher/dedup.rs"
      provides: "CrossSourceDedup TTL-based dedup helper"
      min_lines: 80
      contains: 'pub struct CrossSourceDedup'
    - path: "src-tauri/src/store/mod.rs"
      provides: "SqliteStore::with_conn helper added (existing API otherwise unchanged)"
      contains: 'pub fn with_conn'
    - path: "src-tauri/src/bin/hallmark-cli.rs"
      provides: "Standalone CLI test harness — full pipeline minus Tauri WebView"
      min_lines: 120
      contains: 'fn main'
    - path: "src-tauri/tests/integration_phase1.rs"
      provides: "End-to-end tests covering all 5 ROADMAP Success Criteria, including a real-disk SC3 redirect test and a MockAdapter-driven SC4 dedup test"
      min_lines: 250
      contains: '#[tokio::test]'
  key_links:
    - from: "src-tauri/src/bin/hallmark-cli.rs"
      to: "src-tauri/src/lib.rs (hallmark_lib)"
      via: "use hallmark_lib::{paths, sources, store, watcher}"
      pattern: 'use hallmark_lib::'
    - from: "src-tauri/src/watcher/mod.rs"
      to: "src-tauri/src/watcher/dedup.rs"
      via: "module declaration"
      pattern: 'pub mod dedup'
    - from: "src-tauri/tests/integration_phase1.rs"
      to: "src-tauri/src/lib.rs (hallmark_lib)"
      via: "tests target consumes the lib crate"
      pattern: 'use hallmark_lib::'
    - from: "src-tauri/Cargo.toml"
      to: "src-tauri/src/bin/hallmark-cli.rs"
      via: "[[bin]] hallmark-cli table"
      pattern: 'name = "hallmark-cli"'
---

<objective>
Close the Phase 1 detection pipeline by adding (a) the cross-source dedup stage (REQ DETECT-07), (b) the `hallmark-cli` test harness binary that orchestrates the full pipeline outside the Tauri WebView, and (c) the end-to-end integration tests that exercise all 5 ROADMAP Success Criteria. After this plan, Phase 1's `Goal` ("a reliable, spam-free unlock event stream is flowing end-to-end for Goldberg-emulated games, ready for a UI layer to consume") is provably met.

Purpose: Plans 01–04 left a usable but unwired set of components: Goldberg adapter emits events on a channel, but nothing CONSUMES those events, no dedup is applied, and there's no executable for ROADMAP success-criteria verification. This plan provides:
1. `CrossSourceDedup` (the second deduplication stage layered on top of debounce + content-hash)
2. `SqliteStore::with_conn` helper (so the CLI binary can run typed queries without leaking the connection Mutex)
3. `hallmark-cli` binary that runs the full pipeline and prints unlocks (the deliverable named in ROADMAP Success Criterion #1: "exactly one unlock event in the CLI test harness")
4. Integration tests that automate the 5 success criteria so future regressions are caught — including a real-disk SC3 redirect test that drives `paths::scan_local_save_redirects` end-to-end and a MockAdapter-driven SC4 dedup test that exercises two real `SourceAdapter` implementations.

Output:
- `src-tauri/src/watcher/dedup.rs` (~100 lines, with 4 unit tests)
- `src-tauri/src/store/mod.rs` (existing — extended with one `pub fn with_conn` helper)
- `src-tauri/src/bin/hallmark-cli.rs` (~150 lines)
- `src-tauri/Cargo.toml` updated with `[[bin]] name = "hallmark-cli"` table
- `src-tauri/src/watcher/mod.rs` extended with `pub mod dedup;` and `pub async fn run_pipeline(...)` consumer
- `src-tauri/tests/integration_phase1.rs` with 5 integration tests, one per Success Criterion
</objective>

<execution_context>
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md
@CLAUDE.md

<interfaces>
<!-- Composable APIs from Plans 01–04 (do NOT redefine). -->

From `hallmark_lib::paths` (Plan 03):
```rust
pub struct GoldbergRedirect { pub target_path: PathBuf, pub app_id: u64 }
pub fn discover() -> DiscoveredPaths;
pub fn goldberg_watch_paths(d: &DiscoveredPaths) -> Vec<PathBuf>;
pub fn goldberg_redirect_map(d: &DiscoveredPaths) -> HashMap<PathBuf, u64>;
pub(crate) fn appmanifest_lookup(library: &Path) -> HashMap<String, u64>;  // pub-in-crate; tests reach via direct call
```

From `hallmark_lib::sources` (Plan 02):
```rust
pub trait SourceAdapter: Send + Sync + 'static { /* 5 methods */ }
pub struct RawUnlockEvent { app_id: u64, ach_api_name: String, timestamp: u64, source: SourceKind }
pub enum SourceKind { Goldberg }
```

From `hallmark_lib::sources::goldberg` (Plan 04):
```rust
pub struct GoldbergAdapter;
impl GoldbergAdapter {
    pub fn new(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>) -> Self;
}
```

From `hallmark_lib::store` (Plan 02 + this plan's `with_conn` extension):
```rust
pub struct SqliteStore;
impl SqliteStore {
    pub fn open(path: &Path) -> anyhow::Result<Self>;
    pub fn open_in_memory() -> anyhow::Result<Self>;
    pub fn record_unlock(&self, app_id: u64, ach_api_name: &str, source: &str, session_id: Option<&str>) -> anyhow::Result<bool>;
    pub fn count_unlocks(&self) -> anyhow::Result<i64>;
    pub fn with_conn<F, T>(&self, f: F) -> anyhow::Result<T> where F: FnOnce(&Connection) -> anyhow::Result<T>;
}
pub mod queries {
    pub fn create_session(conn: &Connection, session_id: &str, app_id: Option<u64>) -> anyhow::Result<()>;
    pub fn end_session(conn: &Connection, session_id: &str) -> anyhow::Result<()>;
}
```

From `hallmark_lib::watcher` (Plan 04):
```rust
pub async fn run_watcher(adapters: Vec<Arc<dyn SourceAdapter>>, raw_tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>;
```

NEW APIs this plan adds:
```rust
// In hallmark_lib::watcher::dedup
pub struct CrossSourceDedup {
    seen: HashMap<(u64, String), Instant>,
    ttl: Duration,
}
impl CrossSourceDedup {
    pub fn new(ttl: Duration) -> Self;
    /// Returns true if this event should be dropped as a duplicate.
    pub fn is_duplicate(&mut self, app_id: u64, ach_api_name: &str) -> bool;
}

// In hallmark_lib::watcher
pub async fn run_pipeline(
    adapters: Vec<Arc<dyn SourceAdapter>>,
    raw_rx: mpsc::Receiver<RawUnlockEvent>,
    store: Arc<SqliteStore>,
    session_id: String,
    sink: mpsc::Sender<RawUnlockEvent>,  // forwards kept events for CLI/test consumers
    dedup_ttl: Duration,
) -> anyhow::Result<()>;
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Implement CrossSourceDedup + run_pipeline consumer in watcher module</name>
  <files>
    - src-tauri/src/watcher/dedup.rs
    - src-tauri/src/watcher/mod.rs
  </files>
  <read_first>
    - src-tauri/src/watcher/mod.rs (Plan 04 — DO NOT modify the existing run_watcher; only ADD `pub mod dedup;` and a NEW `pub async fn run_pipeline`)
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md ("Pattern 3: Cross-source dedup as a separate pipeline stage" — provides the verbatim CrossSourceDedup implementation)
    - src-tauri/src/sources/mod.rs (RawUnlockEvent, SourceAdapter)
    - src-tauri/src/store/mod.rs (SqliteStore::record_unlock signature)
  </read_first>
  <behavior>
    Tests for CrossSourceDedup:
    - Test 1 (`first_observation_is_not_duplicate`): Fresh dedup, `is_duplicate(480, "ACH_X")` returns `false`.
    - Test 2 (`repeat_observation_within_ttl_is_duplicate`): After Test 1, a second call within TTL returns `true`.
    - Test 3 (`expired_observation_is_no_longer_duplicate`): After TTL elapses (use 50ms TTL + 100ms sleep), the same key returns `false` again — the sweep evicts expired entries.
    - Test 4 (`different_keys_are_independent`): `(480, "ACH_X")` and `(481, "ACH_X")` and `(480, "ACH_Y")` are all independent keys.

    Tests for run_pipeline (in watcher/mod.rs):
    - Test 5 (`run_pipeline_dedups_simultaneous_cross_source_events`): Two RawUnlockEvents with the same logical key emitted via raw_tx within the dedup TTL window. Pipeline forwards exactly ONE event to the sink and stores ONE row in the store.
  </behavior>
  <action>
    Step 1 — Create `src-tauri/src/watcher/dedup.rs`. Verbatim from RESEARCH.md "Pattern 3" with added doc comments + 4 unit tests:

    ```rust
    //! Cross-source duplicate suppression — REQ DETECT-07.
    //!
    //! When multiple source adapters observe the same logical unlock (e.g. a user runs
    //! a legitimate Steam game with Goldberg also active and watching), each adapter
    //! independently emits a `RawUnlockEvent`. This stage collapses them.
    //!
    //! # Layering
    //!
    //! Phase 1's pipeline has THREE deduplication layers:
    //!
    //! 1. **notify-debouncer-full** — collapses bursts of FS events on the same path
    //!    within 500ms (REQ DETECT-06 layer 1).
    //! 2. **Per-adapter SHA-256 content hash** — collapses identical re-writes within
    //!    one adapter (REQ DETECT-06 layer 2).
    //! 3. **CrossSourceDedup (this module)** — collapses logically identical unlocks
    //!    across DIFFERENT adapters (REQ DETECT-07).
    //!
    //! All three are required: layer 1 doesn't see across files, layer 2 doesn't see
    //! across adapters, layer 3 catches what the others miss.
    //!
    //! # TTL choice (10 seconds default)
    //!
    //! Real-world simultaneity between adapters is sub-second. 10s is a generous safety
    //! margin (RESEARCH.md "Pattern 3"). The SQLite `UNIQUE INDEX` (Plan 02) is the
    //! belt-and-suspenders backstop if a duplicate slips past TTL.

    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    /// In-memory TTL cache for cross-source dedup.
    /// NOT thread-safe by itself — wrap in `tokio::sync::Mutex` for shared access.
    pub struct CrossSourceDedup {
        seen: HashMap<(u64, String), Instant>,
        ttl: Duration,
    }

    impl CrossSourceDedup {
        pub fn new(ttl: Duration) -> Self {
            Self { seen: HashMap::new(), ttl }
        }

        /// Returns `true` if this event should be DROPPED as a duplicate.
        /// Side effect: sweeps expired entries before checking, then inserts the new
        /// observation if not a duplicate.
        pub fn is_duplicate(&mut self, app_id: u64, ach_api_name: &str) -> bool {
            let now = Instant::now();
            // Sweep expired. O(n) but n is bounded by per-session unlock count (small).
            let ttl = self.ttl;
            self.seen.retain(|_, ts| now.duration_since(*ts) < ttl);

            let key = (app_id, ach_api_name.to_string());
            if self.seen.contains_key(&key) {
                true
            } else {
                self.seen.insert(key, now);
                false
            }
        }

        /// Number of currently-tracked entries (for diagnostics).
        pub fn len(&self) -> usize {
            self.seen.len()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn first_observation_is_not_duplicate() {
            let mut d = CrossSourceDedup::new(Duration::from_secs(10));
            assert!(!d.is_duplicate(480, "ACH_X"));
            assert_eq!(d.len(), 1);
        }

        #[test]
        fn repeat_observation_within_ttl_is_duplicate() {
            let mut d = CrossSourceDedup::new(Duration::from_secs(10));
            assert!(!d.is_duplicate(480, "ACH_X"));
            assert!(d.is_duplicate(480, "ACH_X"));
            assert!(d.is_duplicate(480, "ACH_X"));
            assert_eq!(d.len(), 1);
        }

        #[test]
        fn expired_observation_is_no_longer_duplicate() {
            let mut d = CrossSourceDedup::new(Duration::from_millis(50));
            assert!(!d.is_duplicate(480, "ACH_X"));
            std::thread::sleep(Duration::from_millis(100));
            // After TTL the entry is swept; a fresh observation is NOT a duplicate.
            assert!(!d.is_duplicate(480, "ACH_X"));
        }

        #[test]
        fn different_keys_are_independent() {
            let mut d = CrossSourceDedup::new(Duration::from_secs(10));
            assert!(!d.is_duplicate(480, "ACH_X"));
            assert!(!d.is_duplicate(481, "ACH_X"));
            assert!(!d.is_duplicate(480, "ACH_Y"));
            assert!(d.is_duplicate(480, "ACH_X"));
            assert_eq!(d.len(), 3);
        }
    }
    ```

    Step 2 — APPEND to `src-tauri/src/watcher/mod.rs` (do not modify the existing `run_watcher`; add only the new module declaration AT THE TOP and the new `run_pipeline` function and its tests AT THE BOTTOM):

    Add at the very top of `src-tauri/src/watcher/mod.rs` (right after the existing module-level doc comments, before any `use` statements):
    ```rust
    pub mod dedup;
    ```

    Append at the end of the file (after the existing `mod tests`):
    ```rust

    // ----------------- Pipeline consumer (Plan 05) -----------------

    use crate::store::SqliteStore;
    use crate::watcher::dedup::CrossSourceDedup;
    use tokio::sync::Mutex as TokioMutex;

    /// Consumes the `raw_rx` stream from `run_watcher`, applies cross-source dedup
    /// (REQ DETECT-07), persists each KEPT event to the SQLite store, and forwards
    /// kept events to the `sink` for the CLI test harness / Phase 2 popup queue.
    ///
    /// Returns when `raw_rx` is closed (graceful shutdown).
    pub async fn run_pipeline(
        _adapters: Vec<Arc<dyn SourceAdapter>>,  // not directly used here, kept for API symmetry
        mut raw_rx: mpsc::Receiver<RawUnlockEvent>,
        store: Arc<SqliteStore>,
        session_id: String,
        sink: mpsc::Sender<RawUnlockEvent>,
        dedup_ttl: Duration,
    ) -> anyhow::Result<()> {
        let dedup = Arc::new(TokioMutex::new(CrossSourceDedup::new(dedup_ttl)));

        while let Some(evt) = raw_rx.recv().await {
            let is_dup = {
                let mut d = dedup.lock().await;
                d.is_duplicate(evt.app_id, &evt.ach_api_name)
            };
            if is_dup {
                tracing::debug!(
                    app_id = evt.app_id,
                    ach = %evt.ach_api_name,
                    source = %evt.source,
                    "cross-source dedup: dropped duplicate"
                );
                continue;
            }

            // Persist (belt-and-suspenders DB-level dedup via UNIQUE INDEX).
            let inserted = match store.record_unlock(
                evt.app_id,
                &evt.ach_api_name,
                evt.source.as_str(),
                Some(&session_id),
            ) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, "store.record_unlock failed");
                    false
                }
            };
            if !inserted {
                tracing::debug!(
                    app_id = evt.app_id,
                    ach = %evt.ach_api_name,
                    "DB-level dedup: row already existed (UNIQUE INDEX)"
                );
                continue;
            }

            tracing::info!(
                app_id = evt.app_id,
                ach = %evt.ach_api_name,
                source = %evt.source,
                "UNLOCK"
            );
            if sink.send(evt).await.is_err() {
                tracing::debug!("downstream sink closed; pipeline draining");
            }
        }

        tracing::info!("run_pipeline shutting down (raw_rx closed)");
        Ok(())
    }

    #[cfg(test)]
    mod pipeline_tests {
        use super::*;
        use crate::sources::SourceKind;
        use std::time::Duration as StdDuration;

        #[tokio::test]
        async fn run_pipeline_dedups_simultaneous_cross_source_events() {
            let store = Arc::new(SqliteStore::open_in_memory().unwrap());
            let session_id = "test-session-1".to_string();
            let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(8);
            let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(8);
            let store_clone = store.clone();

            let handle = tokio::spawn(run_pipeline(
                vec![],
                raw_rx,
                store_clone,
                session_id.clone(),
                sink_tx,
                StdDuration::from_secs(10),
            ));

            // Two events with the SAME logical key emitted within TTL — the second must dedup.
            for _ in 0..2 {
                raw_tx.send(RawUnlockEvent {
                    app_id: 480,
                    ach_api_name: "ACH_X".into(),
                    timestamp: 0,
                    source: SourceKind::Goldberg,
                }).await.unwrap();
            }

            let evt = tokio::time::timeout(StdDuration::from_millis(200), sink_rx.recv())
                .await.unwrap().expect("first event should pass through");
            assert_eq!(evt.app_id, 480);
            assert_eq!(evt.ach_api_name, "ACH_X");

            // Second event must NOT pass through within 200ms
            let none = tokio::time::timeout(StdDuration::from_millis(200), sink_rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none(),
                "duplicate must be dropped at dedup stage");

            // Store has exactly ONE row
            assert_eq!(store.count_unlocks().unwrap(), 1);

            drop(raw_tx);
            let _ = handle.await;
        }
    }
    ```

    Step 3 — Run tests:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --lib watcher::dedup::tests
    cargo test --manifest-path src-tauri/Cargo.toml --lib watcher::pipeline_tests
    ```
    Both exit 0 — 4 + 1 = 5 new tests pass.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/watcher/dedup.rs)) { exit 1 }; $d = Get-Content src-tauri/src/watcher/dedup.rs -Raw; if ($d -notmatch 'pub struct CrossSourceDedup') { exit 10 }; if ($d -notmatch 'pub fn is_duplicate') { exit 11 }; if ($d -notmatch 'self.seen.retain') { exit 12 }; $w = Get-Content src-tauri/src/watcher/mod.rs -Raw; if ($w -notmatch 'pub mod dedup;') { exit 20 }; if ($w -notmatch 'pub async fn run_pipeline') { exit 21 }; if ($w -notmatch 'CrossSourceDedup::new') { exit 22 }; if ($w -notmatch 'is_duplicate') { exit 23 }; if ($w -notmatch 'store.record_unlock') { exit 24 }; cargo check --manifest-path src-tauri/Cargo.toml --all-targets 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; cargo test --manifest-path src-tauri/Cargo.toml --lib watcher 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 40 }; Write-Host 'dedup + pipeline OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/src/watcher/dedup.rs` exists.
    - Contains `pub struct CrossSourceDedup` with `seen: HashMap<(u64, String), Instant>` and `ttl: Duration`.
    - Contains `pub fn is_duplicate(&mut self, app_id: u64, ach_api_name: &str) -> bool` that performs `seen.retain` sweep, then key-presence check.
    - 4 unit tests pass: `first_observation_is_not_duplicate`, `repeat_observation_within_ttl_is_duplicate`, `expired_observation_is_no_longer_duplicate`, `different_keys_are_independent`.
    - `src-tauri/src/watcher/mod.rs` declares `pub mod dedup;` at module scope (top of file).
    - `src-tauri/src/watcher/mod.rs` contains `pub async fn run_pipeline(...)` consuming `RawUnlockEvent`s from a receiver, applying dedup, persisting via `store.record_unlock`, and forwarding to a sink.
    - The existing `run_watcher` from Plan 04 is UNCHANGED in shape (no signature modification).
    - 1 pipeline test passes: `run_pipeline_dedups_simultaneous_cross_source_events`.
    - `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` exits 0.
    - `cargo test --manifest-path src-tauri/Cargo.toml --lib watcher` runs all watcher tests (Plan 04's 3 + this plan's 5 = 8) and all pass.
    - REQ DETECT-07 is now fully wired: in-memory dedup + SQLite UNIQUE INDEX (from Plan 02) provide two independent layers.
  </acceptance_criteria>
  <done>The cross-source dedup stage is implemented and integration-tested in isolation. The `run_pipeline` consumer is ready for Task 2's CLI harness to wire up.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Add SqliteStore::with_conn helper, then add hallmark-cli binary target wiring the full Phase 1 pipeline</name>
  <files>
    - src-tauri/src/store/mod.rs
    - src-tauri/src/bin/hallmark-cli.rs
    - src-tauri/Cargo.toml
  </files>
  <read_first>
    - src-tauri/Cargo.toml (Plan 01 — currently has only `[[bin]] name = "hallmark"`; we ADD a second `[[bin]]` table)
    - src-tauri/src/store/mod.rs (Plan 02 — confirms `SqliteStore.conn: Mutex<Connection>` is `pub(super)`-visible; we ADD `pub fn with_conn` to expose connection access without further visibility relaxation)
    - src-tauri/src/lib.rs (Plan 01 — confirms `pub fn init_tracing()` is exported)
    - src-tauri/src/watcher/mod.rs (just-extended in Task 1 — confirms `run_pipeline` signature)
    - src-tauri/src/paths.rs (Plan 03 — `discover()`, `goldberg_watch_paths()`, `goldberg_redirect_map()`)
    - src-tauri/src/sources/goldberg.rs (Plan 04 — `GoldbergAdapter::new(roots, redirect_map)`)
  </read_first>
  <action>
    **CRITICAL ORDERING (W-06 fix): Step 1 introduces the `with_conn` helper in `SqliteStore` BEFORE the CLI binary uses it. Do NOT write a CLI binary that touches `store.conn.lock().unwrap()` directly first and then refactor — write the helper first, then write the CLI binary against the new clean API.**

    ## Step 1 — Add `with_conn` helper to `src-tauri/src/store/mod.rs`

    Edit `src-tauri/src/store/mod.rs` and add this method to the `impl SqliteStore` block (alongside `open`, `open_in_memory`, `record_unlock`, `count_unlocks`):

    ```rust
    /// Run a closure against the underlying connection. Used by the CLI binary and
    /// the `queries` submodule to invoke typed query helpers (e.g.
    /// `queries::create_session`) without exposing the connection mutex publicly.
    ///
    /// The mutex is held for the duration of the closure; keep the closure short.
    pub fn with_conn<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Connection) -> anyhow::Result<T>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }
    ```

    Confirm `cargo check` still passes:
    ```powershell
    cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    ```
    Existing `store::tests` and `store::queries::tests` from Plan 02 must still pass:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --lib store
    ```

    ## Step 2 — Append the new bin table to `src-tauri/Cargo.toml`

    Right below the existing `[[bin]] name = "hallmark"` table, add:
    ```toml
    [[bin]]
    name = "hallmark-cli"
    path = "src/bin/hallmark-cli.rs"
    ```

    ## Step 3 — Create `src-tauri/src/bin/hallmark-cli.rs`

    This binary is the deliverable named in ROADMAP Phase 1 Success Criterion #1: the CLI harness that prints unlock events. It bypasses the Tauri WebView entirely (Phase 1 has no UI). Verbatim:

    ```rust
    //! `hallmark-cli` — Phase 1 CLI test harness.
    //!
    //! Wires the detection pipeline end-to-end without launching a Tauri WebView:
    //!
    //!     paths::discover()  →  GoldbergAdapter::new(...)  →  run_watcher
    //!                                                              │
    //!                                                              ▼
    //!                                           run_pipeline (dedup + SQLite store)
    //!                                                              │
    //!                                                              ▼
    //!                                                      stdout println per kept event
    //!
    //! # Usage
    //!
    //! Default — uses real `%APPDATA%` paths discovered by `paths::discover()`:
    //!     cargo run --bin hallmark-cli
    //!
    //! Override — for integration tests / fixtures:
    //!     cargo run --bin hallmark-cli -- --override-goldberg-root C:\path\to\fixture
    //! Or env var (preferred for tests; argv requires the `--` separator):
    //!     HALLMARK_GOLDBERG_ROOT_OVERRIDE=C:\path\to\fixture cargo run --bin hallmark-cli
    //!
    //! Exit: Ctrl-C, or close the input via piping.

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use hallmark_lib::paths;
    use hallmark_lib::sources::{goldberg::GoldbergAdapter, RawUnlockEvent, SourceAdapter};
    use hallmark_lib::store::{queries, SqliteStore};
    use hallmark_lib::watcher::{run_pipeline, run_watcher};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    fn parse_argv_override() -> Option<PathBuf> {
        // Accept either: `--override-goldberg-root <PATH>` argv, or env var.
        if let Ok(env_val) = std::env::var("HALLMARK_GOLDBERG_ROOT_OVERRIDE") {
            if !env_val.is_empty() { return Some(PathBuf::from(env_val)); }
        }
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--override-goldberg-root" {
                return args.next().map(PathBuf::from);
            }
        }
        None
    }

    fn db_path() -> PathBuf {
        // Allow tests to override the DB path too; defaults to in-process tempdir for
        // Phase 1 so the CLI doesn't pollute %APPDATA%\Hallmark\ during unit work.
        if let Ok(p) = std::env::var("HALLMARK_DB_PATH_OVERRIDE") {
            if !p.is_empty() { return PathBuf::from(p); }
        }
        // Default: %APPDATA%\Hallmark\hallmark.db
        if let Some(appdata) = dirs::data_dir() {
            let dir = appdata.join("Hallmark");
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir.join("hallmark.db");
            }
        }
        // Fallback: temp dir
        std::env::temp_dir().join("hallmark.db")
    }

    #[tokio::main]
    async fn main() -> anyhow::Result<()> {
        hallmark_lib::init_tracing();
        tracing::info!("hallmark-cli starting (Phase 1 detection-only harness)");

        // ---- Resolve watch paths + redirect_map ----
        let (goldberg_roots, redirect_map): (Vec<PathBuf>, HashMap<PathBuf, u64>) =
            if let Some(override_path) = parse_argv_override() {
                tracing::info!(path = %override_path.display(),
                    "using --override-goldberg-root (real path discovery skipped)");
                (vec![override_path], HashMap::new())
            } else {
                let discovered = paths::discover();
                let roots = paths::goldberg_watch_paths(&discovered);
                let map = paths::goldberg_redirect_map(&discovered);
                (roots, map)
            };

        if goldberg_roots.is_empty() && redirect_map.is_empty() {
            tracing::warn!("no Goldberg paths discovered; pipeline will be idle");
        }

        // ---- Build adapters ----
        let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(goldberg_roots, redirect_map));
        let adapters = vec![adapter];

        // ---- Open store + create session ----
        let store = Arc::new(SqliteStore::open(&db_path())?);
        let session_id = Uuid::new_v4().to_string();
        store.with_conn(|conn| queries::create_session(conn, &session_id, None))?;
        tracing::info!(session_id = %session_id, "session created");

        // ---- Wire channels: watcher ──[raw_*]→ pipeline ──[sink_*]→ stdout printer ----
        let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
        let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(64);

        let watcher_handle = tokio::spawn(run_watcher(adapters.clone(), raw_tx));
        let pipeline_handle = tokio::spawn(run_pipeline(
            adapters,
            raw_rx,
            store.clone(),
            session_id.clone(),
            sink_tx,
            Duration::from_secs(10),
        ));

        // ---- Stdout printer (the user-visible deliverable for ROADMAP Criterion #1) ----
        let printer_handle = tokio::spawn(async move {
            while let Some(evt) = sink_rx.recv().await {
                println!(
                    "UNLOCK app_id={} ach={} source={}",
                    evt.app_id, evt.ach_api_name, evt.source
                );
            }
        });

        // ---- Shutdown signal ----
        // Block on Ctrl-C; on signal, propagate to subtasks via channel-drop.
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Ctrl-C received; shutting down");

        // End the session in the DB.
        let _ = store.with_conn(|conn| queries::end_session(conn, &session_id));

        // Aborting watcher closes raw_tx → run_pipeline's recv() returns None → it exits → sink_tx drops → printer exits.
        watcher_handle.abort();
        let _ = pipeline_handle.await;
        let _ = printer_handle.await;

        tracing::info!("hallmark-cli stopped cleanly");
        Ok(())
    }
    ```

    ## Step 4 — Build the binary

    ```powershell
    cargo build --manifest-path src-tauri/Cargo.toml --bin hallmark-cli
    ```
    Must exit 0.

    ## Step 5 — Smoke test

    Run the binary briefly to confirm it starts and emits the expected startup log lines, then kill it:
    ```powershell
    $job = Start-Job -ScriptBlock {
        $env:HALLMARK_GOLDBERG_ROOT_OVERRIDE = $env:TEMP + "\hallmark-cli-smoke"
        New-Item -ItemType Directory -Force -Path $env:HALLMARK_GOLDBERG_ROOT_OVERRIDE | Out-Null
        cargo run --manifest-path src-tauri/Cargo.toml --quiet --bin hallmark-cli 2>&1
    }
    Start-Sleep -Seconds 8
    Stop-Job $job
    Receive-Job $job | Select-String "hallmark-cli starting|session created|WatcherCore active"
    Remove-Job $job -Force
    ```
    Output must contain at least the substrings `hallmark-cli starting`, `session created`, AND `WatcherCore active` (or `Baseline seeded` — depends on log ordering). If absent: the wiring is broken; debug before declaring done.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/bin/hallmark-cli.rs)) { exit 1 }; $c = Get-Content src-tauri/Cargo.toml -Raw; if ($c -notmatch 'name = .hallmark-cli.') { exit 2 }; if ($c -notmatch 'path = .src/bin/hallmark-cli.rs.') { exit 3 }; $b = Get-Content src-tauri/src/bin/hallmark-cli.rs -Raw; if ($b -notmatch 'use hallmark_lib::') { exit 10 }; if ($b -notmatch 'GoldbergAdapter::new') { exit 11 }; if ($b -notmatch 'paths::discover') { exit 12 }; if ($b -notmatch 'paths::goldberg_redirect_map' -and $b -notmatch 'goldberg_redirect_map') { exit 13 }; if ($b -notmatch 'run_watcher') { exit 14 }; if ($b -notmatch 'run_pipeline') { exit 15 }; if ($b -notmatch '#\[tokio::main\]') { exit 16 }; if ($b -notmatch 'HALLMARK_GOLDBERG_ROOT_OVERRIDE') { exit 17 }; if ($b -notmatch '--override-goldberg-root') { exit 18 }; if ($b -notmatch 'println!') { exit 19 }; if ($b -notmatch 'tokio::signal::ctrl_c') { exit 20 }; if ($b -notmatch 'store.with_conn') { exit 21 }; if ($b -match 'store\.conn\.lock\(\)') { exit 22 }; $s = Get-Content src-tauri/src/store/mod.rs -Raw; if ($s -notmatch 'pub fn with_conn') { exit 25 }; cargo build --manifest-path src-tauri/Cargo.toml --bin hallmark-cli 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; Write-Host 'hallmark-cli built OK'</automated>
  </verify>
  <acceptance_criteria>
    - `src-tauri/src/store/mod.rs` adds `pub fn with_conn<F, T>(&self, f: F) -> anyhow::Result<T> where F: FnOnce(&Connection) -> anyhow::Result<T>` — added FIRST, before any consumer code is written.
    - File `src-tauri/src/bin/hallmark-cli.rs` exists.
    - The CLI binary uses `store.with_conn(|conn| ...)` for queries — it does NOT contain `store.conn.lock().unwrap()` (W-06 fix: the broken interim version is never written; the correct version is the only version).
    - `src-tauri/Cargo.toml` declares `[[bin]] name = "hallmark-cli"` with `path = "src/bin/hallmark-cli.rs"`.
    - The binary uses `hallmark_lib::*` imports (lib crate) — no copy-pasted code from the lib.
    - Function `parse_argv_override()` checks `HALLMARK_GOLDBERG_ROOT_OVERRIDE` env var AND argv `--override-goldberg-root <PATH>` flag.
    - Function `db_path()` checks `HALLMARK_DB_PATH_OVERRIDE` env var (so integration tests can route the DB to a tempdir).
    - Calls `paths::goldberg_redirect_map(&discovered)` and passes the result to `GoldbergAdapter::new(roots, redirect_map)`.
    - Spawns three tokio tasks: `run_watcher`, `run_pipeline`, and the printer; wires them through two `mpsc::channel` instances.
    - Prints `UNLOCK app_id=<n> ach=<name> source=<source>` to stdout via `println!` for each kept event.
    - Listens for `tokio::signal::ctrl_c()` for graceful shutdown.
    - `cargo build --manifest-path src-tauri/Cargo.toml --bin hallmark-cli` exits 0.
    - The smoke-test job output contains at least `hallmark-cli starting` AND `session created`.
  </acceptance_criteria>
  <done>The CLI test harness compiles and runs end-to-end against override paths, using the clean `with_conn` helper API throughout. `cargo run --bin hallmark-cli -- --override-goldberg-root <fixture>` is the command pattern Task 3's integration tests use.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Write src-tauri/tests/integration_phase1.rs covering all 5 ROADMAP Success Criteria — including a real-disk SC3 redirect pipeline test and a MockAdapter-driven SC4 dedup test</name>
  <files>
    - src-tauri/tests/integration_phase1.rs
  </files>
  <read_first>
    - .planning/ROADMAP.md (Phase 1 Success Criteria 1–5 — verbatim test definitions)
    - src-tauri/src/watcher/mod.rs (run_watcher + run_pipeline signatures)
    - src-tauri/src/sources/goldberg.rs (GoldbergAdapter::new(roots, redirect_map) constructor)
    - src-tauri/src/sources/mod.rs (SourceAdapter trait — MockAdapter must implement it)
    - src-tauri/src/store/mod.rs (SqliteStore::open_in_memory, count_unlocks, with_conn)
    - src-tauri/src/paths.rs (DiscoveredPaths, GoldbergRedirect, goldberg_watch_paths, goldberg_redirect_map; pub(crate) scan_local_save_redirects + appmanifest_lookup are accessible to integration tests via re-export — see Note below)
    - tests/fixtures/goldberg/480/achievements.json (Plan 01 fixture)
    - Cargo.toml (workspace root)

    **Note on path-discovery internals access:** Integration tests at `src-tauri/tests/` are external to the crate, so `pub(crate)` items in `paths` (such as `appmanifest_lookup`) are NOT directly accessible. The fix: SC3 drives discovery through the PUBLIC entry points only — it constructs a Steam-library-shaped tempdir (with `appmanifest_*.acf` + `local_save.txt` fixtures), then exercises the redirect resolution by calling a NEW public test-only entry point on the `paths` module. Step-by-step in the action below.
  </read_first>
  <behavior>
    Five integration tests, one per ROADMAP Success Criterion. Each test runs against tempdir fixtures (no real %APPDATA%), spawns watcher + pipeline tokio tasks, performs disk operations, asserts via the sink channel + SQLite store.

    - **SC1**: Drop a populated `achievements.json` for appid 480; mark `ACH_X` as earned. Receive exactly ONE event in <1500ms AND no further events for 2 seconds.
    - **SC2**: Pre-populate `<tmp>/480/achievements.json` with 50 already-earned achievements. Spawn pipeline. Wait 1.5 seconds. Channel receives ZERO events.
    - **SC3 (real-disk pipeline test)**: Build a Steam-library-shaped tempdir on disk: `<lib>/steamapps/common/FooGame/bin/steam_api64.dll`, `<lib>/steamapps/common/FooGame/bin/local_save.txt` pointing at `<redirect_target>`, `<lib>/steamapps/appmanifest_4242.acf` with `installdir = "FooGame"`. Invoke `paths::scan_local_save_redirects_pub_for_tests(&[lib])` (a public test helper exposed by Plan 03). Construct a real `GoldbergAdapter::new(vec![], redirect_map)` from the discovery output. Spawn the full pipeline. Write `achievements.json` to `<redirect_target>` with one earned achievement transition. Assert exactly ONE `RawUnlockEvent` arrives at the sink with `app_id = 4242` (the appid resolved from the appmanifest, NOT from the directory name).
    - **SC4 (MockAdapter-driven dedup)**: Define a test-only `MockAdapter` that emits `RawUnlockEvent` from a watched file under its own root. Build TWO MockAdapters watching DIFFERENT roots, each writing the same logical unlock when its own watch file changes. Run the full pipeline. Trigger both adapters' file events near-simultaneously. Sink receives exactly ONE event; SQLite has exactly ONE row.
    - **SC5**: Use a `tracing-subscriber` test layer to capture events; call `paths::log_discovery_pub_for_tests(&fixture_paths)` (a public test helper exposed by Plan 03 — same pattern as SC3) and assert at least one info event per discovery category appears in the captured log buffer.
  </behavior>
  <action>
    **Pre-step — Plan 03 must expose two thin test-only public wrappers.** Edit `src-tauri/src/paths.rs` (this is a small additive change, not a redesign) to add at the end of the file (after all `mod tests_*` blocks):

    ```rust
    // ---- Public test helpers (used by integration tests in src-tauri/tests/) ----
    //
    // These are thin shims around `pub(crate)` internals so external integration tests
    // can drive the discovery pipeline against fixture directories without needing
    // access to private items.

    /// Public test entry: invoke `scan_local_save_redirects` against the given
    /// libraries. Returns the resolved `Vec<GoldbergRedirect>`. Intended for
    /// integration-test use only; production code should call `discover()`.
    pub fn scan_local_save_redirects_pub_for_tests(libraries: &[PathBuf]) -> Vec<GoldbergRedirect> {
        scan_local_save_redirects(libraries)
    }

    /// Public test entry: invoke `log_discovery` against a synthesized
    /// `DiscoveredPaths`. Returns nothing (logs only). Used by SC5 to drive
    /// the same logging path that `discover()` uses.
    pub fn log_discovery_pub_for_tests(d: &DiscoveredPaths) {
        log_discovery(d);
    }
    ```

    Confirm `cargo check`:
    ```powershell
    cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    ```

    **Now create `src-tauri/tests/integration_phase1.rs`. Critical formatting (W-10 fix):** The `extern crate hallmark_lib as hallmark;` line at the top of an external test crate is the OLD Rust 2015 style. Rust 2018+ no longer needs it; we just `use hallmark_lib::*` directly. **DO NOT add any `extern crate` line — it is unnecessary and confusing.** All imports use the canonical `use hallmark_lib::...` form.

    Verbatim file content:

    ```rust
    //! Phase 1 end-to-end integration tests covering ROADMAP Success Criteria #1–#5.
    //!
    //! Each test runs the full pipeline (`run_watcher` + `run_pipeline`) against tempdir
    //! fixtures. No `%APPDATA%` writes; no `cargo run --bin hallmark-cli` subprocesses
    //! (we exercise the same library entry points the binary calls, but inline so we
    //! can assert against the channels directly).
    //!
    //! Reference: .planning/ROADMAP.md → "Phase 1 Detection Pipeline Foundation → Success Criteria"

    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use hallmark_lib::paths::{self, DiscoveredPaths, GoldbergRedirect};
    use hallmark_lib::sources::goldberg::GoldbergAdapter;
    use hallmark_lib::sources::{RawUnlockEvent, SourceAdapter, SourceKind};
    use hallmark_lib::store::SqliteStore;
    use hallmark_lib::watcher::{run_pipeline, run_watcher};
    use tokio::sync::mpsc;
    use tokio::time::timeout;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context as LayerContext, SubscriberExt};
    use tracing_subscriber::Layer;

    // ============================================================================
    // Common test helpers
    // ============================================================================

    fn fresh_tmp(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("hallmark-int-{}-{}", label, uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_state(root: &Path, app_id: u64, content: &str) -> PathBuf {
        let dir = root.join(app_id.to_string());
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("achievements.json");
        fs::write(&path, content).unwrap();
        path
    }

    fn write_appmanifest(library: &Path, app_id: u64, installdir: &str) {
        let steamapps = library.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        let content = format!(
            "\"AppState\"\n{{\n  \"appid\"      \"{}\"\n  \"name\"       \"Test\"\n  \"installdir\" \"{}\"\n}}\n",
            app_id, installdir
        );
        fs::write(steamapps.join(format!("appmanifest_{}.acf", app_id)), content).unwrap();
    }

    /// Build the full Phase 1 pipeline (run_watcher + run_pipeline) returning the
    /// sink receiver + the store handle for assertions.
    async fn spawn_pipeline(
        adapters: Vec<Arc<dyn SourceAdapter>>,
        store: Arc<SqliteStore>,
    ) -> (
        mpsc::Receiver<RawUnlockEvent>,
        tokio::task::JoinHandle<anyhow::Result<()>>,
        tokio::task::JoinHandle<anyhow::Result<()>>,
    ) {
        let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
        let (sink_tx, sink_rx) = mpsc::channel::<RawUnlockEvent>(64);
        let watcher_handle = tokio::spawn(run_watcher(adapters.clone(), raw_tx));
        let pipeline_handle = tokio::spawn(run_pipeline(
            adapters,
            raw_rx,
            store,
            "test-session".to_string(),
            sink_tx,
            Duration::from_secs(10),
        ));
        // Allow seeding + watcher attach.
        tokio::time::sleep(Duration::from_millis(400)).await;
        (sink_rx, watcher_handle, pipeline_handle)
    }

    // ============================================================================
    // SC1: single-event detection within 1.5s
    // ============================================================================

    #[tokio::test]
    async fn sc1_single_unlock_emits_exactly_one_event_within_one_second() {
        let root = fresh_tmp("sc1");
        let baseline = r#"{
            "ACH_X": {"earned": false, "earned_time": 0},
            "ACH_Y": {"earned": false, "earned_time": 0}
        }"#;
        let path = write_state(&root, 480, baseline);

        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));
        let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![adapter], store.clone()).await;

        // Mark ACH_X earned
        fs::write(&path, r#"{
            "ACH_X": {"earned": true, "earned_time": 1700000999},
            "ACH_Y": {"earned": false, "earned_time": 0}
        }"#).unwrap();

        let evt = timeout(Duration::from_millis(1500), sink_rx.recv())
            .await.expect("event should arrive within 1.5s")
            .expect("Some(event)");
        assert_eq!(evt.app_id, 480);
        assert_eq!(evt.ach_api_name, "ACH_X");

        // No duplicates for the next 2 seconds.
        let none = timeout(Duration::from_secs(2), sink_rx.recv()).await;
        assert!(none.is_err() || none.unwrap().is_none(),
            "no duplicate events within 2s window (Success Criterion #1)");

        assert_eq!(store.count_unlocks().unwrap(), 1);

        watch.abort(); pipe.abort();
        let _ = fs::remove_dir_all(&root);
    }

    // ============================================================================
    // SC2: pre-populated state emits zero events
    // ============================================================================

    #[tokio::test]
    async fn sc2_pre_populated_state_emits_zero_events() {
        let root = fresh_tmp("sc2");
        let mut entries = Vec::with_capacity(50);
        for i in 0..50 {
            entries.push(format!(
                r#""ACH_{:03}":{{"earned":true,"earned_time":{}}}"#,
                i, 1_700_000_000 + i
            ));
        }
        let baseline = format!("{{ {} }}", entries.join(","));
        write_state(&root, 480, &baseline);

        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));
        let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![adapter], store.clone()).await;

        // Wait 1500ms — well past the debounce window. No events should arrive.
        let none = timeout(Duration::from_millis(1500), sink_rx.recv()).await;
        assert!(none.is_err() || none.unwrap().is_none(),
            "zero historical unlock events (Success Criterion #2)");

        assert_eq!(store.count_unlocks().unwrap(), 0);

        watch.abort(); pipe.abort();
        let _ = fs::remove_dir_all(&root);
    }

    // ============================================================================
    // SC3: real-disk local_save.txt redirect pipeline (B-01 fix)
    //
    // Build a complete Steam-library-shaped fixture, run real path discovery,
    // construct a real GoldbergAdapter from the discovered redirect_map, run the
    // full pipeline, and write achievements.json to the resolved redirect target.
    // Assert exactly one event arrives with the appid resolved from appmanifest.
    // ============================================================================

    #[tokio::test]
    async fn sc3_local_save_txt_redirect_drives_end_to_end_pipeline() {
        // ---- Build Steam-library-shaped fixture on disk ----
        let lib = fresh_tmp("sc3-lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("FooGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();

        // appmanifest mapping installdir "FooGame" → appid 4242
        write_appmanifest(&lib, 4242, "FooGame");

        // local_save.txt redirect target
        let redirect_target = fresh_tmp("sc3-redirect");
        let target_str = redirect_target.to_string_lossy().replace('/', "\\");
        fs::write(game_bin.join("local_save.txt"), &target_str).unwrap();

        // ---- Drive real path discovery ----
        let redirects: Vec<GoldbergRedirect> =
            paths::scan_local_save_redirects_pub_for_tests(&[lib.clone()]);
        assert_eq!(redirects.len(), 1, "expected one redirect from fixture; got {:?}", redirects);
        assert_eq!(redirects[0].target_path, redirect_target);
        assert_eq!(redirects[0].app_id, 4242);

        // Build the redirect_map the same way the CLI binary does.
        let discovered = DiscoveredPaths {
            steam_install: Some(lib.clone()),
            steam_libraries: vec![lib.clone()],
            goldberg_save_roots: vec![],
            goldberg_local_save_redirects: redirects.clone(),
        };
        let redirect_map = paths::goldberg_redirect_map(&discovered);
        assert_eq!(redirect_map.get(&redirect_target).copied(), Some(4242),
            "redirect_map should pair target_path → appid");

        // Seed the redirect target with a baseline state file BEFORE spawning the pipeline,
        // so the adapter sees `ACH_X: false` at seed time and the later `false→true` flip
        // is the only transition.
        let ach_path = redirect_target.join("achievements.json");
        fs::write(&ach_path, r#"{"ACH_X":{"earned":false,"earned_time":0}}"#).unwrap();

        // ---- Build adapter + run pipeline ----
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(vec![], redirect_map));
        let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![adapter], store.clone()).await;

        // ---- Trigger the unlock by mutating achievements.json under the redirect target ----
        fs::write(&ach_path, r#"{"ACH_X":{"earned":true,"earned_time":1700001234}}"#).unwrap();

        let evt = timeout(Duration::from_millis(1500), sink_rx.recv())
            .await.expect("event should arrive within 1.5s")
            .expect("Some(event)");
        assert_eq!(evt.app_id, 4242,
            "appid should be resolved from appmanifest (4242), NOT the redirect target's parent dir name");
        assert_eq!(evt.ach_api_name, "ACH_X");
        assert_eq!(evt.source, SourceKind::Goldberg);

        let none = timeout(Duration::from_millis(800), sink_rx.recv()).await;
        assert!(none.is_err() || none.unwrap().is_none(),
            "no duplicate events");
        assert_eq!(store.count_unlocks().unwrap(), 1);

        watch.abort(); pipe.abort();
        let _ = fs::remove_dir_all(&lib);
        let _ = fs::remove_dir_all(&redirect_target);
    }

    // ============================================================================
    // SC4: cross-source dedup using two real MockAdapter instances (W-08 fix)
    //
    // Each MockAdapter watches its own tempdir root and emits `RawUnlockEvent` when
    // its `<root>/trigger.json` file's `earned` value flips from false to true.
    // We trigger BOTH adapters' files near-simultaneously; the dedup stage drops
    // the second event.
    // ============================================================================

    /// Test-only adapter that mirrors GoldbergAdapter's contract minimally:
    /// watches a single root, on `trigger.json` change reads "{earned: bool}" and
    /// emits a single `RawUnlockEvent` for `(fixed_app_id, fixed_ach)` on `false→true`.
    struct MockAdapter {
        root: PathBuf,
        fixed_app_id: u64,
        fixed_ach: String,
        baseline: Arc<tokio::sync::RwLock<Option<bool>>>,
    }

    impl MockAdapter {
        fn new(root: PathBuf, fixed_app_id: u64, fixed_ach: &str) -> Self {
            Self {
                root,
                fixed_app_id,
                fixed_ach: fixed_ach.to_string(),
                baseline: Arc::new(tokio::sync::RwLock::new(None)),
            }
        }
    }

    #[async_trait::async_trait]
    impl SourceAdapter for MockAdapter {
        fn name(&self) -> &str { "mock" }
        fn kind(&self) -> SourceKind { SourceKind::Goldberg }
        fn watch_paths(&self) -> Vec<PathBuf> {
            if self.root.exists() { vec![self.root.clone()] } else { vec![] }
        }
        async fn seed_baseline(&self) -> anyhow::Result<()> {
            let trigger = self.root.join("trigger.json");
            let val = if trigger.exists() {
                let s = fs::read_to_string(&trigger).unwrap_or_default();
                serde_json::from_str::<serde_json::Value>(&s)
                    .ok()
                    .and_then(|v| v.get("earned").and_then(|e| e.as_bool()))
                    .unwrap_or(false)
            } else { false };
            *self.baseline.write().await = Some(val);
            Ok(())
        }
        async fn on_file_changed(
            &self,
            path: PathBuf,
            tx: mpsc::Sender<RawUnlockEvent>,
        ) -> anyhow::Result<()> {
            if path.file_name().and_then(|n| n.to_str()) != Some("trigger.json") {
                return Ok(());
            }
            let s = match fs::read_to_string(&path) { Ok(s) => s, Err(_) => return Ok(()) };
            let earned_now = serde_json::from_str::<serde_json::Value>(&s)
                .ok()
                .and_then(|v| v.get("earned").and_then(|e| e.as_bool()))
                .unwrap_or(false);
            let was = self.baseline.read().await.unwrap_or(false);
            if !was && earned_now {
                let _ = tx.send(RawUnlockEvent {
                    app_id: self.fixed_app_id,
                    ach_api_name: self.fixed_ach.clone(),
                    timestamp: 0,
                    source: SourceKind::Goldberg,
                }).await;
            }
            *self.baseline.write().await = Some(earned_now);
            Ok(())
        }
    }

    #[tokio::test]
    async fn sc4_cross_source_dedup_collapses_real_adapter_events_to_one() {
        let root_a = fresh_tmp("sc4-a");
        let root_b = fresh_tmp("sc4-b");
        // Both adapters start with `earned: false` so the seed picks up false; the later
        // write of `earned: true` is a true transition for each.
        fs::write(root_a.join("trigger.json"), r#"{"earned":false}"#).unwrap();
        fs::write(root_b.join("trigger.json"), r#"{"earned":false}"#).unwrap();

        let mock_a: Arc<dyn SourceAdapter> = Arc::new(MockAdapter::new(root_a.clone(), 4242, "ACH_DUP"));
        let mock_b: Arc<dyn SourceAdapter> = Arc::new(MockAdapter::new(root_b.clone(), 4242, "ACH_DUP"));

        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![mock_a, mock_b], store.clone()).await;

        // Flip both files near-simultaneously
        fs::write(root_a.join("trigger.json"), r#"{"earned":true}"#).unwrap();
        fs::write(root_b.join("trigger.json"), r#"{"earned":true}"#).unwrap();

        let first = timeout(Duration::from_millis(2000), sink_rx.recv())
            .await.expect("first event").expect("Some");
        assert_eq!(first.app_id, 4242);
        assert_eq!(first.ach_api_name, "ACH_DUP");

        // Second event must be dropped within the dedup TTL window.
        let none = timeout(Duration::from_millis(800), sink_rx.recv()).await;
        assert!(none.is_err() || none.unwrap().is_none(),
            "second event must be dropped at dedup stage (Success Criterion #4)");

        assert_eq!(store.count_unlocks().unwrap(), 1,
            "exactly one row persisted (Success Criterion #4)");

        watch.abort(); pipe.abort();
        let _ = fs::remove_dir_all(&root_a);
        let _ = fs::remove_dir_all(&root_b);
    }

    // ============================================================================
    // SC5: tracing capture proves every discovery category emits an info event
    // ============================================================================

    struct VecLayer {
        events: Arc<Mutex<Vec<String>>>,
    }
    impl<S: Subscriber> Layer<S> for VecLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
            use tracing::field::{Field, Visit};
            struct V(String);
            impl Visit for V {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    self.0.push_str(&format!(" {}={:?}", f.name(), v));
                }
                fn record_str(&mut self, f: &Field, v: &str) {
                    self.0.push_str(&format!(" {}={}", f.name(), v));
                }
            }
            let mut v = V(String::new());
            event.record(&mut v);
            self.events.lock().unwrap().push(format!("{} :: {}", event.metadata().level(), v.0));
        }
    }

    #[test]
    fn sc5_path_discovery_logs_every_category_to_tracing() {
        let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let layer = VecLayer { events: events.clone() };
        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        let d = DiscoveredPaths {
            steam_install: Some(PathBuf::from(r"C:\FakeSteam")),
            steam_libraries: vec![PathBuf::from(r"C:\FakeSteam"), PathBuf::from(r"D:\FakeLibrary")],
            goldberg_save_roots: vec![PathBuf::from(r"C:\Goldberg")],
            goldberg_local_save_redirects: vec![
                GoldbergRedirect { target_path: PathBuf::from(r"D:\Redirect"), app_id: 4242 },
            ],
        };
        paths::log_discovery_pub_for_tests(&d);

        let captured = events.lock().unwrap().clone();
        assert!(captured.iter().any(|e| e.contains("Steam install")),
            "expected 'Steam install' info event; got: {:?}", captured);
        assert!(captured.iter().any(|e| e.contains("Steam library")),
            "expected 'Steam library' info event; got: {:?}", captured);
        assert!(captured.iter().any(|e| e.contains("Goldberg save root")),
            "expected 'Goldberg save root' info event; got: {:?}", captured);
        assert!(captured.iter().any(|e| e.contains("local_save.txt redirect")),
            "expected 'local_save.txt redirect' info event; got: {:?}", captured);
        let info_count = captured.iter().filter(|e| e.starts_with("INFO")).count();
        assert!(info_count >= 4,
            "expected at least 4 INFO-level events; got: {:?}", captured);
    }
    ```

    Run:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1 -- --nocapture
    ```
    All 5 tests pass.

    NOTE on the test target name: Cargo registers a test target named after the file (without `.rs`). So `--test integration_phase1` selects this file specifically.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/tests/integration_phase1.rs)) { exit 1 }; $t = Get-Content src-tauri/tests/integration_phase1.rs -Raw; if ($t -notmatch 'sc1_single_unlock_emits_exactly_one_event_within_one_second') { exit 10 }; if ($t -notmatch 'sc2_pre_populated_state_emits_zero_events') { exit 11 }; if ($t -notmatch 'sc3_local_save_txt_redirect_drives_end_to_end_pipeline') { exit 12 }; if ($t -notmatch 'sc4_cross_source_dedup_collapses_real_adapter_events_to_one') { exit 13 }; if ($t -notmatch 'sc5_path_discovery_logs_every_category_to_tracing') { exit 14 }; if ($t -notmatch 'use hallmark_lib::') { exit 15 }; if ($t -match 'extern crate hallmark') { exit 16 }; if ($t -notmatch 'struct MockAdapter') { exit 17 }; if ($t -notmatch 'impl SourceAdapter for MockAdapter') { exit 18 }; if ($t -notmatch 'scan_local_save_redirects_pub_for_tests') { exit 19 }; if ($t -notmatch 'log_discovery_pub_for_tests') { exit 20 }; if ($t -notmatch 'write_appmanifest') { exit 21 }; if ($t -notmatch '#\[tokio::test\]') { exit 22 }; $p = Get-Content src-tauri/src/paths.rs -Raw; if ($p -notmatch 'pub fn scan_local_save_redirects_pub_for_tests') { exit 25 }; if ($p -notmatch 'pub fn log_discovery_pub_for_tests') { exit 26 }; cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; Write-Host 'integration_phase1 OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/tests/integration_phase1.rs` exists.
    - File contains NO `extern crate hallmark` line (W-10 fix; we use Rust 2018 `use hallmark_lib::...` style only).
    - `src-tauri/src/paths.rs` exposes `pub fn scan_local_save_redirects_pub_for_tests(libraries: &[PathBuf]) -> Vec<GoldbergRedirect>` AND `pub fn log_discovery_pub_for_tests(d: &DiscoveredPaths)` so external tests can drive the discovery internals.
    - Five test functions exist with the EXACT names: `sc1_single_unlock_emits_exactly_one_event_within_one_second`, `sc2_pre_populated_state_emits_zero_events`, `sc3_local_save_txt_redirect_drives_end_to_end_pipeline`, `sc4_cross_source_dedup_collapses_real_adapter_events_to_one`, `sc5_path_discovery_logs_every_category_to_tracing`.
    - SC1 asserts exactly ONE event arrives within 1500ms AND no further events for 2s.
    - SC2 asserts ZERO events from a populated state file.
    - **SC3 (B-01 fix):** Builds a real on-disk Steam-library fixture (`steam_api64.dll` + `local_save.txt` + `appmanifest_<appid>.acf`); invokes `paths::scan_local_save_redirects_pub_for_tests`; constructs a real `GoldbergAdapter` from the discovered `redirect_map`; runs the full `run_watcher` + `run_pipeline` pipeline; writes `achievements.json` to the resolved redirect target; asserts exactly one event arrives with `app_id` resolved from the appmanifest (NOT from the directory name).
    - **SC4 (W-08 fix):** Defines a real test-only `MockAdapter` implementing `SourceAdapter` that emits via a file-event-driven path. Two MockAdapter instances watching different roots, both file-triggered, produce exactly one kept event after dedup; SQLite has 1 row.
    - SC5 uses a `tracing-subscriber` layer to capture events from `paths::log_discovery_pub_for_tests` and asserts each category produces an info-level entry.
    - `cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1` exits 0; all 5 tests pass.
    - Combined with prior plans, total Phase 1 test count: ~10 (sources/store) + 16 (paths) + 11 (goldberg) + 3 (watcher) + 5 (Plan 05 watcher) + 5 (integration) = ~50 tests covering the entire phase.
  </acceptance_criteria>
  <done>All 5 ROADMAP Phase 1 Success Criteria are mapped to automated tests. SC3 now exercises the FULL pipeline against a real on-disk redirect fixture (B-01 fix). SC4 now uses two real `SourceAdapter` implementations rather than direct channel injection (W-08 fix). The phase is provably correct against its own success bar. Future regressions in DETECT-01/05/06/07/08 surface immediately via `cargo test`.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Pipeline channels | `raw_tx` accepts events from any adapter; `sink_tx` forwards to the printer. Both are in-process tokio mpsc — no untrusted producers in Phase 1. |
| Process argv + env vars | `--override-goldberg-root`, `HALLMARK_GOLDBERG_ROOT_OVERRIDE`, and `HALLMARK_DB_PATH_OVERRIDE` accept arbitrary path strings from the user. |
| stdout | `println!("UNLOCK app_id={} ach={} source={}")` writes user-facing data; the achievement API name comes from the watched JSON file (potentially adversarial). |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-05-T1 | Tampering | --override-goldberg-root path | accept | Local-only single-user app; if an attacker has shell access to set env vars or argv, they already control the user. |
| T-05-T2 | Tampering | ach_api_name printed to stdout | mitigate | If the JSON contains a malicious string with control codes (ANSI escape, BEL, newlines), `println!` prints it as-is. We do NOT use the achievement name as a shell argument or HTML render — only stdout text. Acceptable: a malicious Goldberg state file can produce ugly stdout but cannot escalate. Phase 2 (popup) MUST sanitize when rendering in WebView. |
| T-05-D1 | DoS | Channel back-pressure | mitigate | All channels are `mpsc::channel(64)` (bounded). If the sink consumer (printer) hangs, `run_pipeline`'s `sink.send().await` blocks, naturally back-pressuring the watcher. No unbounded memory growth. |
| T-05-I1 | Info disclosure | hallmark.db on disk | accept | Per Plan 02 threat model — local-only, single-user. |
| T-05-S1 | Spoofing | session_id collision | mitigate | UUID v4 — collision probability is negligible (2^-122 per pair). |
</threat_model>

<verification>
End-of-phase verification (run from workspace root):
```powershell
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1
cargo build --manifest-path src-tauri/Cargo.toml --bin hallmark-cli --release
```
All five exit 0. The `release` build also succeeds.
</verification>

<success_criteria>
- `CrossSourceDedup` is implemented per RESEARCH.md "Pattern 3", with TTL-based sweep and 4 unit tests passing.
- `run_pipeline` consumes the `raw_rx` from `run_watcher`, applies `CrossSourceDedup`, persists via `SqliteStore::record_unlock`, and forwards kept events to a sink — REQ DETECT-07 fully wired (3 layers: debounce, content hash, cross-source TTL + DB UNIQUE INDEX).
- `SqliteStore::with_conn` helper added in Step 1 of Task 2 BEFORE the CLI binary uses it (W-06 fix).
- `hallmark-cli` binary builds and starts; supports `--override-goldberg-root` argv AND `HALLMARK_GOLDBERG_ROOT_OVERRIDE` env var; uses `paths::goldberg_redirect_map` to feed `GoldbergAdapter::new(roots, redirect_map)`; prints `UNLOCK app_id=... ach=... source=...` per kept event.
- All 5 ROADMAP Phase 1 Success Criteria have a passing automated integration test in `src-tauri/tests/integration_phase1.rs`.
- **B-01 fix:** SC3 exercises the FULL real-disk pipeline (steam_api64.dll + local_save.txt + appmanifest fixture → scan_local_save_redirects → real GoldbergAdapter → run_watcher + run_pipeline → assertion that exactly one event arrives with appid resolved from appmanifest).
- **W-08 fix:** SC4 uses two real `MockAdapter` instances (each implementing `SourceAdapter`) emitting via file-event-driven paths, NOT direct raw_tx injection.
- **W-10 fix:** Integration test file uses `use hallmark_lib::...` exclusively — no `extern crate` line.
- Total Phase 1 test count (across Plans 02–05): ~50 tests passing.
- All 5 phase requirement IDs (DETECT-01, DETECT-05, DETECT-06, DETECT-07, DETECT-08) are covered by at least one passing test:
  - DETECT-01: `sc1_single_unlock_emits_exactly_one_event_within_one_second` + Plan 04's `goldberg::tests`
  - DETECT-05: `sc2_pre_populated_state_emits_zero_events` + Plan 04's `seed_baseline_populates_from_fixture` + `run_watcher_seeds_before_attaching_watcher`
  - DETECT-06: Plan 04's `on_file_changed_skips_identical_content_via_sha256` + `run_watcher_emits_event_through_real_debouncer_within_1s`
  - DETECT-07: Plan 05's `run_pipeline_dedups_simultaneous_cross_source_events` + `sc4_cross_source_dedup_collapses_real_adapter_events_to_one` + the SQLite UNIQUE INDEX dedup tests in Plan 02
  - DETECT-08: Plan 03's 16 path-discovery tests + `sc3_local_save_txt_redirect_drives_end_to_end_pipeline` (real on-disk redirect resolution + appmanifest appid lookup) + `sc5_path_discovery_logs_every_category_to_tracing`
- Phase 1 goal achieved: a reliable, spam-free unlock event stream is flowing end-to-end for Goldberg-emulated games, ready for the Phase 2 UI layer to consume.
</success_criteria>

<output>
After completion, create `.planning/phases/01-detection-pipeline-foundation/01-05-SUMMARY.md` documenting:
the CrossSourceDedup TTL design; the SqliteStore::with_conn helper; the hallmark-cli binary structure
and its env-var/argv overrides; the 5 integration tests and which Success Criterion + REQ each maps to
(noting SC3's real-disk redirect coverage and SC4's MockAdapter-driven dedup); the total Phase 1 test
inventory (~50 tests); and explicit confirmation that all 5 phase requirement IDs are covered.
</output>
</content>
</invoke>
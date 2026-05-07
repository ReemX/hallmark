---
phase: 01-detection-pipeline-foundation
plan: 04
type: execute
wave: 3
depends_on: [01-01, 01-02, 01-03]
files_modified:
  - src-tauri/src/sources/goldberg.rs
  - src-tauri/src/sources/mod.rs
  - src-tauri/src/watcher/mod.rs
autonomous: true
requirements: [DETECT-01, DETECT-05, DETECT-06]
must_haves:
  truths:
    - "`GoldbergAdapter::new(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>)` constructs an adapter holding an in-memory baseline `HashMap<(u64, String), bool>`, per-file content-hash map, AND a redirect→appid lookup table for non-numeric directory layouts"
    - "`seed_baseline()` walks every watch root, reads every `<appid>/achievements.json` AND every redirect target's `achievements.json`, and populates the baseline with EVERY known (app_id, ach_api_name) → earned-bool — both earned AND unearned entries"
    - "`extract_app_id()` returns the appid from a numeric parent directory if present; OTHERWISE looks up the file's parent directory in the redirect_map; otherwise returns None and the adapter skips with a warn-level trace"
    - "`on_file_changed()` re-reads the file with retry on sharing-violation (PITFALLS.md #3), skips if SHA-256 of contents matches `last_hash`, parses, diffs against baseline, emits ONE `RawUnlockEvent` per `false → true` transition, then updates the baseline"
    - "`run_watcher(adapters, raw_tx)` seeds ALL baselines BEFORE attaching the debouncer (REQ DETECT-05 ordering — never reverse this)"
    - "WatcherCore uses `notify_debouncer_full::new_debouncer(Duration::from_millis(500), None, callback)` (REQ DETECT-06 — 500ms debounce window)"
    - "WatcherCore filters out non-existent watch paths with `path.exists()` BEFORE `debouncer.watch()` (PITFALLS.md / RESEARCH.md Pitfall #5 — `notify::ErrorKind::PathNotFound`)"
    - "Adapter parses Goldberg state file as `HashMap<String, GoldbergEntry>` where `GoldbergEntry { earned: bool, #[serde(default)] earned_time: u64 }` per RESEARCH.md (and per Plan 01's empirical-goldberg-schema-NOTES.md decision)"
    - "`on_file_changed` ignores any path whose filename is not `achievements.json` (top-level dir watch produces events for sibling files; filter)"
    - "Adapter does NOT use `earned_time` as the unlock signal — only the `earned` boolean transition false→true (PITFALLS.md #15)"
    - "Unit + integration tests prove: (a) seed populates baseline correctly from fixture; (b) flipping a `false` entry to `true` emits exactly one event; (c) the `earned_time:0 + earned:true` fixture entry does NOT cause a spurious event on first read after baseline; (d) identical re-write produces zero events (content hash); (e) flipping an entry that was already `true` to `true` produces zero events; (f) redirect_map fallback resolves appid from non-numeric parent directories"
  artifacts:
    - path: "src-tauri/src/sources/goldberg.rs"
      provides: "GoldbergAdapter implementing SourceAdapter, parses state JSON, diffs against baseline, emits RawUnlockEvents, supports redirect→appid lookup for non-numeric directory layouts"
      min_lines: 280
      contains: 'impl SourceAdapter for GoldbergAdapter'
    - path: "src-tauri/src/watcher/mod.rs"
      provides: "WatcherCore: registers adapter watch paths with shared notify-debouncer-full, dispatches per-event to adapters by prefix-match"
      min_lines: 100
      contains: 'new_debouncer'
  key_links:
    - from: "src-tauri/src/sources/goldberg.rs"
      to: "src-tauri/src/sources/mod.rs"
      via: "implements SourceAdapter trait"
      pattern: 'impl SourceAdapter for GoldbergAdapter'
    - from: "src-tauri/src/sources/mod.rs"
      to: "src-tauri/src/sources/goldberg.rs"
      via: "module declaration `pub mod goldberg;`"
      pattern: 'pub mod goldberg;'
    - from: "src-tauri/src/watcher/mod.rs"
      to: "src-tauri/src/sources/mod.rs"
      via: "consumes Arc<dyn SourceAdapter>"
      pattern: 'Arc<dyn SourceAdapter>'
    - from: "src-tauri/src/watcher/mod.rs"
      to: "notify-debouncer-full"
      via: "single shared debouncer driving all adapter watch paths"
      pattern: 'new_debouncer\(Duration::from_millis\(500\)'
---

<objective>
Implement the Goldberg adapter and the central WatcherCore that drives it via `notify-debouncer-full`. This plan delivers the heart of REQs DETECT-01, DETECT-05, and DETECT-06: a watcher that detects Goldberg unlocks within ~1s, never spams historic unlocks on first run, and produces exactly one event per logical write thanks to two-layer dedup (500ms debounce + SHA-256 content equality). The adapter accepts a `redirect_map: HashMap<PathBuf, u64>` from Plan 03 so it can resolve the appid for redirect targets whose parent directory is not numeric (e.g. `D:\Game1\Save\achievements.json`).

Purpose: Plans 01–03 set up the scaffold, types, persistence, and path discovery. This plan closes the loop from "FS event" to "RawUnlockEvent emitted on a tokio mpsc channel". Plan 05 wires the channel to a CLI sink and the SQLite store, completing the end-to-end flow.

Output:
- `src-tauri/src/sources/goldberg.rs` (~280–330 lines) with `GoldbergAdapter` fully implementing `SourceAdapter`, including redirect_map fallback for appid resolution
- `src-tauri/src/sources/mod.rs` updated with `pub mod goldberg;`
- `src-tauri/src/watcher/mod.rs` (~100 lines) with `pub async fn run_watcher(adapters, raw_tx)` driving notify-debouncer-full
- Unit tests for the Goldberg adapter against fixtures (proving baseline seeding, transition detection, content-hash dedup, `earned_time:0` non-spurious behaviour, redirect_map fallback)
- An integration test in `goldberg.rs` that actually writes a temp fixture file, calls `seed_baseline()`, mutates the file, calls `on_file_changed()`, and asserts an event was emitted
</objective>

<execution_context>
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/REQUIREMENTS.md
@.planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md
@.planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md
@.planning/research/PITFALLS.md
@CLAUDE.md

<interfaces>
<!-- Reference contracts for this plan -->

`SourceAdapter` trait (from Plan 02; do NOT redefine):
```rust
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;
    fn watch_paths(&self) -> Vec<PathBuf>;
    async fn seed_baseline(&self) -> anyhow::Result<()>;
    async fn on_file_changed(&self, path: PathBuf, tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>;
}
```

Goldberg state file shape (from Plan 01's empirical-goldberg-schema-NOTES.md):
```json
{
  "ACH_API_NAME_1": { "earned": true,  "earned_time": 1700000000 },
  "ACH_API_NAME_2": { "earned": false, "earned_time": 0 }
}
```
Top-level: object map. Field names: `earned` (bool), `earned_time` (u64, default 0).

Path → appid resolution (NEW: fallback chain):
1. Path layout `<root>/<appid>/achievements.json` — appid is numeric parent directory.
2. Redirect layout (e.g. `D:\Game1\Save\achievements.json`) — parent is "Save" (not numeric); look up the file's parent directory in `redirect_map: HashMap<PathBuf, u64>` provided by Plan 03's `paths::goldberg_redirect_map(&discovered)`.
3. Neither matches → skip with warn-level trace.

`notify-debouncer-full 0.7` API (from RESEARCH.md "Pattern 2"):
```rust
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
let mut debouncer = new_debouncer(
    Duration::from_millis(500),
    None,                        // tick rate auto = timeout / 4
    move |res: DebounceEventResult| { /* sync callback bridges to tokio mpsc via blocking_send */ },
)?;
debouncer.watch(&path, RecursiveMode::Recursive)?;
```

`DebounceEventResult` = `Result<Vec<DebouncedEvent>, Vec<Error>>`. Each `DebouncedEvent` has `.event: notify::Event` with `.paths: Vec<PathBuf>`.

`WatcherCore` public API consumed by Plan 05:
```rust
pub async fn run_watcher(
    adapters: Vec<Arc<dyn SourceAdapter>>,
    raw_tx: mpsc::Sender<RawUnlockEvent>,
) -> anyhow::Result<()>;
```
This function:
1. Calls `seed_baseline()` on every adapter (sequentially, before any watcher is registered).
2. Constructs ONE `notify-debouncer-full` and registers all adapters' watch paths against it.
3. In a tokio recv loop, dispatches each debounced event to the adapter whose `watch_paths()` prefix-matches the event path.
4. Returns when the receiver channel closes (graceful shutdown).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Implement GoldbergAdapter (state-file diffing, baseline seeding, content-hash dedup, redirect_map fallback)</name>
  <files>
    - src-tauri/src/sources/goldberg.rs
    - src-tauri/src/sources/mod.rs
  </files>
  <read_first>
    - src-tauri/src/sources/mod.rs (Plan 02 — confirm SourceAdapter trait shape, RawUnlockEvent fields, SourceKind variants)
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md ("Goldberg state file parse + diff" code block — the canonical reference implementation; "Pitfall 1 — first-launch state seeding"; "Pitfall 4 — schema vs state file confusion")
    - .planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md (Plan 01 output — confirms schema or records fallback decision)
    - .planning/research/PITFALLS.md (Pitfalls #3 file-locked-on-write, #15 unlock_time=0)
    - tests/fixtures/goldberg/480/achievements.json (the canonical fixture)
    - src-tauri/src/paths.rs (Plan 03 — confirms `GoldbergRedirect` shape, `goldberg_redirect_map` accessor)
  </read_first>
  <behavior>
    Tests:
    - Test 1 (`seed_baseline_populates_from_fixture`): Construct adapter pointing at `tests/fixtures/goldberg/`, call `seed_baseline()`, assert baseline contains exactly the 4 fixture entries: `(480, "ACH_WIN_ONE_GAME") → true`, `(480, "ACH_WIN_100_GAMES") → false`, `(480, "ACH_TRAVEL_FAR_ACCUM") → false`, `(480, "ACH_UNKNOWN_TIMESTAMP") → true`.
    - Test 2 (`on_file_changed_emits_event_on_false_to_true_transition`): Setup tempdir `<tmp>/480/achievements.json` with the fixture content. Seed baseline. Modify the file so `ACH_WIN_100_GAMES.earned` becomes `true`. Call `on_file_changed`. Receive on channel — exactly ONE `RawUnlockEvent { app_id: 480, ach_api_name: "ACH_WIN_100_GAMES", source: SourceKind::Goldberg, .. }`. No other events.
    - Test 3 (`on_file_changed_no_event_for_already_earned_at_seed`): Seed sees `ACH_WIN_ONE_GAME.earned = true` at startup. After seeding, NO event is emitted on the first call to `on_file_changed` even if the file is read again with the same content. (Critical for REQ DETECT-05.)
    - Test 4 (`on_file_changed_no_event_for_earned_time_zero_with_earned_true`): Fixture's `ACH_UNKNOWN_TIMESTAMP` is `earned: true, earned_time: 0`. After seeding (which marks it as already-earned), a re-read produces no event. This proves we use `earned: bool` as the unlock signal, NOT `earned_time > 0` (PITFALLS.md #15).
    - Test 5 (`on_file_changed_skips_identical_content_via_sha256`): Call `on_file_changed` on the same unchanged file twice in a row. The second call must short-circuit on hash equality (no parse, no diff, no events).
    - Test 6 (`on_file_changed_no_event_when_filename_not_achievements_json`): Pass a path whose filename is not `achievements.json` (e.g. `.../480/cooldown.txt`). Function returns `Ok(())` immediately, no event emitted.
    - Test 7 (`extract_app_id_returns_none_for_unknown_non_numeric_dir`): Path like `<tmp>/notanumber/achievements.json` with NO redirect_map entry → `extract_app_id` returns None; adapter silently skips (returns Ok, no event), does not panic.
    - Test 8 (`extract_app_id_uses_redirect_map_for_non_numeric_parent`): Path like `<tmp>/Save/achievements.json` where parent is not numeric, BUT `redirect_map` has `<tmp>/Save → 12345`; `extract_app_id` returns `Some(12345)`. The adapter then proceeds with appid 12345.
    - Test 9 (`seed_baseline_reads_redirect_targets`): Construct adapter with empty `roots` but `redirect_map` mapping `<tmp>/CustomSave → 67890`, where `<tmp>/CustomSave/achievements.json` exists. After `seed_baseline()`, the baseline contains entries keyed on `(67890, _)`.
    - Test 10 (`on_file_changed_emits_event_via_redirect_map_lookup`): Tempdir `<tmp>/Save/achievements.json` with redirect_map entry; seed baseline; flip an achievement; call `on_file_changed`; receive exactly ONE event with `app_id` = the redirect_map value.
    - Test 11 (`integration_full_cycle_against_real_disk`): Tempdir fixture, full seed → write → on_file_changed → recv event cycle, asserts the event arrives within a tokio::time::timeout of 200ms (proves no hidden blocking).
  </behavior>
  <action>
    Step 1 — Add `pub mod goldberg;` to `src-tauri/src/sources/mod.rs` at the top:
    ```rust
    pub mod goldberg;
    ```
    (Add this above the existing trait definition; it must be at module scope.)

    Step 2 — Create `src-tauri/src/sources/goldberg.rs` with the FULL implementation. The key change from RESEARCH.md's reference implementation: `GoldbergAdapter::new` now takes `(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>)`, and `extract_app_id` is now a method (not associated fn) that consults the map when the directory parse fails. `seed_baseline` also walks redirect_map keys to seed redirect targets.

    Verbatim file content:
    ```rust
    //! Goldberg / gbe_fork emulator state-file watcher.
    //!
    //! # State file vs schema file (PITFALLS.md #4)
    //!
    //! There are two `achievements.json` files in a Goldberg setup. We only read ONE:
    //!
    //! - **STATE file** (this adapter): `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json`
    //!   Shape: `{ "ACH_NAME": { "earned": bool, "earned_time": u64 } }` (object map).
    //! - **SCHEMA file** (Phase 2): `<game-dir>\steam_settings\achievements.json`
    //!   Shape: `[{ "name": "...", "displayName": "...", "description": "...", ... }]` (array).
    //!
    //! Phase 1 watches ONLY the state file. Mixing them up parses garbage.
    //!
    //! # Why `earned: bool` is the only valid unlock signal (PITFALLS.md #15)
    //!
    //! Goldberg writes `earned_time: 0` for "earned but timestamp unknown". A naive
    //! `unlock_time > 0` check would treat these as never-earned and fire spurious events
    //! after baseline. The boolean `earned` field's `false → true` transition is the
    //! only correct unlock signal.
    //!
    //! # Appid resolution: directory parse + redirect_map fallback
    //!
    //! Two directory layouts exist:
    //! 1. Default: `<save_root>/<appid>/achievements.json` — parent dir name parses as u64.
    //! 2. Redirect: `D:\Game1\Save\achievements.json` — parent is "Save" (not numeric).
    //!    Plan 03's `goldberg_redirect_map()` provides `parent_dir → appid`. The adapter
    //!    consults this map when the directory parse fails.

    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use serde::Deserialize;
    use sha2::{Digest, Sha256};
    use tokio::sync::{mpsc, RwLock};

    use super::{RawUnlockEvent, SourceAdapter, SourceKind};

    /// One entry in the Goldberg state file.
    #[derive(Deserialize, Debug, Clone)]
    struct GoldbergEntry {
        earned: bool,
        /// May be 0 or absent — DO NOT use as unlock signal (PITFALLS.md #15).
        #[serde(default)]
        #[allow(dead_code)]
        earned_time: u64,
    }

    type StateMap = HashMap<String, bool>;

    /// Goldberg / gbe_fork adapter. Watches one or more save roots; for each
    /// `<root>/<appid>/achievements.json` change (or redirect-target change),
    /// diffs current `earned` state against an in-memory baseline and emits a
    /// `RawUnlockEvent` per `false → true` transition.
    pub struct GoldbergAdapter {
        roots: Vec<PathBuf>,
        /// Map from redirect-target parent directory → appid. Populated by Plan 03's
        /// `paths::goldberg_redirect_map()`. Used as a fallback when a state file's
        /// parent directory name does not parse as u64.
        redirect_map: HashMap<PathBuf, u64>,
        baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
        last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
    }

    impl GoldbergAdapter {
        /// `roots` are the default Goldberg save roots (e.g. `%APPDATA%\GSE Saves\`).
        /// `redirect_map` is `HashMap<PathBuf, u64>` mapping each redirect target's
        /// parent directory to its resolved appid (built by `paths::goldberg_redirect_map`).
        pub fn new(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>) -> Self {
            Self {
                roots,
                redirect_map,
                baseline: Arc::new(RwLock::new(HashMap::new())),
                last_hash: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        /// Resolve appid for a state file path. Tries:
        /// 1. Numeric parse of `path.parent().file_name()` (default `<root>/<appid>/achievements.json` layout).
        /// 2. Lookup of `path.parent()` in `redirect_map` (redirect layout where parent is e.g. "Save").
        /// Returns None if neither succeeds; caller logs and skips.
        fn extract_app_id(&self, path: &Path) -> Option<u64> {
            let parent = path.parent()?;
            // Step 1: numeric directory parse
            if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
                if let Ok(id) = name.parse::<u64>() {
                    return Some(id);
                }
            }
            // Step 2: redirect_map fallback
            self.redirect_map.get(parent).copied()
        }

        fn parse_state(json: &str) -> anyhow::Result<StateMap> {
            let raw: HashMap<String, GoldbergEntry> = serde_json::from_str(json)?;
            Ok(raw.into_iter().map(|(k, v)| (k, v.earned)).collect())
        }

        /// Reentrant baseline read for tests — exposed in-crate.
        #[cfg(test)]
        pub(crate) async fn baseline_snapshot(&self) -> HashMap<(u64, String), bool> {
            self.baseline.read().await.clone()
        }
    }

    #[async_trait::async_trait]
    impl SourceAdapter for GoldbergAdapter {
        fn name(&self) -> &str { "goldberg" }
        fn kind(&self) -> SourceKind { SourceKind::Goldberg }

        fn watch_paths(&self) -> Vec<PathBuf> {
            // Watch both default roots AND redirect-target parent dirs (the latter via redirect_map keys).
            let mut paths: Vec<PathBuf> = self.roots
                .iter()
                .filter(|p| p.exists())
                .cloned()
                .collect();
            for redirect_parent in self.redirect_map.keys() {
                if redirect_parent.exists() && !paths.contains(redirect_parent) {
                    paths.push(redirect_parent.clone());
                }
            }
            paths
        }

        async fn seed_baseline(&self) -> anyhow::Result<()> {
            let mut baseline = self.baseline.write().await;
            let mut total_files = 0u32;
            let mut total_entries = 0u32;

            // Pass 1 — default roots, layout `<root>/<appid>/achievements.json`.
            for root in &self.roots {
                if !root.exists() {
                    tracing::warn!(root = %root.display(), "Goldberg root does not exist; skipping");
                    continue;
                }
                // max_depth(2) hits exactly <root>/<appid>/achievements.json without
                // chasing arbitrarily deep subdirs.
                for entry in walkdir::WalkDir::new(root).max_depth(2) {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::warn!(root = %root.display(), error = %e, "walkdir error during seed");
                            continue;
                        }
                    };
                    if !entry.file_type().is_file() { continue; }
                    if entry.file_name() != "achievements.json" { continue; }

                    let path = entry.path();
                    let Some(app_id) = self.extract_app_id(path) else {
                        tracing::warn!(path = %path.display(),
                            "could not resolve appid (numeric parse failed and not in redirect_map); skipping");
                        continue;
                    };
                    let json = match read_with_retry(path) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!(path = %path.display(), error = %e, "seed read failed; skip");
                            continue;
                        }
                    };
                    let state = match Self::parse_state(&json) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!(path = %path.display(), error = %e, "seed parse failed; skip");
                            continue;
                        }
                    };
                    total_files += 1;
                    for (api_name, earned) in state {
                        baseline.insert((app_id, api_name), earned);
                        total_entries += 1;
                    }
                }
            }

            // Pass 2 — redirect targets, layout `<redirect_parent>/achievements.json`.
            // Each redirect_map key IS the redirect target directory; the achievements.json
            // sits directly inside it (no per-appid subdir).
            for (redirect_parent, &app_id) in &self.redirect_map {
                let candidate = redirect_parent.join("achievements.json");
                if !candidate.exists() { continue; }
                let json = match read_with_retry(&candidate) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(path = %candidate.display(), error = %e,
                            "redirect-target seed read failed; skip");
                        continue;
                    }
                };
                let state = match Self::parse_state(&json) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(path = %candidate.display(), error = %e,
                            "redirect-target seed parse failed; skip");
                        continue;
                    }
                };
                total_files += 1;
                for (api_name, earned) in state {
                    baseline.insert((app_id, api_name), earned);
                    total_entries += 1;
                }
            }

            tracing::info!(
                files = total_files,
                entries = total_entries,
                roots = self.roots.len(),
                redirects = self.redirect_map.len(),
                "Goldberg baseline seeded"
            );
            Ok(())
        }

        async fn on_file_changed(
            &self,
            path: PathBuf,
            tx: mpsc::Sender<RawUnlockEvent>,
        ) -> anyhow::Result<()> {
            // Filter: events from the recursive watch may include sibling files in the
            // appid directory. We only care about achievements.json itself.
            if path.file_name().and_then(|n| n.to_str()) != Some("achievements.json") {
                return Ok(());
            }
            let Some(app_id) = self.extract_app_id(&path) else {
                tracing::debug!(path = %path.display(),
                    "could not resolve appid for event path; ignoring");
                return Ok(());
            };

            // Read with retry (PITFALLS.md #3 — file may be locked while emulator writes).
            let json = match read_with_retry(&path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "read_with_retry failed");
                    return Ok(());
                }
            };

            // Content-hash equality (REQ DETECT-06 second layer).
            let hash: [u8; 32] = Sha256::digest(json.as_bytes()).into();
            {
                let mut hashes = self.last_hash.write().await;
                if hashes.get(&path) == Some(&hash) {
                    tracing::trace!(path = %path.display(), "content unchanged (hash match); skip");
                    return Ok(());
                }
                hashes.insert(path.clone(), hash);
            }

            let state = match Self::parse_state(&json) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "state file parse failed");
                    return Ok(());
                }
            };

            // Diff against baseline. Order: read → diff → emit → THEN update baseline.
            let mut baseline = self.baseline.write().await;
            for (api_name, earned_now) in state {
                let key = (app_id, api_name.clone());
                let was = baseline.get(&key).copied().unwrap_or(false);
                if !was && earned_now {
                    let evt = RawUnlockEvent {
                        app_id,
                        ach_api_name: api_name,
                        timestamp: 0, // wall-clock stamping happens downstream
                        source: SourceKind::Goldberg,
                    };
                    if tx.send(evt).await.is_err() {
                        tracing::error!("RawUnlockEvent receiver dropped; pipeline shutting down?");
                    }
                }
                baseline.insert(key, earned_now);
            }
            Ok(())
        }
    }

    /// File read with retry on Windows sharing-violation (PITFALLS.md #3).
    /// Goldberg writes the state file open-write-close; we may hit it mid-write.
    fn read_with_retry(path: &Path) -> anyhow::Result<String> {
        let mut last_err: Option<std::io::Error> = None;
        for _ in 0..3 {
            match std::fs::read_to_string(path) {
                Ok(s) => return Ok(s),
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied
                       || e.raw_os_error() == Some(32) /* ERROR_SHARING_VIOLATION */ => {
                    last_err = Some(e);
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(last_err.unwrap().into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;
        use tokio::sync::mpsc;
        use tokio::time::timeout;

        fn fresh_tmp() -> PathBuf {
            let p = std::env::temp_dir().join(format!("hallmark-gold-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            p
        }

        fn write_fixture(root: &Path, app_id: u64, content: &str) -> PathBuf {
            let dir = root.join(app_id.to_string());
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("achievements.json");
            fs::write(&path, content).unwrap();
            path
        }

        const FIXTURE_BASELINE: &str = r#"{
            "ACH_WIN_ONE_GAME":     { "earned": true,  "earned_time": 1700000001 },
            "ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 },
            "ACH_TRAVEL_FAR_ACCUM": { "earned": false, "earned_time": 0 },
            "ACH_UNKNOWN_TIMESTAMP":{ "earned": true,  "earned_time": 0 }
        }"#;

        #[tokio::test]
        async fn seed_baseline_populates_from_fixture() {
            let root = fresh_tmp();
            write_fixture(&root, 480, FIXTURE_BASELINE);

            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            adapter.seed_baseline().await.unwrap();

            let snap = adapter.baseline_snapshot().await;
            assert_eq!(snap.len(), 4);
            assert_eq!(snap.get(&(480, "ACH_WIN_ONE_GAME".to_string())), Some(&true));
            assert_eq!(snap.get(&(480, "ACH_WIN_100_GAMES".to_string())), Some(&false));
            assert_eq!(snap.get(&(480, "ACH_TRAVEL_FAR_ACCUM".to_string())), Some(&false));
            assert_eq!(snap.get(&(480, "ACH_UNKNOWN_TIMESTAMP".to_string())), Some(&true));

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn on_file_changed_emits_event_on_false_to_true_transition() {
            let root = fresh_tmp();
            let path = write_fixture(&root, 480, FIXTURE_BASELINE);

            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            adapter.seed_baseline().await.unwrap();

            // Flip ACH_WIN_100_GAMES from false to true
            let mutated = FIXTURE_BASELINE.replace(
                r#""ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 }"#,
                r#""ACH_WIN_100_GAMES":    { "earned": true,  "earned_time": 1700000999 }"#,
            );
            assert_ne!(mutated, FIXTURE_BASELINE, "test mutation must change content");
            fs::write(&path, &mutated).unwrap();

            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(path.clone(), tx).await.unwrap();

            let evt = timeout(Duration::from_millis(200), rx.recv())
                .await.unwrap()
                .expect("expected exactly one event");
            assert_eq!(evt.app_id, 480);
            assert_eq!(evt.ach_api_name, "ACH_WIN_100_GAMES");
            assert_eq!(evt.source, SourceKind::Goldberg);

            // No additional events — channel should yield nothing more within 100ms.
            let none = timeout(Duration::from_millis(100), rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none(),
                "no further events should arrive");

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn on_file_changed_no_event_for_already_earned_at_seed() {
            let root = fresh_tmp();
            let path = write_fixture(&root, 480, FIXTURE_BASELINE);

            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            adapter.seed_baseline().await.unwrap();

            // Re-read the SAME file (simulating a debounced no-op event).
            // Mutate content slightly so the hash differs (force the diff path).
            // ACH_WIN_ONE_GAME is already true — re-emitting `earned: true` should produce no event.
            let cosmetic = FIXTURE_BASELINE.replace("1700000001", "1700000002");
            fs::write(&path, &cosmetic).unwrap();

            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(path, tx).await.unwrap();
            let none = timeout(Duration::from_millis(100), rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none(),
                "no event for already-earned achievement");

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn on_file_changed_no_event_for_earned_time_zero_with_earned_true() {
            // Specifically PITFALLS.md #15: ACH_UNKNOWN_TIMESTAMP has earned_time=0 but earned=true.
            // After baseline seeding, no event should fire on a re-read.
            let root = fresh_tmp();
            let path = write_fixture(&root, 480, FIXTURE_BASELINE);

            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            adapter.seed_baseline().await.unwrap();

            // Force hash difference by adding whitespace, but keep state semantically identical.
            fs::write(&path, format!("{}\n  ", FIXTURE_BASELINE)).unwrap();

            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(path, tx).await.unwrap();
            let none = timeout(Duration::from_millis(100), rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none(),
                "earned_time:0 + earned:true after seed must NOT emit (PITFALLS #15)");

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn on_file_changed_skips_identical_content_via_sha256() {
            let root = fresh_tmp();
            let path = write_fixture(&root, 480, FIXTURE_BASELINE);
            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            adapter.seed_baseline().await.unwrap();

            // Without modifying file at all, call on_file_changed twice.
            // First call hashes + diffs (no event because everything matches baseline).
            // Second call short-circuits on hash equality.
            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(path.clone(), tx.clone()).await.unwrap();
            adapter.on_file_changed(path, tx).await.unwrap();

            let none = timeout(Duration::from_millis(100), rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none());

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn on_file_changed_no_event_when_filename_not_achievements_json() {
            let root = fresh_tmp();
            let dir = root.join("480");
            fs::create_dir_all(&dir).unwrap();
            let other = dir.join("cooldown.txt");
            fs::write(&other, "irrelevant").unwrap();

            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(other, tx).await.unwrap();

            let none = timeout(Duration::from_millis(50), rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none());

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn extract_app_id_returns_none_for_unknown_non_numeric_dir() {
            let root = fresh_tmp();
            let dir = root.join("notanumber");
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("achievements.json");
            fs::write(&path, "{}").unwrap();

            let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
            assert_eq!(adapter.extract_app_id(&path), None,
                "non-numeric dir without redirect_map entry should return None");
            adapter.seed_baseline().await.unwrap();
            let snap = adapter.baseline_snapshot().await;
            assert!(snap.is_empty(), "non-numeric dir should produce no baseline entries");

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn extract_app_id_uses_redirect_map_for_non_numeric_parent() {
            let root = fresh_tmp();
            let save_dir = root.join("Save");
            fs::create_dir_all(&save_dir).unwrap();
            let path = save_dir.join("achievements.json");
            fs::write(&path, "{}").unwrap();

            let mut redirect_map = HashMap::new();
            redirect_map.insert(save_dir.clone(), 12345);

            let adapter = GoldbergAdapter::new(vec![], redirect_map);
            assert_eq!(adapter.extract_app_id(&path), Some(12345),
                "redirect_map should resolve appid for non-numeric parent");

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn seed_baseline_reads_redirect_targets() {
            let root = fresh_tmp();
            let save_dir = root.join("CustomSave");
            fs::create_dir_all(&save_dir).unwrap();
            fs::write(save_dir.join("achievements.json"), FIXTURE_BASELINE).unwrap();

            let mut redirect_map = HashMap::new();
            redirect_map.insert(save_dir.clone(), 67890);

            let adapter = GoldbergAdapter::new(vec![], redirect_map);
            adapter.seed_baseline().await.unwrap();

            let snap = adapter.baseline_snapshot().await;
            assert_eq!(snap.len(), 4, "expected 4 entries from redirect target seed; got {:?}", snap);
            assert_eq!(snap.get(&(67890, "ACH_WIN_ONE_GAME".to_string())), Some(&true));
            assert_eq!(snap.get(&(67890, "ACH_WIN_100_GAMES".to_string())), Some(&false));

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn on_file_changed_emits_event_via_redirect_map_lookup() {
            let root = fresh_tmp();
            let save_dir = root.join("Save");
            fs::create_dir_all(&save_dir).unwrap();
            let path = save_dir.join("achievements.json");
            fs::write(&path, FIXTURE_BASELINE).unwrap();

            let mut redirect_map = HashMap::new();
            redirect_map.insert(save_dir.clone(), 99999);

            let adapter = GoldbergAdapter::new(vec![], redirect_map);
            adapter.seed_baseline().await.unwrap();

            // Flip ACH_WIN_100_GAMES
            let mutated = FIXTURE_BASELINE.replace(
                r#""ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 }"#,
                r#""ACH_WIN_100_GAMES":    { "earned": true,  "earned_time": 1700000999 }"#,
            );
            fs::write(&path, &mutated).unwrap();

            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(path.clone(), tx).await.unwrap();

            let evt = timeout(Duration::from_millis(200), rx.recv())
                .await.unwrap()
                .expect("expected one event via redirect_map lookup");
            assert_eq!(evt.app_id, 99999, "appid should come from redirect_map");
            assert_eq!(evt.ach_api_name, "ACH_WIN_100_GAMES");

            let _ = fs::remove_dir_all(&root);
        }

        #[tokio::test]
        async fn integration_full_cycle_against_real_disk() {
            let root = fresh_tmp();
            let path = write_fixture(&root, 480, FIXTURE_BASELINE);
            let adapter = Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));

            adapter.seed_baseline().await.unwrap();
            // Flip TWO entries in one write: ACH_WIN_100_GAMES and ACH_TRAVEL_FAR_ACCUM
            let mutated = FIXTURE_BASELINE
                .replace(
                    r#""ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 }"#,
                    r#""ACH_WIN_100_GAMES":    { "earned": true,  "earned_time": 1700001000 }"#,
                )
                .replace(
                    r#""ACH_TRAVEL_FAR_ACCUM": { "earned": false, "earned_time": 0 }"#,
                    r#""ACH_TRAVEL_FAR_ACCUM": { "earned": true,  "earned_time": 1700001001 }"#,
                );
            fs::write(&path, &mutated).unwrap();

            let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
            adapter.on_file_changed(path, tx).await.unwrap();

            let mut received = Vec::new();
            for _ in 0..2 {
                match timeout(Duration::from_millis(200), rx.recv()).await {
                    Ok(Some(e)) => received.push(e),
                    _ => break,
                }
            }
            assert_eq!(received.len(), 2, "expected both transitions to emit; got {:?}", received);
            let names: Vec<&str> = received.iter().map(|e| e.ach_api_name.as_str()).collect();
            assert!(names.contains(&"ACH_WIN_100_GAMES"));
            assert!(names.contains(&"ACH_TRAVEL_FAR_ACCUM"));

            let _ = fs::remove_dir_all(&root);
        }
    }
    ```

    Step 3 — Run:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --lib sources::goldberg::tests
    ```
    All 11 tests pass.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/sources/goldberg.rs)) { exit 1 }; $m = Get-Content src-tauri/src/sources/mod.rs -Raw; if ($m -notmatch 'pub mod goldberg;') { exit 2 }; $g = Get-Content src-tauri/src/sources/goldberg.rs -Raw; if ($g -notmatch 'pub struct GoldbergAdapter') { exit 10 }; if ($g -notmatch 'impl SourceAdapter for GoldbergAdapter') { exit 11 }; if ($g -notmatch 'sha2::Sha256' -and $g -notmatch 'use sha2') { exit 12 }; if ($g -notmatch 'async fn seed_baseline') { exit 13 }; if ($g -notmatch 'async fn on_file_changed') { exit 14 }; if ($g -notmatch 'serde\(default\)') { exit 15 }; if ($g -notmatch 'earned: bool') { exit 16 }; if ($g -notmatch 'fn read_with_retry') { exit 17 }; if ($g -notmatch 'ERROR_SHARING_VIOLATION' -and $g -notmatch 'raw_os_error\(\) == Some\(32\)') { exit 18 }; if ($g -notmatch 'walkdir::WalkDir') { exit 19 }; if ($g -notmatch 'redirect_map: HashMap<PathBuf, u64>') { exit 20 }; if ($g -notmatch 'extract_app_id_uses_redirect_map_for_non_numeric_parent') { exit 21 }; if ($g -notmatch 'seed_baseline_reads_redirect_targets') { exit 22 }; if ($g -notmatch 'on_file_changed_emits_event_via_redirect_map_lookup') { exit 23 }; cargo test --manifest-path src-tauri/Cargo.toml --lib sources::goldberg::tests 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; Write-Host 'goldberg adapter OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/src/sources/goldberg.rs` exists.
    - `src-tauri/src/sources/mod.rs` declares `pub mod goldberg;`.
    - `goldberg.rs` contains `pub struct GoldbergAdapter` with private fields `roots`, `redirect_map: HashMap<PathBuf, u64>`, `baseline: Arc<RwLock<HashMap<(u64, String), bool>>>`, `last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>`.
    - `GoldbergAdapter::new` takes `(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>)` — both arguments required (Plan 05's CLI binary and integration tests pass an empty `HashMap::new()` if no redirects are needed).
    - `extract_app_id` is a method (`&self`) that tries numeric parse first, then falls back to `self.redirect_map.get(parent).copied()`.
    - Contains `impl SourceAdapter for GoldbergAdapter` with all 5 trait methods implemented.
    - `watch_paths()` returns the union of existing `roots` AND existing `redirect_map.keys()`.
    - `seed_baseline` walks `walkdir::WalkDir::new(root).max_depth(2)` filtering for files named `achievements.json` for default roots, AND additionally seeds redirect targets directly via `<redirect_parent>/achievements.json`.
    - `on_file_changed` filters out non-`achievements.json` paths AS THE FIRST GUARD.
    - `on_file_changed` calls SHA-256 hashing (`Sha256::digest(json.as_bytes())`) and short-circuits on hash equality with `last_hash`.
    - `read_with_retry` exists as a free function, retries 3 times with 50ms sleeps on `ErrorKind::PermissionDenied` or `raw_os_error() == Some(32)`.
    - `GoldbergEntry` struct has `earned: bool` and `#[serde(default)] earned_time: u64`.
    - Adapter NEVER references `earned_time` in the unlock-decision logic — grep `goldberg.rs` for `earned_time` returns only the struct definition + a doc comment.
    - `cargo test --manifest-path src-tauri/Cargo.toml --lib sources::goldberg::tests` exits 0; all 11 tests pass: `seed_baseline_populates_from_fixture`, `on_file_changed_emits_event_on_false_to_true_transition`, `on_file_changed_no_event_for_already_earned_at_seed`, `on_file_changed_no_event_for_earned_time_zero_with_earned_true`, `on_file_changed_skips_identical_content_via_sha256`, `on_file_changed_no_event_when_filename_not_achievements_json`, `extract_app_id_returns_none_for_unknown_non_numeric_dir`, `extract_app_id_uses_redirect_map_for_non_numeric_parent`, `seed_baseline_reads_redirect_targets`, `on_file_changed_emits_event_via_redirect_map_lookup`, `integration_full_cycle_against_real_disk`.
  </acceptance_criteria>
  <done>The GoldbergAdapter implements all of REQ DETECT-01 (Goldberg detection), DETECT-05 (baseline seeding), and the per-file half of DETECT-06 (content hash). Eleven tests prove correct behaviour on real fixture files INCLUDING the redirect_map fallback path for non-numeric directory layouts. Plan 05 will integration-test the watcher event flow.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Implement WatcherCore — single notify-debouncer-full driving all adapters</name>
  <files>
    - src-tauri/src/watcher/mod.rs
  </files>
  <read_first>
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md ("Pattern 2: notify-debouncer-full as the single watcher entry point" — full code block)
    - src-tauri/src/sources/mod.rs (Plan 02 — confirms SourceAdapter trait & RawUnlockEvent shape)
    - src-tauri/src/sources/goldberg.rs (just-created — the only adapter to wire in Phase 1)
    - src-tauri/Cargo.toml (confirm `notify-debouncer-full = "0.7"`)
  </read_first>
  <behavior>
    Tests:
    - Test 1 (`run_watcher_seeds_before_attaching_watcher`): A test adapter records the order of `seed_baseline()` and `on_file_changed()` calls. After spawning `run_watcher` and writing a file event, the recorded sequence MUST start with `seed_baseline` and only THEN have `on_file_changed`.
    - Test 2 (`run_watcher_filters_nonexistent_paths`): Adapter declares two paths — one exists, one doesn't. WatcherCore registers only the existing one and logs a warn for the missing one. The watcher does NOT return Err for the missing path.
    - Test 3 (`run_watcher_dispatches_events_via_path_prefix_match`): Two adapters (one for `<tmp>/A`, one for `<tmp>/B`), one shared `raw_tx`. Write a file under `<tmp>/A/something/achievements.json`. Only adapter A's `on_file_changed` is called.
    - Test 4 (`run_watcher_emits_event_through_real_debouncer_within_1s`): End-to-end: GoldbergAdapter against a fixture in tempdir, run_watcher launched in tokio task, mutate the fixture file, assert exactly ONE `RawUnlockEvent` arrives on the channel within 1500ms (allows for the 500ms debounce + dispatch time). This is the closest test to ROADMAP Success Criterion #1 ("exactly one unlock event in the CLI test harness within one second").
  </behavior>
  <action>
    Create `src-tauri/src/watcher/mod.rs`. Verbatim:

    ```rust
    //! Watcher core — single `notify-debouncer-full` instance driving all source adapters.
    //!
    //! Phase 1 wires only `GoldbergAdapter` (Plan 04 — `sources::goldberg`). Phase 3 will
    //! add Steam-legit, CreamAPI, and SmartSteamEmu adapters; the only change required
    //! is more entries in the `Vec<Arc<dyn SourceAdapter>>` passed to `run_watcher`.
    //!
    //! # Why ONE debouncer for ALL adapters
    //!
    //! Each adapter could spawn its own watcher, but a single debouncer:
    //! 1. Enforces a uniform 500ms debounce policy (REQ DETECT-06).
    //! 2. Prevents adapter-vs-adapter buffer-size races on `ReadDirectoryChangesW`.
    //! 3. Centralizes the sync-callback → tokio-mpsc bridge (one place to get right).
    //!
    //! # Ordering guarantee (REQ DETECT-05)
    //!
    //! `seed_baseline()` MUST complete on EVERY adapter BEFORE the debouncer is wired up.
    //! Reversing this order means an adapter could see a file event before its baseline
    //! is set, treating every existing achievement as a new unlock — the spam scenario
    //! REQ DETECT-05 forbids. This invariant is enforced by the function-call order
    //! in `run_watcher` and asserted by `run_watcher_seeds_before_attaching_watcher`.

    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use notify::RecursiveMode;
    use notify_debouncer_full::{new_debouncer, DebounceEventResult};
    use tokio::sync::mpsc;

    use crate::sources::{RawUnlockEvent, SourceAdapter};

    /// Run the central watcher. Seeds every adapter's baseline first (REQ DETECT-05),
    /// then constructs a single `notify-debouncer-full` driving all adapters' watch
    /// paths, and dispatches debounced events back to the matching adapter.
    ///
    /// Returns when `raw_tx` is dropped (graceful shutdown via channel close on the
    /// receiver side), or on a watcher setup error.
    pub async fn run_watcher(
        adapters: Vec<Arc<dyn SourceAdapter>>,
        raw_tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        // ----- Phase 1: seed baselines BEFORE attaching watchers (REQ DETECT-05) -----
        for adapter in &adapters {
            adapter.seed_baseline().await?;
            tracing::info!(adapter = adapter.name(), "Baseline seeded");
        }

        // ----- Phase 2: construct shared debouncer + register every adapter's paths -----
        let (notify_tx, mut notify_rx) = mpsc::channel::<DebounceEventResult>(64);

        // notify-debouncer-full's callback is sync (runs on debouncer's tick thread).
        // Bridge to tokio mpsc via `blocking_send` — fast forward, no blocking work here.
        let mut debouncer = new_debouncer(
            Duration::from_millis(500), // REQ DETECT-06: 500ms debounce window
            None,                       // tick_rate auto = timeout / 4
            move |res: DebounceEventResult| {
                if let Err(e) = notify_tx.blocking_send(res) {
                    tracing::warn!(error = %e, "notify→tokio bridge full or closed");
                }
            },
        )?;

        let mut total_watched = 0usize;
        for adapter in &adapters {
            for path in adapter.watch_paths() {
                if !path.exists() {
                    tracing::warn!(adapter = adapter.name(), path = %path.display(),
                        "watch path does not exist; skipping (PathNotFound would error)");
                    continue;
                }
                match debouncer.watch(&path, RecursiveMode::Recursive) {
                    Ok(()) => {
                        tracing::info!(adapter = adapter.name(), path = %path.display(),
                            "watching path recursively");
                        total_watched += 1;
                    }
                    Err(e) => {
                        tracing::warn!(adapter = adapter.name(), path = %path.display(),
                            error = %e, "debouncer.watch failed");
                    }
                }
            }
        }
        tracing::info!(
            adapters = adapters.len(),
            paths = total_watched,
            "WatcherCore active"
        );

        // ----- Phase 3: event loop -----
        while let Some(res) = notify_rx.recv().await {
            match res {
                Ok(events) => {
                    for event in events {
                        for path in &event.event.paths {
                            dispatch(&adapters, path.clone(), &raw_tx).await;
                        }
                    }
                }
                Err(errors) => {
                    for e in errors {
                        tracing::warn!(error = %e, "notify watcher error");
                    }
                }
            }
        }

        tracing::info!("WatcherCore shutting down (notify channel closed)");
        Ok(())
    }

    /// Find the adapter whose `watch_paths()` prefix-matches `path`, then forward.
    /// O(adapters × paths_per_adapter); negligible with small adapter counts.
    async fn dispatch(
        adapters: &[Arc<dyn SourceAdapter>],
        path: PathBuf,
        raw_tx: &mpsc::Sender<RawUnlockEvent>,
    ) {
        for adapter in adapters {
            if adapter.watch_paths().iter().any(|wp| path.starts_with(wp)) {
                if let Err(e) = adapter.on_file_changed(path.clone(), raw_tx.clone()).await {
                    tracing::warn!(adapter = adapter.name(), path = %path.display(),
                        error = %e, "adapter on_file_changed errored");
                }
                return; // first prefix-match wins; adapters MUST not have overlapping roots
            }
        }
        tracing::trace!(path = %path.display(), "no adapter claims this path; ignoring");
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::sources::{RawUnlockEvent, SourceAdapter, SourceKind};
        use crate::sources::goldberg::GoldbergAdapter;
        use std::collections::HashMap;
        use std::fs;
        use std::sync::atomic::{AtomicU32, Ordering};
        use tokio::time::timeout;

        // A spy adapter that records the order of method calls.
        struct OrderSpy {
            paths: Vec<PathBuf>,
            seed_count: AtomicU32,
            change_count: AtomicU32,
            change_after_seed: AtomicU32,
        }

        impl OrderSpy {
            fn new(paths: Vec<PathBuf>) -> Self {
                Self {
                    paths,
                    seed_count: AtomicU32::new(0),
                    change_count: AtomicU32::new(0),
                    change_after_seed: AtomicU32::new(0),
                }
            }
        }

        #[async_trait::async_trait]
        impl SourceAdapter for OrderSpy {
            fn name(&self) -> &str { "order_spy" }
            fn kind(&self) -> SourceKind { SourceKind::Goldberg }
            fn watch_paths(&self) -> Vec<PathBuf> {
                self.paths.iter().filter(|p| p.exists()).cloned().collect()
            }
            async fn seed_baseline(&self) -> anyhow::Result<()> {
                self.seed_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            async fn on_file_changed(
                &self,
                _path: PathBuf,
                _tx: mpsc::Sender<RawUnlockEvent>,
            ) -> anyhow::Result<()> {
                let already_seeded = self.seed_count.load(Ordering::SeqCst) > 0;
                self.change_count.fetch_add(1, Ordering::SeqCst);
                if already_seeded {
                    self.change_after_seed.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            }
        }

        fn fresh_tmp() -> PathBuf {
            let p = std::env::temp_dir().join(format!("hallmark-watcher-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            p
        }

        #[tokio::test]
        async fn run_watcher_seeds_before_attaching_watcher() {
            let dir = fresh_tmp();
            let spy = Arc::new(OrderSpy::new(vec![dir.clone()]));
            let (raw_tx, _raw_rx) = mpsc::channel::<RawUnlockEvent>(8);
            let adapters: Vec<Arc<dyn SourceAdapter>> = vec![spy.clone()];

            let handle = tokio::spawn(run_watcher(adapters, raw_tx));
            // Give the watcher a moment to seed and attach.
            tokio::time::sleep(Duration::from_millis(300)).await;

            // Trigger a file event.
            fs::write(dir.join("test.txt"), b"x").unwrap();

            // Wait long enough for debounce + dispatch.
            tokio::time::sleep(Duration::from_millis(900)).await;

            assert_eq!(spy.seed_count.load(Ordering::SeqCst), 1, "seed_baseline called exactly once");
            // change_count >= 1 OR change_count == change_after_seed — either way, every change happens after seeding.
            let changes = spy.change_count.load(Ordering::SeqCst);
            let after_seed = spy.change_after_seed.load(Ordering::SeqCst);
            assert_eq!(changes, after_seed,
                "every on_file_changed must occur after seed_baseline (got {} changes, {} after seed)",
                changes, after_seed);

            handle.abort();
            let _ = fs::remove_dir_all(&dir);
        }

        #[tokio::test]
        async fn run_watcher_filters_nonexistent_paths() {
            let real = fresh_tmp();
            let phantom = real.join("does-not-exist");
            let spy = Arc::new(OrderSpy::new(vec![real.clone(), phantom]));
            let (raw_tx, _raw_rx) = mpsc::channel::<RawUnlockEvent>(8);
            let adapters: Vec<Arc<dyn SourceAdapter>> = vec![spy.clone()];

            let handle = tokio::spawn(run_watcher(adapters, raw_tx));
            tokio::time::sleep(Duration::from_millis(200)).await;
            // If the phantom path had been registered, run_watcher would have errored before this point.
            assert!(!handle.is_finished(), "run_watcher should still be running");

            handle.abort();
            let _ = fs::remove_dir_all(&real);
        }

        #[tokio::test]
        async fn run_watcher_emits_event_through_real_debouncer_within_1s() {
            let root = fresh_tmp();
            let appid_dir = root.join("480");
            fs::create_dir_all(&appid_dir).unwrap();
            let path = appid_dir.join("achievements.json");
            let baseline = r#"{"ACH_X":{"earned":false,"earned_time":0}}"#;
            fs::write(&path, baseline).unwrap();

            let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));
            let (raw_tx, mut raw_rx) = mpsc::channel::<RawUnlockEvent>(8);

            let handle = tokio::spawn(run_watcher(vec![adapter], raw_tx));
            tokio::time::sleep(Duration::from_millis(300)).await; // seed + attach

            // Flip the achievement
            fs::write(&path, r#"{"ACH_X":{"earned":true,"earned_time":1700000999}}"#).unwrap();

            let evt = timeout(Duration::from_millis(1500), raw_rx.recv())
                .await
                .expect("event should arrive within 1500ms (500ms debounce + slack)")
                .expect("expected Some(event)");
            assert_eq!(evt.app_id, 480);
            assert_eq!(evt.ach_api_name, "ACH_X");
            assert_eq!(evt.source, SourceKind::Goldberg);

            // No further events for the next 800ms
            let none = timeout(Duration::from_millis(800), raw_rx.recv()).await;
            assert!(none.is_err() || none.unwrap().is_none(),
                "no further events should arrive (Success Criterion #1: no duplicates within 5s)");

            handle.abort();
            let _ = fs::remove_dir_all(&root);
        }
    }
    ```

    Run:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --lib watcher::tests -- --nocapture
    ```
    All 3 tests pass.

    Then run a full check:
    ```powershell
    cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    ```
    Must exit 0.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/watcher/mod.rs)) { exit 1 }; $w = Get-Content src-tauri/src/watcher/mod.rs -Raw; if ($w -notmatch 'pub async fn run_watcher') { exit 10 }; if ($w -notmatch 'new_debouncer\(') { exit 11 }; if ($w -notmatch 'Duration::from_millis\(500\)') { exit 12 }; if ($w -notmatch 'RecursiveMode::Recursive') { exit 13 }; if ($w -notmatch 'seed_baseline\(\)\.await') { exit 14 }; if ($w -notmatch 'blocking_send') { exit 15 }; if ($w -notmatch 'Arc<dyn SourceAdapter>') { exit 16 }; if ($w -notmatch 'path\.exists\(\)') { exit 17 }; if ($w -notmatch 'starts_with') { exit 18 }; cargo check --manifest-path src-tauri/Cargo.toml --all-targets 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; cargo test --manifest-path src-tauri/Cargo.toml --lib watcher::tests 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 40 }; Write-Host 'watcher OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/src/watcher/mod.rs` exists.
    - Contains `pub async fn run_watcher(adapters: Vec<Arc<dyn SourceAdapter>>, raw_tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>`.
    - Contains `new_debouncer(Duration::from_millis(500), None, ...)` (the EXACT 500ms debounce per REQ DETECT-06).
    - Contains `RecursiveMode::Recursive` (recursive watch).
    - Contains `path.exists()` filter BEFORE `debouncer.watch()` (PITFALLS.md / RESEARCH.md Pitfall #5).
    - Contains `path.starts_with(wp)` for adapter dispatch (path prefix-match).
    - `seed_baseline()` on every adapter is called BEFORE `new_debouncer(...)` is constructed — verify by code inspection (the seeding for-loop appears textually before the `new_debouncer` call).
    - Adapter dispatch is `for adapter in adapters` then `if any path prefix matches`, then `return` after first match (no double-dispatch).
    - `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` exits 0.
    - `cargo test --manifest-path src-tauri/Cargo.toml --lib watcher::tests` exits 0; all 3 tests pass: `run_watcher_seeds_before_attaching_watcher`, `run_watcher_filters_nonexistent_paths`, `run_watcher_emits_event_through_real_debouncer_within_1s`.
    - `run_watcher_emits_event_through_real_debouncer_within_1s` passes within ~3s wall-clock — proving the FULL pipeline (notify event → 500ms debounce → adapter dispatch → mpsc emit) works end-to-end.
  </acceptance_criteria>
  <done>WatcherCore is wired. The full pipeline DISK → notify event → 500ms debounce → adapter prefix-match → on_file_changed → RawUnlockEvent emitted on mpsc is operational and proven by integration test. Plan 05's CLI test harness composes Plans 03+04 into the full Phase 1 deliverable.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| disk → process | Goldberg state files are user-writable JSON. Untrusted JSON crosses on every event. |
| notify watcher → tokio runtime | `notify-debouncer-full`'s sync callback runs on a non-tokio thread; data crosses the sync→async boundary via `blocking_send`. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-04-T1 | Tampering | Goldberg state-file JSON | mitigate | `serde_json::from_str` with strongly-typed `GoldbergEntry`. Parse failure logs warn + returns Ok (skip), never panics. Adversarial JSON cannot crash the watcher. |
| T-04-D1 | DoS | Watcher event flood | mitigate | 500ms debounce collapses event bursts (REQ DETECT-06 layer 1). SHA-256 content hash short-circuits identical re-writes (REQ DETECT-06 layer 2). The notify-debouncer-full file-ID cache + rename tracking handle Steam's open-write-rename pattern. |
| T-04-D2 | DoS | seed_baseline on huge save dir | mitigate | `walkdir::WalkDir::new(root).max_depth(2)` bounds traversal to `<root>/<appid>/achievements.json`. A user with 1000 Goldberg games triggers 1000 file reads at startup, each ≤100KB, completing in seconds. Acceptable. |
| T-04-T2 | Tampering | redirect_map keys (from Plan 03 path discovery) | accept | Plan 03 already validates redirect targets exist + appmanifest matches; this plan only consumes the validated map. We never write to or execute from these paths. |
| T-04-I1 | Info disclosure | Path strings in tracing logs | accept | Logs show full paths including username. Local stdout only; no telemetry. |
| T-04-S1 | Spoofing | Adapter prefix-match collision | mitigate | Each adapter declares non-overlapping watch paths. `dispatch()` does `return` after first match — first-registered adapter wins for ambiguous paths. Phase 1 has only one adapter, so this matters only as a Phase 3 forward-thought (documented inline). |
</threat_model>

<verification>
End-of-plan verification:
```powershell
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::goldberg
cargo test --manifest-path src-tauri/Cargo.toml --lib watcher
```
All four exit 0. Total tests across Plans 02–04: 10 (sources/store) + 16 (paths) + 11 (goldberg) + 3 (watcher) = ~40 unit tests. Plan 05 adds the integration tests.
</verification>

<success_criteria>
- `GoldbergAdapter` correctly parses the Goldberg state file shape (per RESEARCH.md + Plan 01's empirical NOTES.md).
- `GoldbergAdapter::new(roots, redirect_map)` accepts a redirect→appid map and consults it when the directory parse fails (B-02 fix).
- `seed_baseline()` reads ALL existing state files (default-root layout AND redirect-target layout) and populates the in-memory baseline before any watcher fires (REQ DETECT-05).
- `on_file_changed()` short-circuits on SHA-256 content hash equality (REQ DETECT-06 layer 2).
- `on_file_changed()` emits exactly ONE `RawUnlockEvent` per `false → true` transition; `earned_time` is NEVER consulted for the unlock decision (PITFALLS.md #15).
- `run_watcher` enforces seed-then-attach ordering: `seed_baseline` runs on every adapter BEFORE `new_debouncer` is constructed.
- `run_watcher` uses `notify_debouncer_full::new_debouncer(Duration::from_millis(500), None, callback)` (REQ DETECT-06 layer 1).
- `run_watcher` filters out non-existent watch paths (PITFALLS.md / RESEARCH.md Pitfall #5).
- The integration test `run_watcher_emits_event_through_real_debouncer_within_1s` exercises the full pipeline against real disk and asserts: (a) one event arrives within 1500ms of the file write, (b) no duplicate events arrive in the following 800ms.
- 14 new unit tests pass (11 in `goldberg::tests` + 3 in `watcher::tests`), three of which exercise the new redirect_map fallback path.
- REQs DETECT-01, DETECT-05, DETECT-06 are fully covered.
</success_criteria>

<output>
After completion, create `.planning/phases/01-detection-pipeline-foundation/01-04-SUMMARY.md` documenting:
the GoldbergAdapter implementation (baseline seeding, content-hash dedup, transition diff, redirect_map fallback);
the WatcherCore design (single shared debouncer, ordered seed-then-attach, prefix-match dispatch);
all 14 passing tests; and the exact public API Plan 05 will compose
(`Arc::new(GoldbergAdapter::new(roots, redirect_map))` + `tokio::spawn(watcher::run_watcher(...))`).
</output>
</content>
</invoke>
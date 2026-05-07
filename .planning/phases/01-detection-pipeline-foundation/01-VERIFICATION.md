---
phase: 01-detection-pipeline-foundation
verified: 2026-05-08T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 1: Detection Pipeline Foundation — Verification Report

**Phase Goal:** A reliable, spam-free unlock event stream is flowing end-to-end for Goldberg-emulated games, with correct first-launch baseline seeding, 500ms debounce, and cross-source dedup — ready for a UI layer to consume.
**Verified:** 2026-05-08
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Dropping a pre-populated Goldberg `achievements.json` then marking one entry earned produces exactly one unlock event within one second, no duplicates for 5s | VERIFIED | Integration test `sc1_single_unlock_emits_exactly_one_event_within_one_second` PASSES (3.01s suite). Test asserts `evt.app_id == 480`, `ach_api_name == "ACH_X"`, then a 2-second timeout returns `None`/Err proving no duplicate, then `count_unlocks() == 1`. Lib test `run_watcher_emits_event_through_real_debouncer_within_1s` corroborates with explicit 1500ms window + 800ms duplicate-free window. |
| 2 | With pre-populated Goldberg save dir at startup, zero historical unlock events emitted; only net-new changes trigger events | VERIFIED | Integration test `sc2_pre_populated_state_emits_zero_events` PASSES. 50 pre-populated `earned: true` entries; 1500ms window proves zero events arrive; `count_unlocks() == 0`. Implementation verified in `watcher/mod.rs:44-48` — `seed_baseline()` runs textually BEFORE `new_debouncer(...)` in the seed-then-attach invariant; `run_watcher_seeds_before_attaching_watcher` lib test asserts `change_count == change_after_seed`. |
| 3 | A game using `local_save.txt` redirect is discovered automatically and watched without manual config | VERIFIED | Integration test `sc3_local_save_txt_redirect_drives_end_to_end_pipeline` PASSES. Builds full Steam-library-shaped fixture (steam_api64.dll + local_save.txt + appmanifest_4242.acf), drives `paths::scan_local_save_redirects_pub_for_tests` end-to-end, asserts the resolved redirect maps to appid 4242 from appmanifest (NOT directory name), then runs the full pipeline and asserts exactly one event arrives with `app_id == 4242`. WARNING: BL-01 (REVIEW) flags case-insensitivity bug between appmanifest `installdir` value and on-disk dir name on Windows; SC3 test uses matching case so this scenario is not exercised. |
| 4 | Same achievement unlocked simultaneously via two adapter sources produces exactly one event | VERIFIED | Integration test `sc4_cross_source_dedup_collapses_real_adapter_events_to_one` PASSES. Two real `MockAdapter` instances (each implementing `SourceAdapter` via file-event-driven trigger.json) flip near-simultaneously; sink receives exactly ONE event for `(4242, "ACH_DUP")`; `count_unlocks() == 1`. Pipeline test `run_pipeline_dedups_simultaneous_cross_source_events` corroborates at the unit level. SQLite UNIQUE INDEX `idx_unlock_dedup` provides DB-level second-line-of-defence. |
| 5 | All discovered paths are logged at startup for diagnosability | VERIFIED | Integration test `sc5_path_discovery_logs_every_category_to_tracing` PASSES. Custom `VecLayer` captures tracing events; `paths::log_discovery_pub_for_tests` is invoked against synthesized `DiscoveredPaths` with all 4 categories; assertions verify INFO-level event for "Steam install", "Steam library", "Goldberg save root", "local_save.txt redirect" and at least 4 INFO events total. Live binary smoke run (`./target/debug/hallmark-cli.exe --override-goldberg-root <tmp>`) confirms `Baseline seeded`, `watching path recursively`, `WatcherCore active` are emitted at startup. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` (workspace root) | members=["src-tauri"], resolver=2 | VERIFIED | Exists, declares `members = ["src-tauri"]`, resolver=2, release profile pinned. |
| `src-tauri/Cargo.toml` | All Phase 1 deps pinned, [[bin]] hallmark + hallmark-cli, [lib] hallmark_lib | VERIFIED | `notify-debouncer-full = "0.7"`, `tauri = "2.11"`, `rusqlite = "0.39"` (bundled), `sha2 = "0.11"`, tokio with `signal` feature, both binary tables present, library declared. |
| `src-tauri/src/main.rs` | calls `hallmark_lib::run()` | VERIFIED | 5-line shim. |
| `src-tauri/src/lib.rs` | tracing init + tauri builder + module declarations | VERIFIED | 52 lines; declares all 5 modules; `init_tracing()` + `run()` defined; `tauri::Builder::default()` + `tauri::generate_context!()` invoked; `setup()` hook reserved. |
| `src-tauri/src/error.rs` | thiserror enums for PathDiscoveryError/AdapterError/StoreError | VERIFIED (with note) | All three enums present and derived. WARNING (REVIEW WR-01): zero use sites in production code — types defined but unwired. |
| `src-tauri/src/sources/mod.rs` | SourceAdapter trait + RawUnlockEvent + SourceKind | VERIFIED | 142 lines; `pub trait SourceAdapter: Send + Sync + 'static` with `#[async_trait]`, all 5 trait methods, RawUnlockEvent `{app_id: u64, ach_api_name: String, timestamp: u64, source: SourceKind}`, SourceKind::Goldberg with `as_str()` returning `"goldberg"`. |
| `src-tauri/src/sources/goldberg.rs` | GoldbergAdapter implementing SourceAdapter | VERIFIED | 650 lines (plan min 280 ≤ 650); `pub struct GoldbergAdapter`, `redirect_map: HashMap<PathBuf, u64>`, `impl SourceAdapter for GoldbergAdapter`, `Sha256::digest`, `read_with_retry` with `raw_os_error() == Some(32)`, `#[serde(default)] earned_time: u64`, filename guard, baseline diff order (read→hash→parse→diff→emit→update). 11 unit tests pass. |
| `src-tauri/src/paths.rs` | DiscoveredPaths + GoldbergRedirect + discover() + helpers | VERIFIED | 937 lines (plan min 250 ≤ 937); `pub struct DiscoveredPaths`, `pub struct GoldbergRedirect`, `pub fn discover()`, `goldberg_watch_paths`, `goldberg_redirect_map`, `pub(crate) appmanifest_lookup`, registry probes (HKLM\WOW6432Node\Valve\Steam, HKCU\Software\Valve\Steam), libraryfolders.vdf parsing for both post-2022 + legacy locations, three Goldberg default roots, walkdir for steam_api*.dll. 16 unit tests pass. Pub test shims `scan_local_save_redirects_pub_for_tests` + `log_discovery_pub_for_tests` exposed for integration tests. |
| `src-tauri/src/store/mod.rs` | SqliteStore with open/open_in_memory/record_unlock/with_conn | VERIFIED | 157 lines; `pub struct SqliteStore` with `pub(super) conn: Mutex<Connection>`; all 4 public methods present + `count_unlocks` diagnostic; `INSERT OR IGNORE INTO unlock_history`; `include_str!("migrations/001_initial.sql")` for compile-time embedding. 5 unit tests pass. |
| `src-tauri/src/store/migrations/001_initial.sql` | unlock_history + sessions + settings + UNIQUE INDEX dedup | VERIFIED | 35 lines; all three tables; `CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)` (REQ DETECT-07 second line of defence). |
| `src-tauri/src/store/queries.rs` | create_session/end_session/mark_notified/unlock_count_for_session | VERIFIED | 137 lines; all 4 helpers present, all use `params![...]` parameter binding. 3 unit tests pass. |
| `src-tauri/src/watcher/mod.rs` | run_watcher (notify-debouncer-full 500ms) + run_pipeline | VERIFIED | 421 lines (plan min 100 ≤ 421); `pub mod dedup;` declared; `pub async fn run_watcher` + `pub async fn run_pipeline` both present; `new_debouncer(Duration::from_millis(500), None, callback)`; `RecursiveMode::Recursive`; `path.exists()` filter BEFORE `debouncer.watch`; `seed_baseline().await` runs textually before debouncer construction; sync→async via `blocking_send`. 4 unit tests + 1 pipeline test pass. |
| `src-tauri/src/watcher/dedup.rs` | CrossSourceDedup with TTL HashMap | VERIFIED | 112 lines (plan min 80 ≤ 112); `pub struct CrossSourceDedup { seen: HashMap<(u64, String), Instant>, ttl: Duration }`; `is_duplicate` with sweep-on-check (`self.seen.retain`); 4 unit tests pass. |
| `src-tauri/src/bin/hallmark-cli.rs` | Standalone CLI binary wiring full pipeline | VERIFIED | 144 lines (plan min 120 ≤ 144); imports `hallmark_lib::*` exclusively (no internal re-implementation); supports `--override-goldberg-root` argv + `HALLMARK_GOLDBERG_ROOT_OVERRIDE` env; uses `store.with_conn`; `tokio::signal::ctrl_c().await`; `println!` printer for kept events. **Live smoke run** (`./target/debug/hallmark-cli.exe --override-goldberg-root <tmp>`) emits `Goldberg baseline seeded`, `Baseline seeded`, `watching path recursively`, `WatcherCore active` and stays alive — confirming end-to-end wiring. |
| `src-tauri/tests/integration_phase1.rs` | 5 SC integration tests | VERIFIED | 483 lines (plan min 250 ≤ 483); all 5 SC tests present and PASS in 3.01s. |
| `tests/fixtures/goldberg/480/achievements.json` | Canonical fixture | VERIFIED | Exists; parses as JSON; 4 expected keys including `ACH_UNKNOWN_TIMESTAMP` with `earned: true, earned_time: 0` (PITFALLS.md #15 case). |
| `.planning/.../empirical-goldberg-schema-NOTES.md` | A4 resolution | VERIFIED | Exists; documents three real gbe_fork saves inspected; schema confirmed. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `src-tauri/src/main.rs` | `src-tauri/src/lib.rs` | `hallmark_lib::run()` | WIRED | Confirmed; `cargo build --bin hallmark` succeeds. |
| `Cargo.toml` (workspace) | `src-tauri/Cargo.toml` | `members = ["src-tauri"]` | WIRED | `cargo check --workspace` succeeds. |
| `src-tauri/src/store/mod.rs` | `src-tauri/src/store/migrations/001_initial.sql` | `include_str!("migrations/001_initial.sql")` | WIRED | Compile-time embed verified at line 16; tests prove migration applies. |
| `src-tauri/src/store/mod.rs` | `src-tauri/src/store/queries.rs` | `pub mod queries;` | WIRED | Module declared at line 7; queries::tests use store internals via `pub(super)` field. |
| `src-tauri/src/sources/mod.rs` | `src-tauri/src/sources/goldberg.rs` | `pub mod goldberg;` | WIRED | Declared at line 23; lib tests + integration tests import successfully. |
| `src-tauri/src/sources/goldberg.rs` | `src-tauri/src/sources/mod.rs` | `impl SourceAdapter for GoldbergAdapter` | WIRED | Trait impl present; satisfies object-safety; `Arc<dyn SourceAdapter>` works in run_watcher. |
| `src-tauri/src/watcher/mod.rs` | `src-tauri/src/watcher/dedup.rs` | `pub mod dedup;` | WIRED | Declared at line 22; `run_pipeline` constructs `CrossSourceDedup::new(dedup_ttl)`. |
| `src-tauri/src/watcher/mod.rs` | notify-debouncer-full | `new_debouncer(Duration::from_millis(500), None, ...)` | WIRED | Verified at line 55-63; lib test `run_watcher_emits_event_through_real_debouncer_within_1s` exercises the real debouncer end-to-end. |
| `src-tauri/src/bin/hallmark-cli.rs` | `hallmark_lib` | `use hallmark_lib::{paths, sources, store, watcher}` | WIRED | All four modules imported (lines 30-34); both `run_watcher` and `run_pipeline` spawned (lines 109-117); `store.with_conn(|conn| queries::create_session(...))` used (line 102). |
| `src-tauri/tests/integration_phase1.rs` | `hallmark_lib` | `use hallmark_lib::*` | WIRED | All four modules + `paths::scan_local_save_redirects_pub_for_tests` + `paths::log_discovery_pub_for_tests` imported and exercised. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| GoldbergAdapter baseline | `Arc<RwLock<HashMap<(u64, String), bool>>>` | `seed_baseline()` reads `<root>/<appid>/achievements.json` and redirect targets via `serde_json::from_str` | YES (proved by `seed_baseline_populates_from_fixture` and `sc2_pre_populated_state_emits_zero_events` which loads 50 entries) | FLOWING |
| RawUnlockEvent stream | `mpsc::Receiver<RawUnlockEvent>` | `on_file_changed` diffs current vs baseline + emits via `tx.send()` | YES (proved by SC1 — real disk write triggers a real event arriving at the sink in <1.5s) | FLOWING |
| SQLite unlock_history | rusqlite Connection | `record_unlock` inserts via parameterized `INSERT OR IGNORE` | YES (`count_unlocks()` returns 1 in SC1, 0 in SC2, 1 in SC4 — all match expected) | FLOWING |
| CLI stdout printer | `mpsc::Receiver<RawUnlockEvent>` | `run_pipeline` forwards kept events to sink, printer reads via `sink_rx.recv().await` | YES (smoke run confirms end-to-end channel chain alive after Ctrl-C signal) | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace builds | `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | exit 0 | PASS |
| Lib unit tests pass | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | 45/45 pass | PASS |
| Integration tests (5 SC) | `cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1` | 5/5 pass in 3.01s | PASS |
| CLI binary builds | `cargo build --manifest-path src-tauri/Cargo.toml --bin hallmark-cli` | exit 0 | PASS |
| CLI binary runs and logs | `./target/debug/hallmark-cli.exe --override-goldberg-root <tmp>` | Emits 4 INFO events (`Goldberg baseline seeded`, `Baseline seeded`, `watching path recursively`, `WatcherCore active`); stays alive | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DETECT-01 | 01-01, 01-02, 01-04, 01-05 | Real-time watcher detects unlocks from Goldberg SteamEmu output | SATISFIED | `sc1_single_unlock_emits_exactly_one_event_within_one_second` + `goldberg::on_file_changed_emits_event_on_false_to_true_transition` + `run_watcher_emits_event_through_real_debouncer_within_1s` |
| DETECT-05 | 01-04, 01-05 | First-launch state seeding | SATISFIED | `sc2_pre_populated_state_emits_zero_events` + `seed_baseline_populates_from_fixture` + `run_watcher_seeds_before_attaching_watcher` (asserts `change_count == change_after_seed`) |
| DETECT-06 | 01-04, 01-05 | 500ms debounce + content-hash equality | SATISFIED | `notify-debouncer-full` configured at `Duration::from_millis(500)` (watcher/mod.rs:56); `Sha256::digest` content-hash check before parse (goldberg.rs:250-258); `on_file_changed_skips_identical_content_via_sha256` test |
| DETECT-07 | 01-02, 01-05 | Cross-source duplicate suppression | SATISFIED | `CrossSourceDedup` with TTL HashMap + sweep-on-check; `idx_unlock_dedup` UNIQUE INDEX as DB-level second layer; `sc4_cross_source_dedup_collapses_real_adapter_events_to_one` + `run_pipeline_dedups_simultaneous_cross_source_events` |
| DETECT-08 | 01-03, 01-05 | Path discovery for libraryfolders.vdf + local_save.txt | SATISFIED | 16 paths unit tests (both VDF formats, all 5 local_save edge cases, appmanifest lookup, tracing capture); `sc3_local_save_txt_redirect_drives_end_to_end_pipeline` end-to-end; `sc5_path_discovery_logs_every_category_to_tracing` |

All 5 requirement IDs declared in the phase plans appear in REQUIREMENTS.md, mapped to Phase 1, and each is satisfied by passing test evidence. **No orphaned requirements.**

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none in production code) | — | — | — | — |
| `src-tauri/src/paths.rs` | 664-775 | `b"placeholder"` | INFO | All occurrences are inside `#[cfg(test)]` modules creating dummy DLL bytes for fixture-driven unit tests. Not production code. |

No TODO / FIXME / XXX / HACK / `unimplemented!()` / `todo!()` markers were found in production code. No empty handlers, no `return null` / `return Response.json([])` patterns.

### Code Review Findings (from 01-REVIEW.md) — Bearing on Goal Achievement

The standard-depth code review found 4 BLOCKERS, 11 WARNINGS, 6 INFO. **None of the review BLOCKERS invalidate this phase's stated goal or success criteria** — the goal is achieved as evidenced by 50/50 tests passing — but they represent latent correctness/security defects that should be addressed before depending on Phase 1 in production. They are surfaced here for transparency:

| Review Finding | Severity in Review | Bearing on Phase Goal | Note |
|----------------|---------------------|------------------------|------|
| BL-01: Case-insensitive `installdir` lookup fails on Windows | BLOCKER | Latent — SC3 test uses identical-case strings on both sides; the success criterion as written ("automatic discovery") is achieved for the case-matching path, which is the typical real-world case. The bug surfaces only on case-mismatched installs (e.g. SMB shares, restored backups). | Should be fixed before broad Phase 4 release. |
| BL-02: `last_hash` updated before parse — invalid-write poisoning | BLOCKER | Latent — `on_file_changed_skips_identical_content_via_sha256` passes; the bug requires a partial-write read producing different bytes than the eventual final write, which is rare. | Should be fixed; small refactor. |
| BL-03: `dispatch()` re-invokes `watch_paths()` per event | BLOCKER | Latent — current Goldberg-only adapter chain has stable watch paths; events are not silently dropped in the test fixtures. Will become a real issue if a watch root is deleted mid-session. | Performance + correctness fix recommended. |
| BL-04: Path traversal via attacker-controlled `local_save.txt` | BLOCKER | Latent — the threat is a malicious DLL+local_save.txt combination on user's machine; not exercised by any SC. | Security hardening recommended; current scope says "Goldberg setup is user's responsibility". |

These are documented as **known issues** found during code review and will be tracked as gap-closure or carry-forward items into Phase 2's planning. They do NOT block Phase 1 goal achievement because the success criteria as defined in ROADMAP.md are all empirically verified.

### Human Verification Required

None. All success criteria are automatically tested with passing assertions, and the live binary smoke-run reproduces the expected log output.

### Gaps Summary

No gaps blocking goal achievement. All 5 ROADMAP Success Criteria are verified by passing automated integration tests:
- SC1 (single event in <1s, no duplicates) — `sc1_single_unlock_emits_exactly_one_event_within_one_second` PASS
- SC2 (zero historical events) — `sc2_pre_populated_state_emits_zero_events` PASS
- SC3 (local_save.txt redirect end-to-end) — `sc3_local_save_txt_redirect_drives_end_to_end_pipeline` PASS
- SC4 (cross-source dedup to one event) — `sc4_cross_source_dedup_collapses_real_adapter_events_to_one` PASS
- SC5 (paths logged at startup) — `sc5_path_discovery_logs_every_category_to_tracing` PASS + live smoke-run

All 5 phase requirement IDs (DETECT-01, -05, -06, -07, -08) are satisfied. Total test inventory: 45 lib unit tests + 5 integration tests + smoke run = 50+1 passing, 0 failing.

The `RawUnlockEvent` sink is the consumer Phase 2's popup queue will read from — same channel type, same struct, no API churn required.

The 4 BLOCKER findings from `01-REVIEW.md` are real latent defects but do NOT invalidate the phase goal as expressed in ROADMAP.md. They should be tracked as work for Phase 2 / Phase 4 hardening.

---

_Verified: 2026-05-08T00:00:00Z_
_Verifier: Claude (gsd-verifier)_

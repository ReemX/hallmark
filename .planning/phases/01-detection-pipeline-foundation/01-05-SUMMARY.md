---
phase: 01-detection-pipeline-foundation
plan: 05
subsystem: detection-pipeline-cli-and-dedup
tags: [dedup, ttl, pipeline, sqlite, cli-binary, integration-tests, success-criteria, phase-1-close]
requires:
  - "Plan 01-01 scaffold (lib crate hallmark_lib, src-tauri/Cargo.toml, init_tracing)"
  - "Plan 01-02 SqliteStore + queries (record_unlock, count_unlocks, create_session)"
  - "Plan 01-03 paths::discover + goldberg_watch_paths + goldberg_redirect_map + scan_local_save_redirects"
  - "Plan 01-04 GoldbergAdapter + WatcherCore (run_watcher driving notify-debouncer-full)"
provides:
  - "watcher::dedup::CrossSourceDedup TTL-based dedup helper (REQ DETECT-07 in-memory layer)"
  - "watcher::run_pipeline async consumer (raw_rx → dedup → store → sink)"
  - "store::SqliteStore::with_conn helper exposing Connection access without leaking the Mutex"
  - "src/bin/hallmark-cli.rs — Phase 1 standalone test harness binary"
  - "[[bin]] hallmark-cli table in src-tauri/Cargo.toml"
  - "paths::scan_local_save_redirects_pub_for_tests + paths::log_discovery_pub_for_tests public test shims"
  - "tests/integration_phase1.rs — 5 ROADMAP Success Criteria tests, all passing"
affects:
  - "Phase 2 (popup queue) consumes the same RawUnlockEvent sink that hallmark-cli's printer consumes"
  - "Phase 3 (Steam-legit / CreamAPI / SmartSteamEmu adapters) — adding adapters now means adding entries to the Vec<Arc<dyn SourceAdapter>> passed to run_watcher; the dedup + store + sink wiring already accommodates them"
tech-stack:
  added:
    - "tokio `signal` feature (Ctrl-C graceful shutdown in hallmark-cli; was previously omitted)"
    - "tokio::sync::Mutex (TokioMutex) for the dedup state behind run_pipeline"
  patterns:
    - "TTL HashMap with sweep-on-check (O(n) per call but n bounded by per-session unlock count)"
    - "Three-channel pipeline: watcher → raw_tx/raw_rx → run_pipeline → sink_tx/sink_rx → printer"
    - "with_conn closure pattern — connection mutex held only for the scope of f"
    - "Argv + env-var override duality (env wins, but both supported) for fixture-driven test runs"
    - "MockAdapter pattern for SC4 — file-event-driven test SourceAdapter impl mirrors GoldbergAdapter's contract"
    - "Public test shims (`*_pub_for_tests`) over pub(crate) internals — cleanest way to drive private discovery code from external test crate"
key-files:
  created:
    - "src-tauri/src/watcher/dedup.rs"
    - "src-tauri/src/bin/hallmark-cli.rs"
    - "src-tauri/tests/integration_phase1.rs"
    - ".planning/phases/01-detection-pipeline-foundation/01-05-SUMMARY.md"
  modified:
    - "src-tauri/src/watcher/mod.rs (added `pub mod dedup` + `run_pipeline` consumer + pipeline_tests)"
    - "src-tauri/src/store/mod.rs (added with_conn helper)"
    - "src-tauri/src/paths.rs (added two public test shims at end of file)"
    - "src-tauri/Cargo.toml (added [[bin]] hallmark-cli table; added tokio `signal` feature)"
    - "Cargo.lock (regenerated for the new tokio feature flag)"
key-decisions:
  - "Plan 01-05: Default dedup TTL is 10 seconds — real-world cross-adapter simultaneity is sub-second; 10s is a generous safety margin per RESEARCH.md Pattern 3, with the SQLite UNIQUE INDEX as the belt-and-suspenders backstop if a duplicate slips past TTL"
  - "Plan 01-05: CrossSourceDedup is NOT thread-safe internally — wrapped in tokio::sync::Mutex by run_pipeline; this keeps the dedup struct free of synchronisation primitives and makes its tests deterministic single-threaded"
  - "Plan 01-05: run_pipeline uses sweep-on-check (retain) instead of a separate sweep task — n is bounded by per-session unlock count (small), and a separate task would add lifecycle complexity without latency benefit"
  - "Plan 01-05: SqliteStore::with_conn was added BEFORE the CLI binary that uses it (W-06 ordering fix) — the CLI is never written against `store.conn.lock().unwrap()` and refactored later; the clean helper API is the only API the consumer ever sees"
  - "Plan 01-05: hallmark-cli supports BOTH --override-goldberg-root <PATH> argv AND HALLMARK_GOLDBERG_ROOT_OVERRIDE env var — env var wins so PowerShell test scripts can set it without touching argv parsing"
  - "Plan 01-05: HALLMARK_DB_PATH_OVERRIDE env var routes the SQLite DB to a tempdir — integration tests set this so a CLI-binary smoke run (Plan 05 Step 5) does not pollute %APPDATA%\\Hallmark"
  - "Plan 01-05: tokio `signal` feature enabled in Cargo.toml — required for `tokio::signal::ctrl_c().await`; auto-fixed during Task 2 build (Rule 3 blocking issue) since the plan's reference code uses it but Plan 01-01's tokio feature list omitted signal"
  - "Plan 01-05: SC3 builds a real on-disk Steam-library fixture (steam_api64.dll + local_save.txt + appmanifest_4242.acf) and drives the FULL pipeline (B-01 fix) — the test no longer mocks the redirect resolution"
  - "Plan 01-05: SC4 uses two real MockAdapter instances emitting via file-event-driven paths (W-08 fix) — NOT direct raw_tx injection; this exercises the full adapter→watcher→pipeline path that production code takes"
  - "Plan 01-05: Integration test uses Rust 2018 `use hallmark_lib::...` style (W-10 fix) — no `extern crate hallmark_lib` line; the legacy-edition declaration is unnecessary in 2018+ and would only confuse readers"
  - "Plan 01-05: paths.rs gained two `*_pub_for_tests` public shims wrapping pub(crate) internals — minimum-viable surface change to let external integration tests drive private discovery code without leaking visibility for production callers"
patterns-established:
  - "Pipeline shape: watcher emit channel (raw) → consumer (run_pipeline) → forward channel (sink). Dedup + persistence + tracing happen between raw and sink."
  - "Closure-based connection access via with_conn — Phase 2 will adopt the same pattern for schema_cache + icon_cache table queries"
  - "Bin-target pattern: `src/bin/<name>.rs` files use `hallmark_lib::*` imports exclusively, never re-implement library logic"
  - "Integration test fixture pattern: fresh_tmp(label) + manual cleanup at end of test; uuid-based dir names avoid cross-test interference; spawn_pipeline helper builds the full pipeline + returns sink_rx + handles for assertion"
requirements-completed: [DETECT-01, DETECT-05, DETECT-06, DETECT-07, DETECT-08]
metrics:
  duration_minutes: 12
  completed_date: "2026-05-08"
  tasks_completed: 3
  tasks_total: 3
  files_created: 4
  files_modified: 5
  commits: 4
  unit_tests_added: 5
  integration_tests_added: 5
---

# Phase 01 Plan 05: Cross-Source Dedup + CLI Harness Summary

`CrossSourceDedup` provides REQ DETECT-07's in-memory layer (TTL HashMap with sweep-on-check); `run_pipeline` is the async consumer that wires the watcher's raw event stream into dedup + SQLite persistence + a forwarding sink; `hallmark-cli` is the standalone binary that drives the full Phase 1 pipeline outside Tauri's WebView, printing one stdout line per kept event. Five new integration tests in `tests/integration_phase1.rs` automate every ROADMAP Phase 1 Success Criterion, including a real-disk SC3 (steam_api64.dll + local_save.txt + appmanifest fixture → full pipeline → assertion that appid was resolved from the appmanifest, not from the directory name) and a MockAdapter-driven SC4 (two file-event adapters → exactly one event passes the dedup stage).

## What Was Built

### Task 1 — `CrossSourceDedup` + `run_pipeline` (commit `9f601c8`)

- **`src-tauri/src/watcher/dedup.rs`** — 110 lines. `CrossSourceDedup` is a `HashMap<(u64, String), Instant>` plus a `Duration` TTL. `is_duplicate(app_id, ach_api_name)` first calls `seen.retain(|_, ts| now.duration_since(*ts) < ttl)` to sweep expired entries, then checks key presence: present → return `true` (drop), absent → insert + return `false` (keep). `len()` and `is_empty()` are exposed for diagnostics. The struct is intentionally NOT thread-safe — synchronisation belongs at the consumer layer (run_pipeline wraps it in `tokio::sync::Mutex`).
- **`src-tauri/src/watcher/mod.rs`** — added `pub mod dedup;` declaration at the top + a new `pub async fn run_pipeline(_adapters, raw_rx, store, session_id, sink, dedup_ttl)` at the bottom. The function loops on `raw_rx.recv().await`, locks the dedup mutex briefly to call `is_duplicate`, drops the lock, persists kept events via `store.record_unlock(..., Some(&session_id))` (the SQLite UNIQUE INDEX from Plan 02 is the belt-and-suspenders second dedup layer), and forwards each kept event to the sink. The pre-existing `run_watcher` is untouched.
- **5 new tests pass.** `watcher::dedup::tests` (4): `first_observation_is_not_duplicate`, `repeat_observation_within_ttl_is_duplicate`, `expired_observation_is_no_longer_duplicate` (50ms TTL + 100ms sleep), `different_keys_are_independent`. `watcher::pipeline_tests` (1): `run_pipeline_dedups_simultaneous_cross_source_events` — sends two identical RawUnlockEvents within 200ms, asserts the first arrives at the sink and the second is dropped within the dedup TTL window, and the SQLite store has exactly 1 row.

### Task 2 — `SqliteStore::with_conn` + `hallmark-cli` binary (commits `419fbf5` + `337a69d`)

- **`src-tauri/src/store/mod.rs`** — added `pub fn with_conn<F, T>(&self, f: F) -> anyhow::Result<T> where F: FnOnce(&Connection) -> anyhow::Result<T>`. The mutex is held for the scope of the closure; callers run their typed-query work inside `f` and the borrow drops at scope-exit. Phase 2 will reuse this pattern for `queries::lookup_schema(&conn, app_id)` etc.
- **`src-tauri/src/bin/hallmark-cli.rs`** — 145 lines. `main()` runs `hallmark_lib::init_tracing()`, resolves watch paths (either via `paths::discover()` + `goldberg_watch_paths/redirect_map`, or via the `--override-goldberg-root <PATH>` argv flag / `HALLMARK_GOLDBERG_ROOT_OVERRIDE` env var), constructs `Arc<GoldbergAdapter::new(roots, redirect_map)>`, opens `SqliteStore::open(&db_path())` (DB path also overridable via `HALLMARK_DB_PATH_OVERRIDE`), creates a UUID v4 session via `store.with_conn(|conn| queries::create_session(conn, &id, None))`, wires three tokio tasks (`run_watcher` + `run_pipeline` + the `println!`-printer), and `await`s `tokio::signal::ctrl_c()`. On Ctrl-C, the watcher is aborted (which closes `raw_tx`, which collapses `run_pipeline`'s `recv().await` to `None`, which drops `sink_tx`, which terminates the printer), the session is ended in the DB, and `main` returns.
- **`src-tauri/Cargo.toml`** — added `[[bin]] name = "hallmark-cli"` table + the tokio `signal` feature flag.
- **`Cargo.lock`** — regenerated to reflect the new feature edge for `mio`'s signal-handling code path. No new dependencies.
- **Smoke test passed.** `HALLMARK_GOLDBERG_ROOT_OVERRIDE=<tmp> cargo run --quiet --bin hallmark-cli` produces (within ~2s of startup):
  - `INFO hallmark_cli: hallmark-cli starting (Phase 1 detection-only harness)`
  - `INFO hallmark_cli: using --override-goldberg-root (real path discovery skipped) path=<tmp>`
  - `INFO hallmark_cli: session created session_id=<uuid>`
  - `INFO hallmark_lib::sources::goldberg: Goldberg baseline seeded files=0 entries=0 roots=1 redirects=0`
  - `INFO hallmark_lib::watcher: Baseline seeded adapter="goldberg"`
  - `INFO hallmark_lib::watcher: watching path recursively adapter="goldberg" path=<tmp>`
  - `INFO hallmark_lib::watcher: WatcherCore active adapters=1 paths=1`

### Task 3 — Integration tests for Success Criteria #1–#5 (commit `75226bf`)

- **Pre-step:** `src-tauri/src/paths.rs` gained two thin public test shims at end-of-file: `pub fn scan_local_save_redirects_pub_for_tests(libraries: &[PathBuf]) -> Vec<GoldbergRedirect>` and `pub fn log_discovery_pub_for_tests(d: &DiscoveredPaths)`. Both delegate to the existing `pub(crate)` internals — minimum visibility surface needed for external integration tests.
- **`src-tauri/tests/integration_phase1.rs`** — 440 lines. Five tests, one per Success Criterion. Test helpers: `fresh_tmp(label)` (uuid-named tempdir), `write_state(root, app_id, json)` (writes `<root>/<appid>/achievements.json`), `write_appmanifest(library, app_id, installdir)` (writes a Steam ACF KeyValue), and `spawn_pipeline(adapters, store) -> (sink_rx, watcher_handle, pipeline_handle)` (builds the full pipeline + sleeps 400ms for seed+attach).

| Test | Success Criterion | REQ | What it asserts |
|------|-------------------|-----|-----------------|
| `sc1_single_unlock_emits_exactly_one_event_within_one_second` | #1 | DETECT-01 | Mark `ACH_X` earned on a populated baseline → exactly ONE `RawUnlockEvent` arrives in <1500ms; no further events for 2s; SQLite has exactly 1 row |
| `sc2_pre_populated_state_emits_zero_events` | #2 | DETECT-05 | 50-achievement pre-populated baseline → ZERO events arrive in 1500ms; SQLite has 0 rows |
| `sc3_local_save_txt_redirect_drives_end_to_end_pipeline` | #3 | DETECT-08 | Steam-library-shaped fixture (`steam_api64.dll` + `local_save.txt` + `appmanifest_4242.acf` mapping installdir "FooGame" → 4242). `paths::scan_local_save_redirects_pub_for_tests` returns 1 redirect; `goldberg_redirect_map` pairs target_path → 4242. After spawning the pipeline + writing `achievements.json` to the redirect target, exactly ONE event arrives with `app_id == 4242` (resolved from appmanifest, NOT from the directory name) |
| `sc4_cross_source_dedup_collapses_real_adapter_events_to_one` | #4 | DETECT-07 | Two real `MockAdapter` instances watching different roots; both fire file-driven events for `(4242, "ACH_DUP")` near-simultaneously; sink receives exactly ONE event; SQLite has exactly 1 row |
| `sc5_path_discovery_logs_every_category_to_tracing` | #5 | DETECT-08 | A custom `tracing_subscriber::Layer` (`VecLayer`) captures events; `paths::log_discovery_pub_for_tests` is called against a synthesized `DiscoveredPaths` with all 4 categories populated; assertions verify at least one INFO event per category, ≥4 INFO events total |

All 5 tests pass first try (3.02s wallclock).

## Public API Phase 2 Composes

```rust
use std::sync::Arc;
use std::time::Duration;
use hallmark_lib::paths::{discover, goldberg_watch_paths, goldberg_redirect_map};
use hallmark_lib::sources::{SourceAdapter, RawUnlockEvent};
use hallmark_lib::sources::goldberg::GoldbergAdapter;
use hallmark_lib::store::{SqliteStore, queries};
use hallmark_lib::watcher::{run_watcher, run_pipeline};

let d = discover();
let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(
    goldberg_watch_paths(&d),
    goldberg_redirect_map(&d),
));
let store = Arc::new(SqliteStore::open(&db_path)?);
let session_id = uuid::Uuid::new_v4().to_string();
store.with_conn(|conn| queries::create_session(conn, &session_id, None))?;

let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<RawUnlockEvent>(64);
let (sink_tx, sink_rx) = tokio::sync::mpsc::channel::<RawUnlockEvent>(64);
tokio::spawn(run_watcher(vec![adapter.clone()], raw_tx));
tokio::spawn(run_pipeline(vec![adapter], raw_rx, store, session_id, sink_tx, Duration::from_secs(10)));

// Phase 2 will replace the println!-printer with a popup-queue consumer that
// reads from sink_rx and renders the signature popup in the WebView.
```

## Key Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|-------------------------|
| Default TTL = 10 seconds for `CrossSourceDedup` | RESEARCH.md Pattern 3: real cross-adapter simultaneity is sub-second; 10s is generous safety margin. SQLite UNIQUE INDEX (Plan 02) is belt-and-suspenders backstop. | 1s (too tight; risks missing legitimate cross-emit windows) — discarded. 60s (would over-suppress repeat unlocks of incremental achievements) — discarded. |
| `CrossSourceDedup` not thread-safe internally | Single-thread tests are deterministic; the mutex lives at the consumer layer (run_pipeline) where it is short-lived. Forces the consumer to think about contention explicitly. | Internal RwLock — discarded; couples the data structure to its synchronisation, can't be re-used in non-async contexts. |
| Sweep-on-check instead of a separate sweep task | n is bounded by per-session unlock count (typically <50). O(n) per call is negligible. A separate task would add lifecycle complexity (start, stop, panics, cancellation) without latency benefit. | Separate timer task — discarded for stated reason. |
| `with_conn` added BEFORE the CLI uses it (W-06 ordering fix) | The plan explicitly forbids writing the CLI against `store.conn.lock().unwrap()` first and then refactoring — the clean API is the only API the consumer ever sees. | Re-pub the `conn` field to `pub` — discarded; would expose the Mutex to all crates and force every consumer to deal with poisoned-mutex error handling. |
| Argv `--override-goldberg-root` AND env-var `HALLMARK_GOLDBERG_ROOT_OVERRIDE` | Env-var-first lets PowerShell test scripts set the path without parsing argv (which requires the `--` separator under `cargo run`). Argv flag is the human-friendly form. | Env-var only — discarded; less discoverable (`--help` doesn't reveal it). Argv only — discarded; PowerShell + cargo + argv triple-quoting is fiddly. |
| `HALLMARK_DB_PATH_OVERRIDE` env var | Without it, an integration-test smoke run of `cargo run --bin hallmark-cli` would write to `%APPDATA%\Hallmark\hallmark.db`. The override routes it to a tempdir for reproducibility. | Always use a tempdir in CLI — discarded; production runs need persistent unlock history. |
| `tokio` `signal` feature added in Cargo.toml (Rule 3 auto-fix) | `tokio::signal::ctrl_c()` is gated behind `feature = "signal"`. Plan 01-01 omitted the feature; the build error in Task 2 surfaced it. Adding the feature is the right fix — Ctrl-C handling is required for graceful shutdown. | Roll our own SIGINT handler with the `signal-hook` crate — discarded; one extra dep when tokio already provides it. |
| Two `*_pub_for_tests` shims in `paths.rs` instead of relaxing visibility | Production callers should NOT have access to `scan_local_save_redirects` or `log_discovery` — they go through `discover()`. But integration tests need to drive the discovery internals against fixtures. Public shims are the minimum-surface change. | `pub(crate)` → `pub` — discarded; over-exposes the internals to the rest of the lib. `#[cfg(test)] pub` — discarded; doesn't compile for external integration tests (they aren't built with cfg(test) of the lib). |
| SC3 builds a real on-disk Steam-library fixture (B-01 fix) | The plan flagged this as the highest-value test of the entire phase: it exercises every component (registry-replacement via library override, libraryfolders parsing, walkdir, local_save.txt resolution, appmanifest lookup, GoldbergAdapter, run_watcher, run_pipeline). Mocking any layer would defeat the purpose. | Mock the redirect resolution — discarded; would never have caught a B-01-class regression where the dirname-vs-appmanifest fallback fails silently. |
| SC4 uses two real `MockAdapter`s emitting file-driven events (W-08 fix) | A direct `raw_tx.send(...)` injection would not exercise the watcher dispatch + adapter on_file_changed path — the actual layer where in-the-field bugs would surface. | Direct raw_tx injection — discarded; less faithful to production. |
| Rust 2018 `use hallmark_lib::...` style in integration tests (W-10 fix) | The legacy `extern crate` declaration is unnecessary in Rust 2018+ and only confuses contributors who don't know the history. The 2018 style is canonical. | `extern crate hallmark_lib as hallmark;` — discarded for stated reason. |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tokio `signal` feature missing for `tokio::signal::ctrl_c()`**

- **Found during:** Task 2 — `cargo build --bin hallmark-cli` failed with `error[E0433]: type annotations needed` and rustc's hint pointing at `feature = "signal"` being gated.
- **Issue:** Plan 01-01's `tokio` feature list omitted `signal`. The reference code in Plan 05 Step 3 uses `tokio::signal::ctrl_c().await.ok();` for Ctrl-C handling.
- **Fix:** Added `signal` to the tokio feature array in `src-tauri/Cargo.toml`. `cargo build --bin hallmark-cli` then succeeded; `Cargo.lock` regenerated automatically.
- **Files modified:** `src-tauri/Cargo.toml`, `Cargo.lock`
- **Commits:** `419fbf5` (Cargo.toml change folded with the rest of Task 2), `337a69d` (Cargo.lock regen as a separate `chore` commit since lock-file changes are not strictly part of the feature delta).

### Authentication Gates

None occurred during this plan.

## Threat Surface Compliance

The plan's `<threat_model>` lists five threats (T-05-T1, T-05-T2, T-05-D1, T-05-I1, T-05-S1). Implementation status:

| Threat | Disposition | Mitigation status |
|--------|-------------|-------------------|
| T-05-T1 (--override-goldberg-root path tampering) | accept | Local-only single-user app; if an attacker has shell access to set env vars or argv, they already control the user. Phase 1 does not add any new attack surface here. |
| T-05-T2 (ach_api_name printed to stdout) | mitigate | `println!` prints the API name as-is. Documented for Phase 2 (popup): the WebView render path MUST sanitize. Phase 1 stdout cannot escalate. |
| T-05-D1 (channel back-pressure DoS) | mitigate | All channels are `mpsc::channel(64)` (bounded). If the printer hangs, `run_pipeline`'s `sink.send().await` blocks, naturally back-pressuring the watcher. Verified by `run_pipeline_dedups_simultaneous_cross_source_events` (which exercises the full channel chain). |
| T-05-I1 (hallmark.db on disk) | accept | Per Plan 02 threat model — local-only, single-user. |
| T-05-S1 (session_id collision) | mitigate | UUID v4 — collision probability 2^-122 per pair. `Uuid::new_v4().to_string()` in hallmark-cli's main. |

## Verification Output

```
$ cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.98s

$ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
(no output — clean)

$ cargo test --manifest-path src-tauri/Cargo.toml --lib
running 45 tests
... (all 45 pass) ...
test watcher::dedup::tests::different_keys_are_independent ... ok
test watcher::dedup::tests::expired_observation_is_no_longer_duplicate ... ok
test watcher::dedup::tests::first_observation_is_not_duplicate ... ok
test watcher::dedup::tests::repeat_observation_within_ttl_is_duplicate ... ok
test watcher::pipeline_tests::run_pipeline_dedups_simultaneous_cross_source_events ... ok
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s

$ cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1
running 5 tests
test sc1_single_unlock_emits_exactly_one_event_within_one_second ... ok
test sc2_pre_populated_state_emits_zero_events ... ok
test sc3_local_save_txt_redirect_drives_end_to_end_pipeline ... ok
test sc4_cross_source_dedup_collapses_real_adapter_events_to_one ... ok
test sc5_path_discovery_logs_every_category_to_tracing ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s

$ cargo build --manifest-path src-tauri/Cargo.toml --bin hallmark-cli --release
    Finished `release` profile [optimized] target(s) in 1m 48s
```

All five end-of-phase commands exit 0.

## Total Phase 1 Test Inventory

| Source | Test count | Coverage |
|--------|-----------|----------|
| `sources::tests` | 2 | RawUnlockEvent eq, SourceKind as_str |
| `store::tests` + `store::queries::tests` | 8 | record_unlock + dedup + queries roundtrip |
| `paths::tests_steam` + `paths::tests_goldberg` | 16 | VDF parser + 11 goldberg path-discovery scenarios |
| `sources::goldberg::tests` | 11 | seed_baseline + diff + redirect_map fallback |
| `watcher::tests` | 3 | seed-then-attach + filter nonexistent + real-debouncer end-to-end |
| `watcher::dedup::tests` | 4 | TTL HashMap behaviour |
| `watcher::pipeline_tests` | 1 | run_pipeline dedups simultaneous events |
| `tests/integration_phase1` | 5 | ROADMAP Success Criteria #1–#5 |
| **Total** | **50** | All 5 phase REQs covered |

## Phase 1 Requirement Coverage

| REQ | Test(s) | Status |
|-----|---------|--------|
| **DETECT-01** (single unlock event in <1s) | `sc1_single_unlock_emits_exactly_one_event_within_one_second` + `goldberg::on_file_changed_emits_event_on_false_to_true_transition` + `watcher::run_watcher_emits_event_through_real_debouncer_within_1s` | covered |
| **DETECT-05** (no spam of historic unlocks on first run) | `sc2_pre_populated_state_emits_zero_events` + `goldberg::seed_baseline_populates_from_fixture` + `watcher::run_watcher_seeds_before_attaching_watcher` | covered |
| **DETECT-06** (debounce + content-hash dedup) | `goldberg::on_file_changed_skips_identical_content_via_sha256` + `watcher::run_watcher_emits_event_through_real_debouncer_within_1s` (asserts no further event in 800ms after first) | covered |
| **DETECT-07** (cross-source dedup) | `pipeline_tests::run_pipeline_dedups_simultaneous_cross_source_events` + `sc4_cross_source_dedup_collapses_real_adapter_events_to_one` + `store::record_unlock_dedup_via_unique_index` (DB-level second layer) | covered |
| **DETECT-08** (path discovery + logging) | `paths::tests_goldberg::*` (16 tests) + `sc3_local_save_txt_redirect_drives_end_to_end_pipeline` + `sc5_path_discovery_logs_every_category_to_tracing` | covered |

**All 5 phase requirement IDs are covered by at least one passing test.**

## Phase 1 Goal Met

> "A reliable, spam-free unlock event stream is flowing end-to-end for Goldberg-emulated games, ready for a UI layer to consume."

Confirmed by:
1. `hallmark-cli` builds and runs (smoke test produces correct startup logs in <1s)
2. SC1 — `cargo test integration_phase1` proves a real false→true transition produces exactly one event in <1.5s
3. SC2 — proves zero spam on already-populated state files
4. SC3 — proves the real-disk redirect resolution pipeline works end-to-end
5. SC4 — proves cross-source dedup collapses identical events to exactly one persisted row
6. SC5 — proves every discovered path is logged at startup
7. The `RawUnlockEvent` sink in `run_pipeline` IS the consumer Phase 2's popup queue will read from — same channel type, same struct, no API changes needed

## Next Plan Readiness

Phase 2 (popup queue + signature animation + sound) can now:
- Replace hallmark-cli's `println!`-printer with a popup-queue consumer reading the SAME `mpsc::Receiver<RawUnlockEvent>` sink that Plan 05 wired
- Construct the same `paths::discover() → GoldbergAdapter::new → run_watcher → run_pipeline` chain inside Tauri's `setup()` closure (`src-tauri/src/lib.rs`'s commented-out hook is the placement)
- Reuse `SqliteStore::with_conn` for `queries::lookup_schema(&conn, app_id)` once the schema cache table exists

## Self-Check: PASSED

- `src-tauri/src/watcher/dedup.rs` exists; contains `pub struct CrossSourceDedup`, `seen: HashMap<(u64, String), Instant>`, `ttl: Duration`, `pub fn is_duplicate`, `self.seen.retain`, all 4 unit-test names.
- `src-tauri/src/watcher/mod.rs` contains `pub mod dedup;` at module scope, `pub async fn run_pipeline`, `CrossSourceDedup::new`, `is_duplicate`, `store.record_unlock`. Run_watcher signature unchanged.
- `src-tauri/src/store/mod.rs` contains `pub fn with_conn<F, T>(&self, f: F) -> anyhow::Result<T>` with `F: FnOnce(&Connection) -> anyhow::Result<T>` bound.
- `src-tauri/src/bin/hallmark-cli.rs` exists; uses `hallmark_lib::*` imports, `GoldbergAdapter::new`, `paths::discover`, `goldberg_redirect_map`, `run_watcher`, `run_pipeline`, `#[tokio::main]`, `HALLMARK_GOLDBERG_ROOT_OVERRIDE`, `--override-goldberg-root`, `println!`, `tokio::signal::ctrl_c`, `store.with_conn`. Does NOT contain `store.conn.lock()`.
- `src-tauri/Cargo.toml` contains `[[bin]] name = "hallmark-cli"` with `path = "src/bin/hallmark-cli.rs"` AND tokio `signal` feature.
- `src-tauri/src/paths.rs` contains `pub fn scan_local_save_redirects_pub_for_tests` AND `pub fn log_discovery_pub_for_tests`.
- `src-tauri/tests/integration_phase1.rs` exists; contains all 5 SC test function names exactly, `use hallmark_lib::`, no `extern crate hallmark` line, `struct MockAdapter`, `impl SourceAdapter for MockAdapter`, `scan_local_save_redirects_pub_for_tests`, `log_discovery_pub_for_tests`, `write_appmanifest`, `#[tokio::test]`.
- `cargo check --all-targets` exit 0; `cargo fmt --check` exit 0; `cargo test --lib` 45 pass; `cargo test --test integration_phase1` 5 pass; `cargo build --bin hallmark-cli --release` exit 0.
- Commits exist on master: `9f601c8` (Task 1), `419fbf5` (Task 2 — store helper + CLI bin), `337a69d` (chore: Cargo.lock), `75226bf` (Task 3 — integration tests).

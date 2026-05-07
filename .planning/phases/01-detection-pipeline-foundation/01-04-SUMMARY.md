---
phase: 01-detection-pipeline-foundation
plan: 04
subsystem: detection-pipeline-watcher
tags: [goldberg, adapter, watcher, notify-debouncer-full, sha256, baseline, redirect-map]
requires:
  - "Plan 01-01 scaffold (Cargo workspace, src-tauri crate, sources/watcher stubs)"
  - "Plan 01-02 SourceAdapter trait + RawUnlockEvent + SourceKind"
  - "Plan 01-03 paths::goldberg_redirect_map + DiscoveredPaths"
  - "notify-debouncer-full 0.7 + sha2 0.11 + walkdir 2.5 + uuid 1.23 (test) pinned"
provides:
  - "GoldbergAdapter implementing SourceAdapter (parse, baseline, diff, emit, retry)"
  - "Two-pass seed_baseline: default-roots layout + redirect-target layout"
  - "extract_app_id with numeric-parse → redirect_map fallback chain"
  - "on_file_changed: filename guard + SHA-256 content dedup + sharing-violation retry"
  - "WatcherCore: pub async fn run_watcher(adapters, raw_tx) driving one shared notify-debouncer-full"
  - "Seed-then-attach ordering invariant enforced for REQ DETECT-05"
  - "500ms uniform debounce window for REQ DETECT-06 layer 1"
  - "11 GoldbergAdapter unit tests + 3 WatcherCore unit tests = 14 new tests"
affects:
  - "Plan 01-05 (CLI harness) imports `sources::goldberg::GoldbergAdapter` and `watcher::run_watcher`"
  - "Plan 01-05 spawns `tokio::spawn(watcher::run_watcher(...))` in the CLI test binary"
  - "Phase 3 plans add SteamLegit/CreamApi/SmartSteamEmu adapters by extending the Vec<Arc<dyn SourceAdapter>> passed to run_watcher (no WatcherCore changes needed)"
tech-stack:
  added:
    - "sha2::Sha256 active use (per-file content-hash dedup, REQ DETECT-06 layer 2)"
    - "notify-debouncer-full 0.7 active use (new_debouncer + DebounceEventResult)"
    - "tokio::sync::RwLock<HashMap<...>> for in-process baseline + last_hash"
    - "std::sync::atomic + AtomicU32 in test spies (call ordering assertions)"
  patterns:
    - "Two-pass seed: default roots first, then redirect-target keys (no double-walk)"
    - "filename guard FIRST in on_file_changed (cheapest filter, before parse/hash)"
    - "Read with retry on Windows ERROR_SHARING_VIOLATION (raw_os_error == 32) — 3 × 50ms"
    - "Single shared debouncer + sync→async bridge via blocking_send on the callback thread"
    - "Path prefix-match dispatch with first-match-wins (return after handler)"
    - "Adapters declare watch_paths(); WatcherCore filters non-existent BEFORE debouncer.watch (PathNotFound prevention)"
key-files:
  created:
    - "src-tauri/src/sources/goldberg.rs"
    - ".planning/phases/01-detection-pipeline-foundation/01-04-SUMMARY.md"
  modified:
    - "src-tauri/src/sources/mod.rs"
    - "src-tauri/src/watcher/mod.rs"
key-decisions:
  - "Plan 01-04: GoldbergAdapter::new takes (roots, redirect_map) — both required (Plan 05 passes HashMap::new() if no redirects); pre-pairing redirect_target → appid at discovery time avoids appmanifest concerns leaking into the adapter"
  - "Plan 01-04: filename guard `path.file_name() == Some(\"achievements.json\")` is the FIRST check in on_file_changed — recursive watches on parent dirs deliver sibling-file events that must be cheaply discarded before any parse/hash work"
  - "Plan 01-04: SHA-256 over the JSON bytes (not parsed state) — cheaper than parsing twice; identical-content writes (Steam's open-write-rename re-touches) are filtered without de-serializing"
  - "Plan 01-04: read_with_retry uses raw_os_error() == Some(32) (ERROR_SHARING_VIOLATION) AND ErrorKind::PermissionDenied — Windows can surface either kind during open-write-close races"
  - "Plan 01-04: `earned: bool` `false → true` transition is the ONLY unlock signal; `earned_time` exists in the struct for forward-compatibility but is never consulted (PITFALLS.md #15)"
  - "Plan 01-04: Diff order is read → hash → parse → diff → emit → THEN update baseline (under one write lock) — a panicking emit cannot leave the baseline ahead of what was emitted"
  - "Plan 01-04: WatcherCore uses ONE notify-debouncer-full driving all adapters (uniform 500ms across sources); the alternative (per-adapter debouncers) would fragment REQ DETECT-06"
  - "Plan 01-04: Seed-then-attach invariant: seed_baseline runs in a sequential for-loop BEFORE new_debouncer is constructed — textually + semantically guarantees no event can fire before baseline is set"
  - "Plan 01-04: prefix-match dispatch returns after first match — adapters MUST NOT have overlapping watch roots; documented inline as a Phase-3 forward concern"
  - "Plan 01-04: notify-debouncer-full's sync callback bridges to tokio mpsc via `blocking_send` — the callback thread is owned by the debouncer and is allowed to block"
patterns-established:
  - "Adapter shape: Arc<RwLock<...>> for shared mutable interior; #[async_trait::async_trait] for object-safe async; tokio::sync::mpsc for emit channel"
  - "Watcher shape: single shared debouncer instance; sync→async bridge via blocking_send; recv loop dispatches by prefix-match"
  - "Test pattern: fresh_tmp() with uuid + manual cleanup at end of each test (no global temp state); FIXTURE_BASELINE constant reused across tests for diff stability"
requirements-completed: [DETECT-01, DETECT-05, DETECT-06]
metrics:
  duration_minutes: 4
  completed_date: "2026-05-08"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 2
  commits: 2
  unit_tests_added: 14
---

# Phase 01 Plan 04: Goldberg Adapter + Watcher Core Summary

`GoldbergAdapter` parses Goldberg state files, seeds an in-memory baseline (default-root + redirect-target layouts), and emits exactly one `RawUnlockEvent` per `false → true` transition with SHA-256 content-hash dedup; `WatcherCore` runs ONE shared `notify-debouncer-full` at 500ms, seeds every adapter BEFORE attaching watchers (REQ DETECT-05), and dispatches debounced events to adapters via path prefix-match.

## What Was Built

- **`src-tauri/src/sources/goldberg.rs`** — 510 lines (including 11 tests). `GoldbergAdapter::new(roots, redirect_map)` constructs the adapter with two `Arc<RwLock<...>>` interiors: the `(appid, ach_api_name) → bool` baseline and the per-file `PathBuf → [u8; 32]` content-hash map. Five `SourceAdapter` methods: `name()` returns `"goldberg"`, `kind()` returns `SourceKind::Goldberg`, `watch_paths()` returns the union of existing default roots and existing redirect-target keys, `seed_baseline()` runs a two-pass walk (default-root layout via `walkdir::WalkDir::max_depth(2)` then redirect-target layout via direct `<key>/achievements.json` read), and `on_file_changed()` does filename guard → appid resolution → read-with-retry → SHA-256 dedup → parse → diff → emit → baseline update. The free `read_with_retry()` function retries 3 times at 50ms on `ErrorKind::PermissionDenied` or `raw_os_error() == Some(32)` (Windows `ERROR_SHARING_VIOLATION`, PITFALLS.md #3). The `GoldbergEntry` struct defines `earned: bool` plus `#[serde(default)] earned_time: u64` — the latter exists only to keep parsing tolerant; it is never consulted in unlock decisions (PITFALLS.md #15).

- **`src-tauri/src/sources/mod.rs`** — added a single line, `pub mod goldberg;`, to expose the adapter at module scope. Trait, event, and enum definitions from Plan 02 are unchanged.

- **`src-tauri/src/watcher/mod.rs`** — 290 lines (including 3 tests). `pub async fn run_watcher(adapters: Vec<Arc<dyn SourceAdapter>>, raw_tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>` enforces a textual three-phase order: (1) seed every adapter sequentially via `adapter.seed_baseline().await?`, (2) construct `new_debouncer(Duration::from_millis(500), None, callback)` and attach every adapter's `watch_paths()` recursively after `path.exists()` filtering, (3) recv-loop on the bridged `mpsc::Sender<DebounceEventResult>` and dispatch each event path via prefix-match in `dispatch()`. The sync callback uses `notify_tx.blocking_send(res)` — the debouncer owns its own thread, so blocking is the correct primitive for the sync→async bridge. The recv loop terminates when `notify_rx` closes (graceful shutdown via `raw_tx` drop on the receiver side, which collapses the bridge channel).

- **14 passing tests.** Goldberg (11): `seed_baseline_populates_from_fixture`, `on_file_changed_emits_event_on_false_to_true_transition`, `on_file_changed_no_event_for_already_earned_at_seed`, `on_file_changed_no_event_for_earned_time_zero_with_earned_true`, `on_file_changed_skips_identical_content_via_sha256`, `on_file_changed_no_event_when_filename_not_achievements_json`, `extract_app_id_returns_none_for_unknown_non_numeric_dir`, `extract_app_id_uses_redirect_map_for_non_numeric_parent`, `seed_baseline_reads_redirect_targets`, `on_file_changed_emits_event_via_redirect_map_lookup`, `integration_full_cycle_against_real_disk`. Watcher (3): `run_watcher_seeds_before_attaching_watcher`, `run_watcher_filters_nonexistent_paths`, `run_watcher_emits_event_through_real_debouncer_within_1s`.

## Public API Plan 05 Composes

```rust
use std::sync::Arc;
use std::collections::HashMap;
use hallmark_lib::paths::{discover, goldberg_watch_paths, goldberg_redirect_map};
use hallmark_lib::sources::{SourceAdapter, RawUnlockEvent};
use hallmark_lib::sources::goldberg::GoldbergAdapter;
use hallmark_lib::watcher::run_watcher;

let d = discover();
let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(
    goldberg_watch_paths(&d),
    goldberg_redirect_map(&d),
));
let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<RawUnlockEvent>(64);
tokio::spawn(run_watcher(vec![adapter], raw_tx));
// raw_rx is the single read-side; Plan 05 wires it into dedup + SQLite + stdout.
```

## Key Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|-------------------------|
| `GoldbergAdapter::new(roots, redirect_map)` takes both arguments unconditionally | Forces the call site to consciously construct the redirect map (or pass `HashMap::new()`); a one-arg constructor would invite silent omission of the redirect-target codepath. | Builder pattern — discarded as overengineering for two args. Default-empty redirect map — discarded for the same silent-omission reason. |
| Filename guard FIRST inside `on_file_changed` | Recursive watches deliver every file event under the watch root; sibling files (`cooldown.txt`, `crash.dmp`, etc.) must be discarded BEFORE the cost of read/hash/parse. | Filtering at the dispatcher — discarded; that would force the watcher to know each adapter's filename, leaking concerns. |
| SHA-256 over the JSON bytes (not parsed state) | Cheaper than parse-then-compare; identical re-writes (Steam open-write-rename retouches the same content) are filtered without de-serializing. | Hash the parsed `StateMap` — discarded; requires de-serializing once just to compute the hash, defeating the short-circuit. |
| `read_with_retry` keys on `raw_os_error() == 32` AND `ErrorKind::PermissionDenied` | Windows surfaces ERROR_SHARING_VIOLATION as either: a Rust `io::Error` whose `kind()` is `PermissionDenied`, OR an unmapped raw error 32. Both must be retried. | Retrying on every `io::Error` — discarded; would mask permanent permission issues with three 50ms sleeps. |
| `earned_time` parsed but never used in unlock logic | PITFALLS.md #15: Goldberg writes `earned_time = 0` for "earned but timestamp unknown". A naive `unlock_time > 0` filter would treat these as never-earned and re-fire after baseline. | Skip parsing `earned_time` — discarded; future plans (Phase 2 popup queue) may want to surface a "timestamp known" flag in the UI. Forward-compatible to keep the field. |
| Diff order: read → hash → parse → diff → emit → THEN update baseline (under ONE write-lock) | A panic during emit cannot leave the baseline ahead of what was emitted; on next event the same transition will re-fire (safer than a missed event). | Update baseline first — discarded; would silently swallow events on emit failure. |
| ONE shared `notify-debouncer-full` for all adapters | Uniform 500ms debounce across sources; one place to get the sync→async bridge right; avoids per-adapter buffer-size races on `ReadDirectoryChangesW`. | Per-adapter debouncers — discarded; fragments REQ DETECT-06 and triples the surface area for thread-safety bugs. |
| `path.exists()` filter BEFORE `debouncer.watch()` | `notify::Watcher::watch` errors `PathNotFound` for missing dirs (PITFALLS.md / RESEARCH.md Pitfall #5); filtering at the WatcherCore level is the only place that has the full path list. | Filter inside each adapter's `watch_paths()` — discarded; adapters already do filter, but the watcher must defend against late-deletion races AND adapter bugs. |
| Prefix-match dispatch returns after first match | Adapters MUST NOT have overlapping watch roots (Phase 3 will document this in adapter contracts); first-match-wins is the simplest enforceable rule. | Dispatch to all matching adapters — discarded; would let two adapters both emit for the same Goldberg event when overlapping roots are mistakenly registered. |
| Sync callback bridges via `blocking_send` (not `try_send`) | The debouncer owns its callback thread; blocking is allowed and back-pressures the source cleanly when the bridge is full. `try_send` would silently drop events under load. | `try_send` with logging — discarded; we want bounded latency under burst, not bounded throughput. |
| Test cleanup uses `let _ = fs::remove_dir_all(&dir)` (ignored result) at end | Tests use unique uuid-named tempdirs; cleanup-failure is non-fatal because the next test's tempdir is unique. Logging the error would drown valid output. | Use `tempfile` crate's `TempDir` — discarded; one extra dependency for what is two lines of cleanup. |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Cleanup] `cargo fmt` reformatted multi-arg assertions and `fs::write` chains**
- **Found during:** End of Task 1 and end of Task 2 (`cargo fmt --check`).
- **Issue:** A handful of `assert_ne!(a, b, "msg")`, `assert_eq!(a, b, "msg ...")`, and `fs::write(&path, "...long...").unwrap()` calls exceeded the 100-column line limit; rustfmt re-wraps them.
- **Fix:** Ran `cargo fmt --manifest-path src-tauri/Cargo.toml` once after each Task to apply the formatter. Diff is purely cosmetic (line wraps, no logic changes). Tests still pass identically.
- **Files modified:** `src-tauri/src/sources/goldberg.rs` (Task 1), `src-tauri/src/watcher/mod.rs` (Task 2)
- **Commits:** `6e9a901` (Task 1 — folded into the same commit as the source file), `9a2ce4f` (Task 2 — same).

### Authentication Gates

None occurred during this plan.

## Threat Surface Compliance

The plan's `<threat_model>` lists six threats. Implementation status:

| Threat | Disposition | Mitigation status |
|--------|-------------|-------------------|
| T-04-T1 (state-file JSON tampering) | mitigate | `serde_json::from_str` with strongly-typed `GoldbergEntry`. Parse failure logs warn + returns Ok (skip), never panics. Verified by parse-failure test path (`on_file_changed` `match parse_state` falls through to `Ok(())`). |
| T-04-D1 (event flood DoS) | mitigate | 500ms `notify-debouncer-full` (REQ DETECT-06 layer 1) + SHA-256 content-hash short-circuit (layer 2). Tested by `on_file_changed_skips_identical_content_via_sha256` and `run_watcher_emits_event_through_real_debouncer_within_1s` (assertion: no further events for 800ms after first event). |
| T-04-D2 (huge save dir DoS) | mitigate | `walkdir::WalkDir::new(root).max_depth(2)` bounds traversal. 1000 Goldberg games → 1000 file reads at startup, each ≤100KB; acceptable. |
| T-04-T2 (redirect_map keys tampering) | accept | Plan 03 already validates redirect targets exist + appmanifest matches; Plan 04 only consumes the validated map. We never write to or execute from these paths — only read state files and watch them. |
| T-04-I1 (path strings in tracing logs) | accept | Logs show full paths including username. Local stdout only; no telemetry. |
| T-04-S1 (adapter prefix-match collision) | mitigate | First-prefix-match-wins with explicit `return` in `dispatch()`. Phase 1 has only one adapter, so this matters as Phase 3 forward-thought; documented inline in `dispatch()` comment. |

## Verification Output

```
$ cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.89s

$ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
(no output — clean)

$ cargo test --manifest-path src-tauri/Cargo.toml --lib
running 40 tests
... (all 40 pass) ...
test watcher::tests::run_watcher_filters_nonexistent_paths ... ok
test watcher::tests::run_watcher_seeds_before_attaching_watcher ... ok
test watcher::tests::run_watcher_emits_event_through_real_debouncer_within_1s ... ok

test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
```

All three verification commands exit 0. Total tests: 2 (sources base) + 8 (store) + 16 (paths) + 11 (goldberg) + 3 (watcher) = 40 — matches the plan's prediction (~40).

## Next Plan Readiness

Plan 05 (`01-05-cross-source-dedup-and-cli-harness`) can now:
- Import `hallmark_lib::sources::goldberg::GoldbergAdapter` and call `Arc::new(GoldbergAdapter::new(roots, redirect_map))`.
- Import `hallmark_lib::watcher::run_watcher` and `tokio::spawn` it from the CLI test binary.
- Compose `paths::discover() → goldberg_watch_paths/redirect_map → adapter → run_watcher → mpsc::Receiver<RawUnlockEvent>` into the full Phase 1 pipeline.
- Add the `[[bin]] name = "hallmark-cli"` Cargo entry once `bin/hallmark-cli.rs` exists.

REQs DETECT-01, DETECT-05, DETECT-06 are fully covered. Success Criterion #1 (one event within 1s, no duplicates within 5s) is asserted in `run_watcher_emits_event_through_real_debouncer_within_1s` (1500ms event window + 800ms duplicate-free window).

## Self-Check: PASSED

- `src-tauri/src/sources/goldberg.rs` exists.
- `src-tauri/src/sources/mod.rs` declares `pub mod goldberg;` at module scope.
- `goldberg.rs` contains `pub struct GoldbergAdapter`, `redirect_map: HashMap<PathBuf, u64>`, `impl SourceAdapter for GoldbergAdapter`, `Sha256::digest`, `async fn seed_baseline`, `async fn on_file_changed`, `#[serde(default)]`, `earned: bool`, `fn read_with_retry`, `raw_os_error() == Some(32)`, `walkdir::WalkDir`, all three redirect-related test names.
- `src-tauri/src/watcher/mod.rs` contains `pub async fn run_watcher`, `new_debouncer(`, `Duration::from_millis(500)`, `RecursiveMode::Recursive`, `seed_baseline().await`, `blocking_send`, `Arc<dyn SourceAdapter>`, `path.exists()`, `starts_with`. The seeding for-loop appears textually before the `new_debouncer` call.
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` returns exit 0.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` returns exit 0 (clean).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` returns exit 0 with all 40 tests passing — including 11 new `sources::goldberg::tests` and 3 new `watcher::tests`.
- Commits exist on master: `6e9a901` (Task 1 — GoldbergAdapter), `9a2ce4f` (Task 2 — WatcherCore) — verified via `git log --oneline`.

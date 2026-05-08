---
phase: 02-premium-ui-popup-companion-game-session
plan: "07"
subsystem: integration-wiring
tags: [integration, lib-setup, tauri-builder, e2e-tests, popup-queue, game-detect]

# Dependency graph
requires:
  - plan: 02-01
    provides: "lib.rs module ladder, Tauri scaffold, stub modules"
  - plan: 02-02
    provides: "SchemaCache::new, list_for_app, lookup — schema resolution chain"
  - plan: 02-03
    provides: "game_detect::run, monitor::*, game-started event with pid (B-1 fix)"
  - plan: 02-04
    provides: "AudioDispatcher::new, Tier enum, play()"
  - plan: 02-05
    provides: "ui::create_popup_window, ui::create_companion_window, popup_queue::run"
  - plan: 02-06
    provides: "AppState struct, CompanionState, 3 Tauri commands in pub mod commands"
provides:
  - "lib.rs::run() fully wired: setup() with all Phase 2 spawns + window builds + AppState + invoke_handler"
  - "4 tokio tasks: run_watcher, run_pipeline, popup_queue::run (or sink-drainer fallback), game_detect::run"
  - "game-started listener writing pid into current_pid mutex (POPUP-03 functional monitor routing)"
  - "session_id created at startup via uuid v4 + queries::create_session"
  - "B-3 fix: paths::goldberg_watch_paths + paths::goldberg_redirect_map used (no fictional DiscoveredPaths fields)"
  - "Integration test popup_pipeline_e2e.rs: 2 tests (POPUP-01 latency + POPUP-02 dedup)"
  - "Integration test companion_lifecycle.rs: 3 tests (COMP-03 persist + schema cache + D-11 completion flag)"
affects:
  - Phase 3 (Steam-legit adapter wires into same run() topology)
  - Phase 4 (signature SFX + real icon assets replace placeholders)

# Tech tracking
tech-stack:
  added:
    - "tauri::{Listener, Manager} imports in lib.rs (Rule 1: required for app.listen + app.manage)"
    - "tauri::Event type annotation on game-started closure parameter (Rule 1: type inference requires explicit annotation)"
  patterns:
    - "B-3 fix: bind goldberg_watch_paths + goldberg_redirect_map BEFORE consuming discovery into closures"
    - "B-1 consumption: game-started listener reads payload.pid, writes into shared Arc<TokioMutex<Option<u32>>>"
    - "Audio best-effort: AudioDispatcher::new() failure → warn + sink-drainer, NOT popup_queue skip"
    - "sink-drainer fallback: tokio::spawn drains sink_rx when audio unavailable, prevents run_pipeline backpressure"
    - "generate_handler! uses crate::commands:: prefix for commands in pub mod commands sub-module"

key-files:
  created:
    - src-tauri/tests/popup_pipeline_e2e.rs
    - src-tauri/tests/companion_lifecycle.rs
  modified:
    - src-tauri/src/lib.rs (pub fn run() fully wired with all Phase 2 subsystems)

key-decisions:
  - "tauri::{Listener, Manager} imports added — Tauri 2.x traits must be explicitly imported (not re-exported from tauri root at call site)"
  - "tauri::Event type annotation required on listen closure because serde_json::from_str inference needs the concrete Event type"
  - "game-started listener uses app.listen (not app_handle.listen) — app: &mut tauri::App implements Listener, not AppHandle inside setup()"
  - "sink-drainer fallback preserves Phase 1 run_pipeline backpressure-free operation when audio unavailable"
  - "generate_handler! references commands::get_companion_state etc. (not bare get_companion_state) due to pub mod commands wrapping from Plan 06"

# Metrics
duration: ~8 min
completed: 2026-05-08T11:38:50Z
---

# Phase 2 Plan 07: Integration Wiring Summary

**Full lib.rs::run()::setup() wired — 4 tokio task spawns, game-started PID listener, AppState management, Tauri command registration, B-3 fix using real DiscoveredPaths helpers; 5 integration tests pass (popup pipeline e2e + companion lifecycle)**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-08T11:30:00Z
- **Completed:** 2026-05-08T11:38:50Z
- **Tasks:** 2 automated completed + 1 checkpoint awaiting manual verification
- **Files modified:** 3 (1 modified, 2 created)

## Accomplishments

### Task 1: Wire lib.rs::run()::setup()

Replaced the Phase 1 empty `setup()` closure with the fully wired Phase 2 implementation:

1. **SQLite store** — opened at `dirs::data_dir()/Hallmark/hallmark.db` with 001+002 migrations applied automatically
2. **Session** — uuid v4 session_id created via `queries::create_session` at startup
3. **Path discovery** — `paths::discover()` returns canonical `DiscoveredPaths`
4. **B-3 fix** — bound `goldberg_watch_paths(&discovery)` and `goldberg_redirect_map(&discovery)` as local variables BEFORE any move-into-closure; the prior plan referenced non-existent `.goldberg_roots` and `.redirect_map` fields on `DiscoveredPaths`
5. **GoldbergAdapter** built from goldberg_paths + goldberg_map; single-element adapters Vec
6. **Channels** — `raw_tx/rx` (watcher → pipeline) + `sink_tx/rx` (pipeline → popup_queue), capacity 64 each
7. **AudioDispatcher** — `AudioDispatcher::new()` best-effort; failure logs warn and falls through to sink-drainer fallback (popups are visual-only, not silenced entirely)
8. **SchemaCache** — constructed once; cloned into AppState + popup_queue + game-started listener
9. **Windows** — `ui::create_popup_window` + `ui::create_companion_window` called; both start hidden
10. **AppState** — `app.manage(AppState { store, schema, session_id })` registered for Tauri command access
11. **current_pid** — `Arc<TokioMutex<Option<u32>>>` shared between popup_queue and game-started listener
12. **4 tokio spawns** — run_watcher, run_pipeline, popup_queue::run (or sink-drainer), game_detect::run; each logged
13. **game-started listener** — parses JSON payload, extracts `app_id` AND `pid` (Plan 03 B-1 fix); writes pid into current_pid mutex; spawns schema::resolve for this app_id

#### Tauri Commands Registered

```rust
.invoke_handler(tauri::generate_handler![
    commands::get_companion_state,
    commands::set_companion_prefs_cmd,
    commands::get_companion_prefs_cmd,
])
```

### Task 2: Integration Tests

**`src-tauri/tests/popup_pipeline_e2e.rs`** (POPUP-01, POPUP-02):
- `raw_unlock_event_arrives_at_sink_within_1s` — in-memory store + synthetic RawUnlockEvent → run_pipeline → sink arrival within 1 second timeout + persisted in unlock_history
- `duplicate_unlocks_dedup_at_sink` — 3 identical events → only 1 arrives at sink within 1s; no further within 200ms

**`src-tauri/tests/companion_lifecycle.rs`** (COMP-01, COMP-03, POPUP-05/D-11):
- `schema_cache_populates_after_resolve` — upsert_schema rows + list_for_app returns correct schema list
- `earned_unlock_history_persists_session` — record_unlock persists across SchemaCache reconstruction (COMP-03 mid-restart restore)
- `completion_flag_persists_once_per_app` — mark_completion_fired idempotent; per-app-id independent (D-11)

All 5 tests pass. Full workspace test suite green (5 Phase 1 SC tests + 5 Plan 07 tests + all unit tests).

## Cross-Task PID Hand-Off (B-1 → POPUP-03 Functional Routing)

The game-started listener in setup() now implements the full POPUP-03 chain:

```
game_detect::run  →  Tauri emit("game-started", {app_id, pid})
                             ↓
    game-started listener in setup() parses payload.pid
                             ↓
    tokio::spawn writes pid → Arc<TokioMutex<Option<u32>>> (current_pid)
                             ↓
    popup_queue::run reads current_pid on every fire
                             ↓
    monitor::hwnd_for_pid(pid) → monitor::popup_position(rect, monitor_rect)
                             ↓
    app_handle.emit("popup-show", payload) + window.set_position(pos)
```

Previously (Plans 03, 05) the helpers existed but the wiring across tasks was absent. Plan 07 closes the seam.

## Setup() Task Graph

```
SqliteStore::open()
    ↓
queries::create_session (session_id: uuid v4)
    ↓
paths::discover() → steam_libraries + goldberg_watch_paths + goldberg_redirect_map (B-3)
    ↓
GoldbergAdapter::new(goldberg_paths, goldberg_map)
    ↓
mpsc channels: raw (64) + sink (64)
    ↓
AudioDispatcher::new() — best-effort [warn + fallback if Err]
    ↓
SchemaCache::new(store)
    ↓
ui::create_popup_window + ui::create_companion_window (both hidden)
    ↓
app.manage(AppState { store, schema, session_id })
    ↓
Arc<TokioMutex<Option<u32>>> current_pid
    ↓
tokio::spawn run_watcher(adapters, raw_tx)
tokio::spawn run_pipeline(raw_rx, store, session_id, sink_tx, 10s)
tokio::spawn popup_queue::run(app, sink_rx, schema, audio, store, session_id, pid)
    OR  drain-sink fallback (if audio unavailable)
tokio::spawn game_detect::run(app, store, steam_libraries, goldberg_map)
    ↓
app.listen("game-started") → write pid + spawn schema::resolve
```

## Manual Smoke-Test Instructions (ROADMAP Success Criteria)

The following require manual execution with a real Goldberg-emulated game:

**Success Criterion #1 (popup within 1s):**
1. `cargo tauri dev`
2. Drop an updated `achievements.json` into `%APPDATA%\GSE Saves\<appid>\`
3. Expect popup in top-right of game's monitor within 1 second

**Success Criterion #3 (companion auto-show + restart restore):**
1. Launch a Goldberg-emulated game → companion window should appear showing achievement list
2. Close and relaunch app → companion should re-show with previously earned items still checked

**Success Criterion #4 / POPUP-04 (DPI + rarity — Task 3 / W-10 checklist):**
See Task 3 checkpoint below — requires 4K + 1080p multi-monitor hardware verification.

**Success Criterion #5 (schema cached before first popup):**
- Schema::resolve fires on game-started before the first file-watcher event can arrive
- Verified at component level: schema_cache_populates_after_resolve test

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing tauri::{Listener, Manager} imports**
- **Found during:** Task 1 build
- **Issue:** `app.manage()` requires `tauri::Manager` in scope; `app.listen()` requires `tauri::Listener` in scope. Tauri 2.x traits are not re-exported from the tauri root without explicit import.
- **Fix:** Added `use tauri::{Listener, Manager};` to lib.rs imports.
- **Files modified:** `src-tauri/src/lib.rs`
- **Commit:** 0911788

**2. [Rule 1 - Bug] Missing type annotation on game-started closure parameter**
- **Found during:** Task 1 build (E0282 type annotations needed)
- **Issue:** `app.listen("game-started", move |event| { ... })` — Rust cannot infer the closure parameter type because `serde_json::from_str(event.payload())` needs `Event` to be known.
- **Fix:** Added explicit annotation: `move |event: tauri::Event|`
- **Files modified:** `src-tauri/src/lib.rs`
- **Commit:** 0911788

## Task 3 Checkpoint (W-10 / POPUP-04)

Task 3 is a `checkpoint:human-verify` requiring manual DPI + multi-monitor verification:
- Launch `cargo tauri dev` with a Goldberg game on 1080p monitor → popup on correct monitor, crisp text
- Move game to 4K monitor → popup follows, scales crisply
- Trigger 6+ burst unlocks on 4K → all popups fire, adaptive compression visible

This is deferred to manual smoke-test step. Results should be documented in the Phase 2 closing run.

## Integration Test Coverage Map

| Test | ROADMAP Criterion | Verified Level |
|------|------------------|----------------|
| raw_unlock_event_arrives_at_sink_within_1s | #1 (popup within 1s) | Sink boundary (pipeline layer) |
| duplicate_unlocks_dedup_at_sink | POPUP-02 (dedup adjacent) | Sink boundary |
| schema_cache_populates_after_resolve | #5 (schema cached) | SQLite layer |
| earned_unlock_history_persists_session | #3 (COMP-03 restore) | SQLite layer |
| completion_flag_persists_once_per_app | POPUP-05/D-11 (once per app) | SQLite layer |
| Manual smoke (Task 3) | #1, #3, #4 full round-trip | WebView + hardware |

## Deferred Phase 3 Items

- **Steam-state-authoritative leg of D-21** — game_detect currently uses sysinfo polling only. Phase 3 will add localconfig.vdf binary VDF parsing for the Steam-IPC authoritative leg.
- **Full Tauri runtime integration tests** — testing the webview round-trip (popup-show emission → React AnimatePresence → CSS animation visible in WebView) requires a real Tauri test harness. Deferred to Phase 4 acceptance testing.
- **Steam binary VDF (UserGameStats_*.bin) unlock detection** — Phase 3 adapter.

## Known Stubs

None — all Phase 2 subsystems are fully wired. The only remaining stub is the Phase 4 signature SFX (currently placeholder WAV synthesized in Plan 04) and real icon assets.

## Threat Surface Scan

No new threat surface beyond what the plan's `<threat_model>` documents:
- T-02-41: Task panic doesn't propagate (each spawn is fire-and-forget; tokio absorbs panics)
- T-02-42: Audio failure does NOT block popups (sink-drainer fallback)
- T-02-46: game-started payload malformed JSON → tracing::warn + return, no panic
- T-02-47: current_pid race (popup positions on previous game's monitor for one fire; self-corrects)

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Wire lib.rs::run()::setup() — full Phase 2 task graph | 0911788 |
| 2 | Integration tests — popup_pipeline_e2e + companion_lifecycle | 172f52b |

## Self-Check: PASSED

Files verified present:
- src-tauri/src/lib.rs (pub fn run() with full setup): FOUND
- src-tauri/tests/popup_pipeline_e2e.rs: FOUND
- src-tauri/tests/companion_lifecycle.rs: FOUND

Commits verified:
- 0911788 (Task 1): FOUND
- 172f52b (Task 2): FOUND

Builds verified:
- cargo build -p hallmark: PASS
- cargo build --release -p hallmark: PASS
- cargo test --workspace: PASS (all tests green)

Acceptance criteria verified:
- invoke_handler(tauri::generate_handler!): PASS
- get_companion_state referenced: PASS
- app.manage(AppState): PASS
- ui::create_popup_window: PASS
- ui::create_companion_window: PASS
- audio::AudioDispatcher::new: PASS
- schema::SchemaCache::new: PASS
- tokio::spawn(watcher::run_watcher): PASS
- watcher::run_pipeline: PASS
- popup_queue::run: PASS
- game_detect::run: PASS
- app.listen("game-started"): PASS
- paths::goldberg_watch_paths (B-3): PASS
- paths::goldberg_redirect_map (B-3): PASS
- no discovery.goldberg_roots (B-3 fictional field gone): PASS
- no discovery.redirect_map (B-3 fictional field gone): PASS
- discovery.steam_libraries (real field): PASS
- payload.get("pid") (B-1 consumption): PASS
- current_pid write Some(pid): PASS
- queries::create_session: PASS

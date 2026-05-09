---
phase: 04-polish-distribution
plan: "03"
subsystem: testing
tags: [test-popup, portable-mode, schema-fixture, mpsc, sqlite, canonicalize]

# Dependency graph
requires:
  - phase: 04-01a
    provides: portable_mode.rs stub + test_trigger.rs stub + constants TEST_API_NAME / TEST_APP_ID
  - phase: 04-01b
    provides: AppState.raw_tx Sender + SqliteStore in AppState + seed_test_fixture call site in lib.rs::run()
  - phase: 04-02
    provides: tray.rs fire_test handler that calls test_trigger::fire()
  - phase: 02-01
    provides: schema_cache table + upsert_schema / get_schema_row helpers + SchemaCacheRow
provides:
  - is_portable() — exe-parent vs %LOCALAPPDATA%\Hallmark canonicalize compare; safest-default=false (installed)
  - is_portable_with() — pure testable helper for portable detection logic
  - test_trigger::fire() — blocking_send synthetic RawUnlockEvent via AppState.raw_tx (D-04)
  - test_trigger::seed_test_fixture() — idempotent INSERT OR REPLACE of fixture row at (480, HALLMARK_TEST_UNLOCK)
affects:
  - 04-04 (updater_glue gates spawn_background_check on is_portable())
  - 04-06 (distribution — portable detection affects update skip logic)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure helper pattern: is_portable_with(exe_parent, installed) separates env coupling from logic for testability"
    - "Safest-default canonicalize: when either path fails to canonicalize, return false (assumed installed) to preserve updater behavior"
    - "blocking_send over try_send in sync tray handlers: backpressure wait, not silent drop"
    - "Schema fixture pre-seed at startup: INSERT OR REPLACE idempotence allows repeated calls without error"

key-files:
  created: []
  modified:
    - src-tauri/src/portable_mode.rs
    - src-tauri/src/test_trigger.rs

key-decisions:
  - "Portable default=false when canonicalize fails: test/dev portable scenario still runs updater (fails gracefully) rather than silently disabling it on real installs"
  - "blocking_send in fire(): sync tray handler must wait for backpressure, not silently drop event (try_send would make user think their click did nothing)"
  - "SourceKind::Goldberg for synthetic event: CrossSourceDedup keys on (app_id, ach_api_name) not source — any variant produces correct dedup; Goldberg is consistent with RESEARCH Pattern 3"
  - "global_pct=None for fixture: classify_tier routes to Tier::Standard; rare/completion tier would be misleading for a test popup"
  - "seed_test_fixture takes &SqliteStore (not &AppHandle): pure + testable with in-memory store; lib.rs already has &store at call point"

patterns-established:
  - "Exe-path portable detection: current_exe().parent() vs dirs::data_local_dir().join(AppName) with canonicalize; failure-to-detect defaults to false"
  - "Test popup injection point: raw_tx Sender clone at adapter→dedup boundary, not at file-watcher level"

requirements-completed:
  - POL-01

# Metrics
duration: 12min
completed: 2026-05-09
---

# Phase 4 Plan 03: Test Trigger + Portable Mode Summary

**Synthetic RawUnlockEvent injector at adapter→dedup boundary + exe-path portable detection against %LOCALAPPDATA%\Hallmark via canonicalize compare**

## Performance

- **Duration:** 12 min
- **Started:** 2026-05-09T15:51:07Z
- **Completed:** 2026-05-09T16:03:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `is_portable()` implemented: compares canonical form of running exe's parent dir against `%LOCALAPPDATA%\Hallmark`. When either path fails to canonicalize (typical for dev builds where `%LOCALAPPDATA%\Hallmark` doesn't exist), defaults to false (assumed installed) — the safer outcome. Emits `tracing::info!` with `portable=true|false` + both paths once at startup.
- `test_trigger::fire()` implemented: clones `AppState.raw_tx` and pushes a synthetic `RawUnlockEvent { app_id: 480, ach_api_name: "HALLMARK_TEST_UNLOCK", source: Goldberg }` via `blocking_send`. The event flows through the entire real production pipeline: CrossSourceDedup → SchemaCache::lookup (finds pre-seeded fixture) → AudioDispatcher → popup_queue → monitor placement → popup window animation.
- `test_trigger::seed_test_fixture()` implemented: idempotent `INSERT OR REPLACE` into `schema_cache` at PK `(480, "HALLMARK_TEST_UNLOCK")` with `display_name="Test Achievement"`, `description="Hallmark is working correctly on your system."`, `global_pct=None` (Tier::Standard). Pre-seeded at startup so subsequent test fires hit a warm cache without a Web API roundtrip (D-05).
- 7 unit tests across both files, all passing. Full lib test suite (144 tests) passes with no regressions.

## Test Fixture Row Details

| Field | Value |
|-------|-------|
| app_id | 480 (Spacewar — Steam test app) |
| ach_api_name | `HALLMARK_TEST_UNLOCK` |
| display_name | `Test Achievement` |
| description | `Hallmark is working correctly on your system.` |
| icon_path | `None` (popup falls back to bundled placeholder) |
| hidden | `false` |
| global_pct | `None` — routes to `Tier::Standard` via `classify_tier` |

## Pipeline Stage NOT Exercised by Test Trigger

The file-watcher kernel callback chain (ReadDirectoryChangesW → notify-debouncer-full → adapter.on_file_changed) is intentionally NOT exercised by the test trigger. Rationale:

1. Real game unlocks already validate this path end-to-end.
2. Synthesizing file writes for a test popup would be slow (disk I/O, debounce wait) and path-fragile (must know a valid game save directory for the test to work).
3. The injection point (raw_tx Sender, adapter→dedup boundary) still exercises every production stage after the file watcher, which is where the rendering, audio, and dedup logic lives.

This is documented as D-04 in CONTEXT.md and RESEARCH.md Pattern 3.

## Plan 04-04 Dependency

`is_portable()` is the gate for `updater_glue::spawn_background_check`. Plan 04-04 calls `is_portable()` from `AppState.portable_mode` (already wired in Plan 04-01b) and skips the background update check when the result is `true`. This prevents update nag dialogs for users running from an extracted .zip or USB drive.

## Task Commits

Each task was committed atomically:

1. **Task 1: portable_mode.rs — exe-path heuristic vs %LOCALAPPDATA%\Hallmark** - `d8c2aec` (feat)
2. **Task 2: test_trigger.rs — synthetic RawUnlockEvent injector + schema fixture** - `287a0d2` (feat)

## Files Created/Modified

- `src-tauri/src/portable_mode.rs` — Full `is_portable()` implementation with `is_portable_with()` pure helper; preserves `is_silent_launch()` from Plan 04-01
- `src-tauri/src/test_trigger.rs` — Full `fire()` and `seed_test_fixture()` implementations with 3 unit tests

## Decisions Made

- **Portable default=false when canonicalize fails:** the installed dir won't exist on a fresh extracted .zip — canonicalize fails — so defaulting to false (assumed installed) means the updater runs and fails gracefully. The opposite risk (silently disabling updates on a real installed copy) is worse.
- **blocking_send in fire():** sync tray handler must wait for backpressure rather than silently drop. `try_send` would silently discard the event when the channel buffer is momentarily full, making the user think their click did nothing.
- **SourceKind::Goldberg for synthetic event:** CrossSourceDedup keys on `(app_id, ach_api_name)`, not source. Any variant produces correct dedup. Goldberg chosen per RESEARCH Pattern 3 line 514.
- **global_pct=None for fixture:** routes `classify_tier` to `Tier::Standard`. Using a rarity value (e.g., `Some(0.1)`) would make the test popup appear as a "rare achievement" popup, which is misleading.

## Deviations from Plan

None - plan executed exactly as written.

The plan provided complete implementation code in `<action>` blocks. Minor deviation: removed the unused `PathBuf` import from `portable_mode.rs` that was included in the plan's code sample (compiler warning Rule 1 auto-fix — trivial, no behavior change).

## Issues Encountered

- Minor: the plan's `portable_mode.rs` action block included `use std::path::{Path, PathBuf}` but `PathBuf` was unused. Removed to eliminate compiler warning (Rule 1 auto-fix, one-line change).

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced beyond what the plan's `<threat_model>` covers:
- T-04-11 (seed overwrites real schema): mitigated — `HALLMARK_TEST_UNLOCK` sentinel, PK granularity test passes.
- T-04-12 (path logging): accepted — logs user's own exe path, no sensitive data.
- T-04-13 (rapid clicks): accepted — CrossSourceDedup 10s TTL governs.
- T-04-14 (--silent argv): accepted — user controls own HKCU\Run.

## Known Stubs

None — both `is_portable()` and `test_trigger::fire()` / `seed_test_fixture()` are fully implemented. No `STUB` strings remain in either file.

## Next Phase Readiness

- Plan 04-04 (updater_glue): `is_portable()` is ready. `AppState.portable_mode` already set in lib.rs::run() from Plan 04-01b.
- Click "Fire test popup" in tray during `cargo tauri dev` to smoke-test the full pipeline end-to-end (manual step; not in automated CI).
- D-06 verified by design: clicking twice within 10 seconds correctly suppresses the second popup (CrossSourceDedup TTL governs — production behavior).

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*

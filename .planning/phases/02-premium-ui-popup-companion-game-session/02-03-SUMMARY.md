---
phase: 02-premium-ui-popup-companion-game-session
plan: "03"
subsystem: game-detection + monitor-placement
tags: [sysinfo, win32, hwnd, monitor, steam-state, game-detect]

# Dependency graph
requires:
  - phase: 02-premium-ui-popup-companion-game-session
    plan: "01"
    provides: "stub files for monitor.rs, game_detect/ submodules, paths.rs appmanifest_lookup"
provides:
  - "monitor.rs: hwnd_for_pid + monitor_rect_for_hwnd + popup_position (Win32 cfg-gated)"
  - "game_detect::run: long-running tokio task polling sysinfo every 3s; emits game-started + game-stopped"
  - "game-started payload includes app_id + pid (B-1 fix for POPUP-03 functional routing)"
  - "steam_state::parse_loginusers + current_steam_user (loginusers.vdf parser)"
  - "process_scan::scan_running_games + refresh_and_scan (sysinfo + appmanifest matching)"
  - "paths.rs::appmanifest_lookup visibility relaxed to pub for cross-module consumption"
affects:
  - 02-05 (popup_queue calls hwnd_for_pid + monitor_rect_for_hwnd + popup_position for POPUP-03)
  - 02-06 (companion listens for game-started/game-stopped events to show/hide)
  - 02-07 (registers game-started listener to populate current_pid mutex used by popup_queue)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Win32 cfg-gate pattern: dual #[cfg(target_os = 'windows')] / #[cfg(not(...))] arms mirroring paths.rs::read_steam_install"
    - "EnumWindows callback with non-panicking ops only (Pitfall 12 — panic across FFI is UB)"
    - "tokio::time::interval(3s) long-running poll loop — mirrors watcher::run_watcher shape"
    - "prev_running HashMap<app_id, pid> diff pattern for game-start/stop edge detection"
    - "VDF parsing via keyvalues_parser — same pattern as paths.rs::parse_libraryfolders_text"
    - "BL-01: appmanifest installdir lowercased on both insert and lookup side for case-insensitive matching"

key-files:
  created: []
  modified:
    - src-tauri/src/monitor.rs
    - src-tauri/src/game_detect/mod.rs
    - src-tauri/src/game_detect/process_scan.rs
    - src-tauri/src/game_detect/steam_state.rs
    - src-tauri/src/paths.rs

key-decisions:
  - "sysinfo 0.38 API: refresh_processes requires 2 args (bool remove_dead_processes); plan specified 0.39 single-arg API — fixed inline (Rule 1)"
  - "match_steam_library lowercases installdir path component before map lookup to match appmanifest_lookup BL-01 convention"
  - "D-21 Steam-state-authoritative leg deferred to Phase 3 per CONTEXT.md; inline comment documents the deferral"
  - "D-22 conflict-resolution hook present as placeholder comment for Phase 3 binary VDF Steam-legit adapter"
  - "game-started payload carries both app_id AND pid (B-1 fix) so Plan 07 can populate current_pid mutex for POPUP-03 functional routing"
  - "_store param reserved in game_detect::run for Plan 07 session wiring without re-plumbing"

# Metrics
duration: ~15min
completed: 2026-05-08
---

# Phase 2 Plan 03: Game Detection + Monitor Placement Summary

**Win32 HWND lookup + multi-monitor placement helpers + hybrid sysinfo/Steam game-detect tokio task with game-started/stopped Tauri events carrying app_id and pid for POPUP-03 functional monitor routing**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-08
- **Completed:** 2026-05-08
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Implemented `monitor.rs` with Win32 `EnumWindows`-based HWND lookup for a PID, `GetMonitorInfoW`-based rcWork rect (excludes taskbar), and a pure-math `popup_position()` with top-right anchor at 25% from top and 32px margin per D-01; all Win32 calls cfg-gated; 4 unit tests verify 1080p, 4K secondary, negative-x secondary, and the 32px margin invariant
- Relaxed `paths.rs::appmanifest_lookup` from `pub(crate)` to `pub` so `process_scan.rs` can import it across modules
- Implemented `steam_state.rs` with `parse_loginusers` (VDF parser matching `parse_libraryfolders_text` pattern) and `current_steam_user`; 3 unit tests cover two-account parse, malformed VDF, and empty users block
- Implemented `process_scan.rs` with `scan_running_games` (two-leg detection: steamapps installdir match + Goldberg redirect root match) and `refresh_and_scan` convenience wrapper; 3 unit tests including a real filesystem fixture verifying appmanifest ACF round-trip
- Implemented `game_detect::mod.rs` orchestrator as long-running tokio task at 3s interval; diffs prev/curr HashMap<app_id, pid>; emits `game-started` (with app_id + pid) and `game-stopped` events; D-22 conflict-resolution hook placeholder and D-21 deferral comment; 5 unit tests cover diff logic + struct shape + B-1 payload regression

## Task Commits

Each task was committed atomically:

1. **Task 1: monitor.rs Win32 HWND-by-PID + multi-monitor placement** - `a9fe31d` (feat)
2. **Task 2: steam_state.rs + process_scan.rs + paths.rs visibility** - `65d79b1` (feat)
3. **Task 3: game_detect orchestrator mod.rs** - `65bbbae` (feat)

## Files Created/Modified

- `src-tauri/src/monitor.rs` — replaced stub with Win32 HWND + monitor placement impl (125 lines)
- `src-tauri/src/game_detect/mod.rs` — replaced stub with tokio poll task + diff + emit (166 lines)
- `src-tauri/src/game_detect/steam_state.rs` — replaced stub with loginusers.vdf parser (107 lines)
- `src-tauri/src/game_detect/process_scan.rs` — replaced stub with sysinfo + appmanifest scan (162 lines)
- `src-tauri/src/paths.rs` — `appmanifest_lookup` visibility relaxed pub(crate) → pub (1 line change + comment)

## Decisions Made

- sysinfo 0.38 is in the project (plan specified 0.39); `refresh_processes` takes an extra `bool remove_dead_processes` argument in 0.38 — fixed inline as Rule 1 auto-fix
- `match_steam_library` lowercases installdir from the exe path component to match `appmanifest_lookup`'s BL-01 lowercase-keys convention; plan's original test used "MyGame" (would fail) — corrected
- D-21 Steam-state-authoritative leg (binary VDF localconfig.vdf parsing) deferred to Phase 3 per CONTEXT.md; code has placeholder comment at the hook point
- `_store` parameter accepted-but-unused in `game_detect::run` so Plan 07 can wire SQLite session updates without changing the function signature

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] sysinfo 0.38 API: refresh_processes requires 2 arguments**
- **Found during:** Task 2 (process_scan.rs)
- **Issue:** Plan specified sysinfo 0.39 API (`refresh_processes(ProcessesToUpdate::All)` — 1 arg). Project has sysinfo 0.38 which requires a second `bool` parameter (`remove_dead_processes`).
- **Fix:** Changed to `sys.refresh_processes(ProcessesToUpdate::All, true)` in `process_scan.rs`
- **Files modified:** `src-tauri/src/game_detect/process_scan.rs`
- **Commit:** `65d79b1`

**2. [Rule 1 - Bug] match_steam_library test used mixed-case installdir key not matching BL-01 lowercase**
- **Found during:** Task 2 code review
- **Issue:** Plan's test used `map.insert("MyGame".to_string(), 480_u64)` but `appmanifest_lookup` inserts lowercase keys per BL-01; lookup would return None.
- **Fix:** Changed map key to `"mygame"` in test and lowercased the installdir component in `match_steam_library` before lookup.
- **Files modified:** `src-tauri/src/game_detect/process_scan.rs`
- **Commit:** `65d79b1`

## Known Stubs

None — all stubs from Plan 01 assigned to this plan have been replaced with full implementations.

## Threat Surface

No new network endpoints, auth paths, or schema changes introduced. The Win32 surfaces and process enumeration patterns are exactly as described in the plan's threat model (T-02-17 through T-02-22b).

## Self-Check: PASSED

Files verified present:
- src-tauri/src/monitor.rs: FOUND
- src-tauri/src/game_detect/mod.rs: FOUND
- src-tauri/src/game_detect/process_scan.rs: FOUND
- src-tauri/src/game_detect/steam_state.rs: FOUND

Commits verified:
- a9fe31d (Task 1): FOUND
- 65d79b1 (Task 2): FOUND
- 65bbbae (Task 3): FOUND

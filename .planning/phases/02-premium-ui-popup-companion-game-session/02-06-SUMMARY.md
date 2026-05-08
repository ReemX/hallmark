---
phase: 02-premium-ui-popup-companion-game-session
plan: "06"
subsystem: companion-ui
tags: [companion, react, framer-motion, sqlite-prefs, tauri-commands, useGameSession]

# Dependency graph
requires:
  - plan: 02-01
    provides: "lib.rs module ladder, React/Vite scaffold, companion.css stub, main-companion.tsx stub"
  - plan: 02-02
    provides: "SchemaCache::list_for_app, AchievementSchema struct, schema_cache SQLite table"
  - plan: 02-03
    provides: "game-started/game-stopped events, game_detect task"
  - plan: 02-05
    provides: "ui::create_companion_window (window created hidden)"
provides:
  - "AppState struct (store + schema + session_id) wired for Plan 07 manage(state)"
  - "CompanionState struct (app_id, schema, earned map, session_id) — Serialize+Deserialize"
  - "3 Tauri commands: get_companion_state, set_companion_prefs_cmd, get_companion_prefs_cmd"
  - "useGameSession hook: game-started/stopped/schema-resolved event listeners"
  - "7 React components: CompanionHeader, FilterBar, SortToggle, AchievementRow, SkeletonRow, EmptyState + full CompanionRoot in main-companion.tsx"
  - "companion.css: full UI-SPEC.md token set (#111114 bg, #1A2030 earned, cyan accent)"
affects:
  - 02-07 (wires AppState via manage(state) + registers handlers in tauri::generate_handler!)

# Tech tracking
tech-stack:
  added:
    - "pub mod commands inline module in lib.rs — avoids tauri::command proc-macro name collision in crate root"
  patterns:
    - "Tauri commands in sub-module (pub mod commands) to avoid proc-macro __cmd__ conflicts at crate root"
    - "useDebouncedPersist: 500ms window.setTimeout + cleanup return for D-18 prefs persistence"
    - "D-20 race: skeleton rows shown immediately; schema-resolved event triggers get_companion_state refetch for in-place upgrade"
    - "D-17 show/hide: useEffect on appId null-check, getCurrentWebviewWindow().show()/hide()"
    - "AchievementRow uses Framer Motion layout animation; SkeletonRow uses pure CSS @keyframes"
    - "convertFileSrc() for local icon paths; pass-through for http:// Steam CDN URLs"

key-files:
  created:
    - src/hooks/useGameSession.ts
    - src/components/CompanionHeader.tsx
    - src/components/FilterBar.tsx
    - src/components/SortToggle.tsx
    - src/components/AchievementRow.tsx
    - src/components/SkeletonRow.tsx
    - src/components/EmptyState.tsx
  modified:
    - src-tauri/src/lib.rs (AppState + CompanionState + 3 Tauri commands in pub mod commands)
    - src-tauri/src/store/queries.rs (CompanionPrefs: added serde::Deserialize)
    - src-tauri/src/schema/mod.rs (AchievementSchema: added serde::Deserialize)
    - src/main-companion.tsx (replaced Plan 01 stub with full CompanionRoot)
    - src/styles/companion.css (replaced Plan 01 stub with full UI-SPEC.md styles)

key-decisions:
  - "Commands wrapped in pub mod commands to avoid Tauri proc-macro __cmd__ name collision in lib.rs crate root — Plan 07 uses crate::commands::get_companion_state etc. in tauri::generate_handler!"
  - "AchievementSchema gained serde::Deserialize (needed by CompanionState Deserialize derive — CompanionState.schema is Vec<AchievementSchema>)"
  - "useDebouncedPersist uses window.setTimeout cleanup pattern (not useRef) to stay React hook-rules compliant"

# Metrics
duration: ~4 min
completed: 2026-05-08
---

# Phase 2 Plan 06: Companion Window UI Summary

**3 Tauri commands (AppState, get_companion_state, set_companion_prefs_cmd, get_companion_prefs_cmd) + 7 React components (CompanionHeader/FilterBar/SortToggle/AchievementRow/SkeletonRow/EmptyState) + useGameSession hook + full UI-SPEC.md companion CSS — D-17/D-18/D-20 interaction patterns implemented**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-08
- **Completed:** 2026-05-08
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- Added `pub mod commands` block to lib.rs containing `AppState` struct (store + schema + session_id Arc fields for Plan 07 `app.manage(state)`) and `CompanionState` serde struct
- Implemented 3 Tauri command handlers: `get_companion_state` (schema list + earned map SQL query), `set_companion_prefs_cmd` (writes companion_prefs row), `get_companion_prefs_cmd` (reads back)
- Replaced Plan 01 stub `main-companion.tsx` with full `CompanionRoot`: D-17 auto-show/hide on game-started/stopped, D-18 500ms debounced prefs persistence, D-20 skeleton + schema-resolved in-place upgrade
- Created `useGameSession` hook that subscribes to `game-started`, `game-stopped`, `schema-resolved` Tauri events and exposes `appId` + `resolveStage`
- Created 6 React components per UI-SPEC.md: `CompanionHeader` (data-tauri-drag-region, custom close), `FilterBar` (role=radiogroup with 3 radio chips), `SortToggle` (earned-first / A–Z), `AchievementRow` (Framer Motion layout, 4px cyan earned stripe, 36px circular icon, tap-to-expand), `SkeletonRow` (CSS @keyframes pulse), `EmptyState` (4 copy variants per UI-SPEC.md Copywriting Contract)
- Replaced stub `companion.css` with full UI-SPEC.md tokens: `#111114` background, `#1A2030` earned rows, `rgba(120,220,255,0.85)` accent, skeleton animation, typography hierarchy

## Tauri Command Signatures

```rust
// In lib.rs pub mod commands — Plan 07 registers via tauri::generate_handler!
pub fn get_companion_state(app_id: u64, state: State<'_, AppState>) -> Result<CompanionState, String>
pub fn set_companion_prefs_cmd(prefs: CompanionPrefs, state: State<'_, AppState>) -> Result<(), String>
pub fn get_companion_prefs_cmd(app_id: u64, state: State<'_, AppState>) -> Result<Option<CompanionPrefs>, String>
```

**Plan 07 wire-up:** Add `app.manage(AppState { store, schema, session_id })` and `.invoke_handler(tauri::generate_handler![crate::commands::get_companion_state, crate::commands::set_companion_prefs_cmd, crate::commands::get_companion_prefs_cmd])`

## D-15 / D-18 Persistence Flow

1. On game-started: `invoke('get_companion_prefs_cmd', { app_id })` fetches saved prefs; defaults to `filter:'all', sort:'earned-first'` if `null`
2. On filter/sort/expand change: React `setPrefs()` triggers re-render; `useDebouncedPersist` fires `invoke('set_companion_prefs_cmd', { prefs })` after 500ms of idle
3. Window size/position persistence (D-15): Plan 07 wires Tauri window resize/move events to update `prefs.width/height/pos_x/pos_y` fields (CompanionPrefs already has columns)

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | AppState + 3 Tauri commands (lib.rs) + CompanionPrefs serde | 4f464ff |
| 2 | 7 React components + useGameSession hook + companion.css | d848e99 |

## Files Created/Modified

- `src-tauri/src/lib.rs` — `pub mod commands` block: AppState, CompanionState, 3 `#[tauri::command]` handlers; `pub use commands::{AppState, CompanionState}` re-export
- `src-tauri/src/store/queries.rs` — CompanionPrefs: added `serde::Serialize, serde::Deserialize` derives
- `src-tauri/src/schema/mod.rs` — AchievementSchema: added `serde::Deserialize` derive
- `src/main-companion.tsx` — full CompanionRoot: D-17/D-18/D-20 logic + filter/sort/expand
- `src/styles/companion.css` — full UI-SPEC.md token CSS
- `src/hooks/useGameSession.ts` — game-started/stopped/schema-resolved listener
- `src/components/CompanionHeader.tsx` — drag region + custom close button
- `src/components/FilterBar.tsx` — 3-chip radio filter
- `src/components/SortToggle.tsx` — 2-state sort toggle
- `src/components/AchievementRow.tsx` — earned/locked row with Framer Motion layout
- `src/components/SkeletonRow.tsx` — pulsing CSS placeholder
- `src/components/EmptyState.tsx` — 4-variant empty state copy

## Decisions Made

- Tauri commands wrapped in `pub mod commands` to avoid proc-macro `__cmd__` name collision at crate root. Plan 07 references them as `crate::commands::get_companion_state` etc.
- `AchievementSchema` needed `serde::Deserialize` because `CompanionState` derives `Deserialize` and contains `Vec<AchievementSchema>`; added as Rule 1 auto-fix.
- `useMemo` import removed from main-companion.tsx (was in plan template, not actually used — TypeScript strict mode caught it).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Commands wrapped in pub mod commands submodule**
- **Found during:** Task 1 build
- **Issue:** `#[tauri::command]` proc-macro generates `__cmd__get_companion_state` macros that conflict when the function is in the crate root (`lib.rs`). Rust reports E0255 "name defined multiple times" for all 3 commands.
- **Fix:** Wrapped the 3 commands, AppState, and CompanionState in `pub mod commands { ... }` inline block; added `pub use commands::{AppState, CompanionState}` for convenience. Plan 07 uses `crate::commands::get_companion_state` in `tauri::generate_handler!`.
- **Files modified:** `src-tauri/src/lib.rs`
- **Impact:** `pub mod` count is 11 (was expected 10). All 10 Plan 01 modules preserved.

**2. [Rule 1 - Bug] AchievementSchema required serde::Deserialize**
- **Found during:** Task 1 build
- **Issue:** `CompanionState` derives `Deserialize` and holds `Vec<AchievementSchema>`, but `AchievementSchema` only derived `Serialize`. Build error E0277.
- **Fix:** Added `serde::Deserialize` to `AchievementSchema` derive list.
- **Files modified:** `src-tauri/src/schema/mod.rs`

**3. [Rule 1 - Bug] Removed unused useMemo import**
- **Found during:** Task 2 pnpm build
- **Issue:** Plan template included `useMemo` in imports but CompanionRoot doesn't use it. TypeScript strict mode (TS6133) fails build.
- **Fix:** Removed `useMemo` from import statement.
- **Files modified:** `src/main-companion.tsx`

## Known Stubs

None — all companion files are fully implemented per plan specification.

## Threat Surface

No new threat surface beyond what Plan 06's `<threat_model>` documents:
- T-02-35: Achievement display_name rendered via React `{title}` — auto-escaped. No XSS risk.
- T-02-36: Filter/sort values TypeScript-constrained; no SQL injection risk in companion_prefs.
- T-02-39: companion.json capability grants only `core:window:allow-close + allow-hide + allow-show + allow-set-size + allow-set-position + allow-start-dragging + allow-minimize`.

## Self-Check: PASSED

Files verified present:
- src/hooks/useGameSession.ts: FOUND
- src/components/AchievementRow.tsx: FOUND
- src/components/FilterBar.tsx: FOUND
- src/components/SortToggle.tsx: FOUND
- src/components/CompanionHeader.tsx: FOUND
- src/components/SkeletonRow.tsx: FOUND
- src/components/EmptyState.tsx: FOUND
- src/main-companion.tsx (useGameSession): FOUND
- src/styles/companion.css (#111114): FOUND
- src-tauri/src/lib.rs (AppState + 3 commands): FOUND

Commits verified:
- 4f464ff (Task 1): FOUND
- d848e99 (Task 2): FOUND

Builds verified:
- cargo build -p hallmark: PASS
- pnpm build: PASS

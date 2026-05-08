---
phase: 02-premium-ui-popup-companion-game-session
plan: "01"
subsystem: scaffolding
tags: [tauri, vite, react, sqlite, rodio, sysinfo, reqwest, windows-rs, framer-motion, migration]

# Dependency graph
requires:
  - phase: 01-detection-pipeline-foundation
    provides: "SqliteStore with migration loader, lib.rs module ladder, Cargo.toml foundation, Tauri builder skeleton"
provides:
  - "Rust deps added: rodio 0.22 (audio), sysinfo 0.39 (process scan), reqwest 0.13 (HTTP), windows-rs 0.58 (Win32)"
  - "React 19 + Vite 6 + Framer Motion 12 frontend scaffold with two webview entry points (popup.html + index.html)"
  - "SQLite migration 002_schema_cache.sql: schema_cache + companion_prefs tables"
  - "tauri.conf.json with CSP locked to Steam CDN + API, devUrl pinned to localhost:1420"
  - "capabilities/popup.json (event-listen only) + capabilities/companion.json (event + window control)"
  - "lib.rs declares all 6 Phase 2 module stubs: schema, audio, monitor, popup_queue, ui, game_detect"
  - "14 stub Rust files for Wave 2 plans to populate without lib.rs conflicts"
  - "TypeScript types (PopupPayload, AchievementSchema, Tier, GameStartedPayload)"
affects:
  - 02-02 (schema resolution chain — uses schema/ stub files)
  - 02-03 (game detection — uses game_detect/ + monitor/ stubs)
  - 02-04 (audio — uses audio.rs stub)
  - 02-05 (popup UI — uses popup_queue.rs + ui.rs stubs + popup.html entry)
  - 02-06 (companion UI — uses index.html entry + companion capability)

# Tech tracking
tech-stack:
  added:
    - "rodio 0.22 (wav feature) — WAV/OGG one-shot SFX via WASAPI"
    - "sysinfo 0.39 — EnumProcesses-based process scanner"
    - "reqwest 0.13 (json + rustls-tls, no default-features) — public Steam Web API calls"
    - "windows-rs 0.58 (Win32_Foundation, Win32_UI_WindowsAndMessaging, Win32_Graphics_Gdi) — HWND manipulation"
    - "React 19 + react-dom 19 — frontend framework"
    - "Vite 6 — bundler with multi-entry rollup config"
    - "framer-motion 12 — PS5-style popup animation"
    - "@tauri-apps/api ^2 — frontend IPC"
    - "@vitejs/plugin-react ^4 — React transform for Vite"
    - "TypeScript 5 — frontend type checking"
  patterns:
    - "Vite multi-entry build: rollupOptions.input maps companion→index.html and popup→popup.html"
    - "Two SQLite migrations applied in sequence via include_str! constants in SqliteStore::open()"
    - "Stub-first module pattern: lib.rs declares all modules upfront; owning plans fill bodies in Wave 2"
    - "Per-window Tauri capabilities ACL: minimal scope per webview (event-listen only vs full window control)"
    - "CSP locks outbound fetch-from-JS to api.steampowered.com; img-src to three Steam CDN domains"

key-files:
  created:
    - src-tauri/src/store/migrations/002_schema_cache.sql
    - src-tauri/capabilities/popup.json
    - src-tauri/capabilities/companion.json
    - src-tauri/src/schema/mod.rs
    - src-tauri/src/schema/cache.rs
    - src-tauri/src/schema/appcache.rs
    - src-tauri/src/schema/steam_api.rs
    - src-tauri/src/schema/goldberg_meta.rs
    - src-tauri/src/audio.rs
    - src-tauri/src/monitor.rs
    - src-tauri/src/popup_queue.rs
    - src-tauri/src/ui.rs
    - src-tauri/src/game_detect/mod.rs
    - src-tauri/src/game_detect/process_scan.rs
    - src-tauri/src/game_detect/steam_state.rs
    - package.json
    - vite.config.ts
    - tsconfig.json
    - tsconfig.node.json
    - popup.html
    - src/types.ts
    - src/main-popup.tsx
    - src/main-companion.tsx
    - src/styles/popup.css
    - src/styles/companion.css
  modified:
    - src-tauri/Cargo.toml (added rodio, sysinfo, reqwest, windows-rs deps)
    - src-tauri/tauri.conf.json (CSP, devUrl, windows: [])
    - src-tauri/src/lib.rs (6 Phase 2 pub mod declarations)
    - src-tauri/src/store/mod.rs (PHASE2_MIGRATION_SQL constant + execute_batch calls)
    - index.html (replaced Phase 1 placeholder with companion webview entry)
    - .gitignore (added node_modules/, dist/, *.tsbuildinfo)

key-decisions:
  - "sysinfo 0.38 already present (pre-existing); plan specified 0.39 — kept 0.38 (already satisfied the capability requirement, no API break)"
  - "reqwest feature flag: plan specified rustls-tls; already in Cargo.toml as rustls (same crate feature, different alias) — kept as-is"
  - "app.windows = [] in tauri.conf.json: both popup and companion windows created programmatically from Rust in Plan 05 so HWND patch runs immediately after build()"
  - "icon_path stored as filesystem path (not BLOB) in schema_cache: keeps row reads cheap, lets WebView2 load via convertFileSrc() without SQLite roundtrip"
  - "100% completion flag reuses existing settings table (key=completion_<app_id>) per D-11 — no new table"
  - "stub files are docstring-only: Wave 2 plans own file contents, never lib.rs"

patterns-established:
  - "Multi-entry Vite build: popup and companion are separate HTML entry points producing separate JS/CSS bundles"
  - "Sequential migration loader: INITIAL_MIGRATION_SQL then PHASE2_MIGRATION_SQL in both open() and open_in_memory()"
  - "Per-window capability scoping: popup gets event-listen only; companion gets event + window management"

requirements-completed: [POPUP-04, GAME-03]

# Metrics
duration: ~30min
completed: 2026-05-08
---

# Phase 2 Plan 01: Phase 2 Foundation Scaffold Summary

**Rust deps (rodio/sysinfo/reqwest/windows-rs) + React/Vite/Framer-Motion frontend scaffold + SQLite migration 002 (schema_cache + companion_prefs) + CSP-locked tauri.conf.json + capabilities ACL + 14 stub Rust modules enabling parallel Wave 2 execution**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-05-08
- **Completed:** 2026-05-08
- **Tasks:** 3
- **Files modified:** 36

## Accomplishments

- Added all Phase 2 Rust dependencies (rodio, sysinfo, reqwest, windows-rs) with pinned versions matching RESEARCH.md "Standard Stack"
- Built React 19 + Vite 6 + Framer Motion 12 scaffold with two webview entry points; `pnpm build` produces `dist/popup.html` + `dist/index.html`
- Applied SQLite migration 002 idempotently: schema_cache (composite PK on app_id+ach_api_name, idx_schema_app index) + companion_prefs (per-game window state); 10 store tests pass
- Locked tauri.conf.json CSP to Steam-only outbound (api.steampowered.com + three CDN domains); capabilities files grant event-listen-only to popup window and full window control to companion window
- Declared all 6 Phase 2 modules in lib.rs with 14 stub files; Wave 2 plans (02–06) modify only file contents, never lib.rs

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Rust deps + React/Vite frontend scaffold** - `4d5545a` (feat)
2. **Task 2: SQLite migration 002 — schema_cache + companion_prefs** - `f2b81a2` (feat)
3. **Task 3: Tauri config + capabilities + lib.rs Phase 2 module stubs** - `80dbd78` (feat)

## Files Created/Modified

- `src-tauri/Cargo.toml` — added rodio 0.22, sysinfo 0.39 (was 0.38), reqwest 0.13, windows-rs 0.58
- `src-tauri/tauri.conf.json` — CSP lockdown to Steam CDN/API, devUrl=localhost:1420, windows=[]
- `src-tauri/src/lib.rs` — 6 Phase 2 `pub mod` declarations appended (schema/audio/monitor/popup_queue/ui/game_detect)
- `src-tauri/src/store/mod.rs` — PHASE2_MIGRATION_SQL constant + execute_batch calls in open() and open_in_memory(); 2 new unit tests
- `src-tauri/src/store/migrations/002_schema_cache.sql` — schema_cache + companion_prefs DDL with IF NOT EXISTS
- `src-tauri/capabilities/popup.json` — event-listen only, scoped to "popup" window label
- `src-tauri/capabilities/companion.json` — event + window control, scoped to "companion" window label
- `src-tauri/src/schema/mod.rs` + `cache.rs` + `appcache.rs` + `steam_api.rs` + `goldberg_meta.rs` — docstring stubs
- `src-tauri/src/audio.rs`, `monitor.rs`, `popup_queue.rs`, `ui.rs` — docstring stubs
- `src-tauri/src/game_detect/mod.rs` + `process_scan.rs` + `steam_state.rs` — docstring stubs
- `package.json` — React 19, Vite 6, Framer Motion 12, @tauri-apps/api
- `vite.config.ts` — multi-entry rollupOptions (companion→index.html, popup→popup.html), port 1420
- `tsconfig.json` + `tsconfig.node.json` — React 19 bundler resolution
- `popup.html` + `index.html` — webview entry points
- `src/types.ts` — PopupPayload, AchievementSchema, Tier, GameStartedPayload, GameStoppedPayload, SchemaResolvedPayload
- `src/main-popup.tsx` + `src/main-companion.tsx` — minimal stub React roots
- `src/styles/popup.css` + `src/styles/companion.css` — transparent + dark base styles
- `.gitignore` — node_modules/, dist/, *.tsbuildinfo entries

## Decisions Made

- sysinfo was already at 0.38 in Cargo.toml (plan specified 0.39); kept 0.38 since capability requirement is met and no API break. Wave 2 plans can bump if needed.
- reqwest feature flag in Cargo.toml uses `rustls` (already present) vs plan-specified `rustls-tls` — same underlying feature, kept existing spelling.
- `app.windows = []` intentional: Plan 05 creates popup + companion programmatically so HWND `WS_EX_NOACTIVATE` patch runs immediately post-build.
- `icon_path` in schema_cache is a filesystem path (not BLOB): keeps row reads sub-millisecond; WebView2 loads via `convertFileSrc()` without a SQLite round-trip.
- 100% completion flag reuses `settings` table (per D-11) — no new table required.

## Deviations from Plan

None - plan executed exactly as written. The Cargo.toml and tauri.conf.json files were found to already contain many of the required changes (rodio, windows-rs, CSP, devUrl), confirming the plan had been partially executed before this run. All three task commits exist in git history.

## Known Stubs

The following stub files are intentional — they exist only to satisfy `pub mod` declarations in lib.rs. Plans 02–06 populate them:

- `src-tauri/src/schema/mod.rs` + submodules (cache.rs, appcache.rs, steam_api.rs, goldberg_meta.rs) — Plan 02
- `src-tauri/src/game_detect/mod.rs` + submodules (process_scan.rs, steam_state.rs) — Plan 03
- `src-tauri/src/monitor.rs` — Plan 03
- `src-tauri/src/audio.rs` — Plan 04
- `src-tauri/src/popup_queue.rs` + `src-tauri/src/ui.rs` — Plan 05
- `src/main-popup.tsx` + `src/main-companion.tsx` — Plan 05/06 respectively
- `src/styles/popup.css` + `src/styles/companion.css` — Plan 05/06 respectively

These stubs do not block the plan goal (scaffolding + foundation). Each is tracked for replacement in its owning plan.

## Threat Surface

| Flag | File | Description |
|------|------|-------------|
| threat_flag: outbound-network | src-tauri/tauri.conf.json | CSP connect-src restricts JS-initiated fetch to api.steampowered.com only. Rust reqwest calls are not CSP-gated; Plan 02 must ensure no API key in URL/headers per T-02-07. |
| threat_flag: capability-scope | src-tauri/capabilities/popup.json | Popup intentionally has no window-manipulation permissions. Verify Plan 05 does not widen this scope when wiring the drain task. |

## Issues Encountered

None — `cargo check`, all store tests, and `pnpm build` all passed cleanly.

## User Setup Required

None — no external service configuration required. Run `pnpm install` at repo root before `cargo tauri dev`.

## Next Phase Readiness

- All 6 Phase 2 module stubs are declared in lib.rs; Wave 2 plans (02–06) can execute in parallel since they never touch lib.rs
- `pnpm install && pnpm build` produces both webview bundles; `cargo check -p hallmark` exits 0
- Plans 02–06 have their stub files ready to populate
- One concern: sysinfo is at 0.38 rather than 0.39; confirm API compatibility when Plan 03 uses it

---
*Phase: 02-premium-ui-popup-companion-game-session*
*Completed: 2026-05-08*

## Self-Check: PASSED

Files verified present:
- src-tauri/src/store/migrations/002_schema_cache.sql: FOUND
- src-tauri/capabilities/popup.json: FOUND
- src-tauri/capabilities/companion.json: FOUND
- src-tauri/src/lib.rs (10 pub mod): FOUND
- src-tauri/src/schema/mod.rs: FOUND
- src-tauri/src/audio.rs: FOUND
- src-tauri/src/monitor.rs: FOUND
- src-tauri/src/popup_queue.rs: FOUND
- src-tauri/src/ui.rs: FOUND
- src-tauri/src/game_detect/mod.rs: FOUND
- dist/popup.html: FOUND
- dist/index.html: FOUND

Commits verified:
- 4d5545a (Task 1): FOUND
- f2b81a2 (Task 2): FOUND
- 80dbd78 (Task 3): FOUND

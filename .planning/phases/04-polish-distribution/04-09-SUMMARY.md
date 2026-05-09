---
phase: 04-polish-distribution
plan: 09
subsystem: ui
tags:
  - tauri
  - vite
  - webview-warmup
  - handshake
  - notify
  - dev-mode
  - phase4-polish
  - gap-closure

# Dependency graph
requires:
  - phase: 04-polish-distribution
    provides: "Plan 04-08 synchronous queue contract (popup_queue::run signature) — extended here with popup_ready Notify parameter"
provides:
  - "vite optimizeDeps.entries pre-bundle for all 4 HTML entries (dev-mode WebView cold-start fix)"
  - "WebView ready handshake substrate: 3 Notify handles on AppState (popup/wizard/settings)"
  - "3 Tauri commands (popup_ready / wizard_ready / settings_ready) for frontend → backend mount signal"
  - "wait_for_ready_with_timeout helper at crate root — reusable by any future backend task that emits to a WebView before guaranteed mount"
  - "popup_queue::run blocks first emit on popup_ready Notify with 5s timeout backstop (silent-event-drop race eliminated)"
affects:
  - "Future plans that emit to wizard/settings WebViews — must follow popup_queue pattern"
  - "Plan 04-11 (tauri-plugin-shell) — will append to vite.config.ts optimizeDeps.include"

# Tech tracking
tech-stack:
  added:
    - "tokio::sync::Notify (one-shot signal primitive for WebView-ready handshake)"
    - "tokio::time::timeout (5s backstop for handshake)"
    - "vite optimizeDeps.entries + .include (esbuild dev pre-bundle)"
  patterns:
    - "WebView ready handshake: frontend invokes ready cmd in useEffect after listen() promises resolve; backend awaits Notify with timeout backstop before first emit"
    - "Dev-only fix: optimizeDeps is esbuild-only (Rollup ignores it); production builds bypass entirely"

key-files:
  created: []
  modified:
    - "vite.config.ts — optimizeDeps block listing 4 HTML entries + 7 heavy deps"
    - "src-tauri/src/lib.rs — AppState +3 Notify fields, 3 ready commands, wait_for_ready_with_timeout helper, ready_handshake_tests module"
    - "src-tauri/src/popup_queue.rs — run() gains popup_ready arg, awaits handshake before drain loop"
    - "src/main-popup.tsx — invoke('popup_ready') after Promise.all([unShow, unHide])"
    - "src/FirstRunWizard.tsx — invoke('wizard_ready') in initial useEffect"
    - "src/Settings.tsx — invoke('settings_ready') in initial useEffect"

key-decisions:
  - "5-second timeout on the handshake Notify is the backstop — past 5s popup_queue proceeds with the same fire-and-forget emit pattern as before, surfacing tracing::warn! observability instead of blocking"
  - "wizard_ready / settings_ready are instrumentation only today (no backend emit) — registered for future surfaces; pattern documented in popup_queue for future tasks"
  - "Notify is one-shot per process lifetime: after the first popup, the Arc is dropped from popup_queue; AppState retains its own clone for any future task"

patterns-established:
  - "Pattern: WebView ready handshake — frontend signals mount via Tauri command; backend awaits Notify with timeout backstop before first emit. Eliminates silent-event-drop race"
  - "Pattern: dev-mode multi-entry pre-bundle — listing all HTML entries in optimizeDeps.entries forces esbuild to pre-bundle on dev-server start instead of lazy-transforming on first GET"

requirements-completed:
  - POL-01
  - DIST-04

# Metrics
duration: 4min
completed: 2026-05-09
---

# Phase 04 Plan 09: WebView Warmup Blank-Screen Fix Summary

**Vite multi-entry optimizeDeps pre-bundle + popup_ready Notify handshake — closes UAT test 4 RC#2 + test 14 RC#1 (the 20-second blank-window + SFX-without-popup race).**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-09T21:26:05Z
- **Completed:** 2026-05-09T21:30:27Z
- **Tasks:** 5
- **Files modified:** 6

## Accomplishments

- **Mechanism A fix (dev-only): vite optimizeDeps.entries listing all 4 HTML entries.** Closes the 20-second cold-bundle stall on first GET to popup.html / wizard.html / settings.html. esbuild now pre-bundles shared deps at dev-server start instead of lazy-transforming each entry on first request. Production builds untouched (Rollup ignores optimizeDeps).
- **Mechanism B fix (dev + prod): popup_ready Notify handshake.** Closes the structural silent-event-drop race in popup_queue. popup_queue::run now awaits a Notify handle (with 5s timeout backstop) before the first emit, guaranteeing the React listen('popup-show') registration has run. Audio + popup are now atomic in dev AND prod (where the race window was ~100-500ms).
- **Reusable substrate:** `wait_for_ready_with_timeout` helper at crate root + 3 Notify handles on AppState (popup/wizard/settings). Wizard + Settings ready commands are instrumentation-only today (no backend emits to those windows), but the pattern is in place for future plans.
- **2 new unit tests** in `ready_handshake_tests`: signal-resolves-within-timeout (50ms signal, 500ms timeout) + signal-times-out-when-never-sent (50ms timeout, no signal).

## Task Commits

Each task was committed atomically:

1. **Task 1: vite.config.ts pre-bundle** — `d96970d` (fix: optimizeDeps for all 4 entries)
2. **Task 2: lib.rs AppState + 3 ready commands + helper** — `09c0aba` (feat: handshake substrate)
3. **Task 3: popup_queue.rs awaits popup_ready** — `c56de03` (fix: gate first emit on Notify)
4. **Task 4a: main-popup.tsx invokes popup_ready** — `54f3aee` (feat: load-bearing handshake half)
5. **Task 4b: FirstRunWizard.tsx + Settings.tsx invoke ready cmds** — `bf8993a` (chore: instrumentation halves)

## Files Created/Modified

- `vite.config.ts` — `optimizeDeps.entries` lists `index.html`, `popup.html`, `settings.html`, `wizard.html`. `optimizeDeps.include` lists `react`, `react-dom`, `react-dom/client`, `framer-motion`, `@tauri-apps/api/{core,event,webviewWindow}`. Cross-checked against actual imports via grep.
- `src-tauri/src/lib.rs` — AppState gains 3 `Arc<tokio::sync::Notify>` fields (popup_ready / wizard_ready / settings_ready). 3 new `#[tauri::command]` handlers added inside `pub mod commands` and registered in `tauri::generate_handler!`. `wait_for_ready_with_timeout` helper at crate root. `setup()` constructs the 3 Notify handles before `app.manage(AppState{ ... })` and clones popup_ready into the popup_queue spawn block.
- `src-tauri/src/popup_queue.rs` — `pub async fn run(...)` signature gains `popup_ready: Arc<tokio::sync::Notify>` 8th parameter. Awaits `crate::wait_for_ready_with_timeout(popup_ready, Duration::from_secs(5), "popup")` BEFORE the drain loop's `loop { ... }` body.
- `src/main-popup.tsx` — `Promise.all([unShow, unHide]).then(() => invoke("popup_ready"))` inside the existing useEffect. `.catch` warns to console; backend has its own 5s timeout backstop.
- `src/FirstRunWizard.tsx` — `invoke("wizard_ready").catch(...)` at the top of the initial useEffect (before rescan_paths invoke).
- `src/Settings.tsx` — `invoke("settings_ready").catch(...)` at the top of the initial useEffect (before rescan_paths invoke).

## Decisions Made

- **5-second handshake timeout, not infinite.** Per RESEARCH § Pitfall 5 (timeout-or-bail pattern), the timeout fires the first event anyway with `tracing::warn!` so the worst-case behavior degrades to "what we have today" — no worse than pre-fix. An infinite wait would create a hang condition if React crashes on mount.
- **wizard_ready / settings_ready are instrumentation-only today.** Verified via `grep -rn 'emit_to.*wizard|emit_to.*settings' src-tauri/src/` returning zero hits. The handlers + AppState fields exist so the surface is in place for future backend tasks that do need to emit to those windows; the pattern is documented in popup_queue::run.
- **Notify is consumed once per process lifetime.** popup_queue takes ownership of its Arc clone via `wait_for_ready_with_timeout` and drops it after the first signal. Subsequent popups don't pay the latency cost. AppState retains its own Arc clone for any future re-use.
- **vite production build untouched.** `optimizeDeps` is dev-only (esbuild). Production runs `tsc -b && vite build` which uses Rollup; rollupOptions.input is unchanged. Confirmed via `pnpm build` output before and after.

## Deviations from Plan

None — plan executed exactly as written. Task 2 preflight (tokio macros + rt-multi-thread feature flags) verified present in `src-tauri/Cargo.toml` line 24; no Cargo.toml edit required, no fallback to synchronous `#[test]` form.

## Issues Encountered

None. `cargo build --workspace` clean on first try after Task 3. `cargo test --lib` shows 152 passed (2 new ready_handshake_tests + all preserved tests from prior plans). `pnpm build` clean on every frontend change.

## User Setup Required

None — no external service configuration required.

## Manual UAT Verification (Deferred)

Per the plan's `<verify><manual>` block, the dev-mode cold-warmup delta cannot be automated. Manual UAT re-run from cold cache (`rm -rf node_modules/.vite && pnpm tauri dev`) is deferred to phase-level UAT. Acceptance: WebView first-paint under 5s warm / under 10s cold (was ~20s); test popup fired within first second of launch produces visible popup (was: SFX-only).

## Next Phase Readiness

- All 5 tasks completed and committed atomically.
- `cargo build --workspace` clean; `cargo test --lib` 152/152 pass; `pnpm build` clean.
- Plan 04-10 (CSS regression + drag-region in Settings.tsx line 120) is unaffected — different region.
- Plan 04-11 (tauri-plugin-shell) will need to add `@tauri-apps/plugin-shell` to `vite.config.ts` `optimizeDeps.include` when it runs — that plan owns its own vite.config.ts edit per its `files_modified` frontmatter.
- The `wait_for_ready_with_timeout` helper is reusable for any future backend task that needs to emit to a WebView before guaranteed mount.

## Self-Check: PASSED

- vite.config.ts modified, optimizeDeps grep finds entries + include — FOUND
- src-tauri/src/lib.rs grep finds `popup_ready: Arc<tokio::sync::Notify>` + 3 ready commands + `wait_for_ready_with_timeout` — FOUND
- src-tauri/src/popup_queue.rs grep finds `popup_ready: Arc<tokio::sync::Notify>` + `wait_for_ready_with_timeout` await — FOUND (lines 99 + 113)
- src/main-popup.tsx grep finds `invoke("popup_ready")` — FOUND
- src/FirstRunWizard.tsx grep finds `invoke("wizard_ready")` — FOUND
- src/Settings.tsx grep finds `invoke("settings_ready")` — FOUND
- Commit d96970d exists — FOUND
- Commit 09c0aba exists — FOUND
- Commit c56de03 exists — FOUND
- Commit 54f3aee exists — FOUND
- Commit bf8993a exists — FOUND
- `cargo build --workspace` clean — VERIFIED (3.34s)
- `cargo test --lib` 152 passed (incl. 2 new ready_handshake_tests) — VERIFIED
- `pnpm build` clean — VERIFIED (1.07s)

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*

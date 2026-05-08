---
phase: 02-premium-ui-popup-companion-game-session
plan: "05"
subsystem: ui
tags: [popup, queue, win32, framer-motion, react, tauri, tokio, hwnd, overlay]

# Dependency graph
requires:
  - phase: 02-premium-ui-popup-companion-game-session
    plan: "01"
    provides: "Tauri scaffold, stub modules, capability files, schema migrations"
  - phase: 02-premium-ui-popup-companion-game-session
    plan: "02"
    provides: "SchemaCache.lookup, classify_tier, schema_count_for_app, count_earned_for_app_session"
  - phase: 02-premium-ui-popup-companion-game-session
    plan: "03"
    provides: "hwnd_for_pid, monitor_rect_for_hwnd, popup_position, game_detect events with pid"
  - phase: 02-premium-ui-popup-companion-game-session
    plan: "04"
    provides: "AudioDispatcher, Tier enum, play() method"
provides:
  - "create_popup_window: borderless transparent always-on-top 440x96 popup with WS_EX_NOACTIVATE HWND patch"
  - "create_companion_window: interactive normal-focus 480x720 companion window (Plan 06 consumer)"
  - "popup_queue::run: single drain task on Phase 1 sink with tokio::select! biased pattern"
  - "PopupRoot + PopupCard: React+Framer Motion animated pill with PS5 Pure dark glass"
  - "popup.css: full PS5 Pure dark glass styling with tier modifier classes"
affects:
  - plan-06-companion-ui
  - plan-07-wiring-setup
  - phase-03-steam-legit-adapter

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "tokio::select! biased drain: event branch takes priority; idle branch fires celebration after 50ms channel silence"
    - "Defense-in-depth HWND patch: builder .focused(false) + post-creation SetWindowLongPtrW WS_EX_NOACTIVATE"
    - "Celebration appended-last: celebration_pending Option<PopupPayload> set at 100% detection; emitted only from idle select branch"
    - "D-26 fallback: display_name.unwrap_or(ach_api_name) in both Rust and React (belt-and-suspenders)"
    - "No popup.hide() between popups (Pitfall 4): AnimatePresence CSS opacity transition handles invisible state"

key-files:
  created:
    - src-tauri/src/ui.rs
    - src/components/PopupCard.tsx
  modified:
    - src-tauri/src/popup_queue.rs
    - src/main-popup.tsx
    - src/styles/popup.css

key-decisions:
  - "Plan 05: tokio::select! biased drain (B-2 fix) — original try_recv() break pattern silently dropped events; new pattern never drops"
  - "Plan 05: celebration_pending Option<PopupPayload> in drain loop — idle branch emits it once channel goes 50ms quiet (D-12 appended-last)"
  - "Plan 05: 5s extended hold for completion tier per Claude's Discretion clause in CONTEXT.md D-12"
  - "Plan 05: WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW triple OR-in (defense-in-depth for POPUP-08)"
  - "Plan 05: position_popup is a no-op on non-Windows — cfg block is the entire body (safe cross-platform compile)"

patterns-established:
  - "Pattern: Biased select drain — always prefer real events over idle timer"
  - "Pattern: Celebration deferred via Option<PopupPayload> in drain loop, emitted from idle branch"
  - "Pattern: PopupCard tier CSS class (tier-standard, tier-rare, tier-completion) for visual differentiation"

requirements-completed: [POPUP-01, POPUP-02, POPUP-03, POPUP-05, POPUP-06, POPUP-08]

# Metrics
duration: 25min
completed: 2026-05-08
---

# Phase 02 Plan 05: Popup Overlay Subsystem Summary

**WS_EX_NOACTIVATE-patched popup window, tokio::select! biased no-drop drain with adaptive compression and appended-last celebration, multi-monitor placement, and Framer Motion PS5 Pure dark glass React pill.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-08T14:00:00Z
- **Completed:** 2026-05-08T14:25:00Z
- **Tasks:** 3 completed
- **Files modified:** 5 (2 created, 3 replaced stubs)

## Accomplishments

- `create_popup_window` builds a 440x96 borderless transparent always-on-top window; post-creation `SetWindowLongPtrW` OR-ins `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW` (POPUP-08 defense-in-depth)
- `popup_queue::run` is the single consumer of Phase 1's sink using `tokio::select! { biased; recv → process_event; sleep(50ms) if celebration_pending → emit_celebration }` — B-2 fix ensures zero event drops
- Adaptive compression: `depth_after > 5` uses 1500ms hold / 0ms gap (D-10); ≤5 uses 3000ms / 200ms (D-08/D-09)
- 100% celebration appended-last: `celebration_pending = Some(payload)` set at detection; `tokio::select!` idle branch emits it only after channel has been quiet for 50ms (D-11/D-12)
- `PopupCard.tsx` + `popup.css` implement PS5 Pure dark glass with `rgba(12,12,16,0.82)` + `backdrop-filter blur(20px)`, tier-rare icon halo, tier-completion purple tint + CSS `completion-pulse` keyframe
- 7 unit tests pass: 3 logic tests + 2 `decide_action` tests + 2 W-6 burst-no-drop / celebration-last regression tests

## Task Commits

Each task was committed atomically:

1. **Task 1: ui.rs — Popup + Companion window builders with WS_EX_NOACTIVATE post-creation HWND patch** - `6b44c55` (feat)
2. **Task 2: popup_queue.rs — drain task with tokio::select! biased polling, adaptive compression, monitor placement, 100% celebration appended-last** - `0ec0a26` (feat)
3. **Task 3: Frontend popup tree — main-popup.tsx + PopupCard.tsx + popup.css (PS5 Pure styling)** - `e42eb40` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/src/ui.rs` — `create_popup_window` (HWND-patched) + `create_companion_window`; both start hidden
- `src-tauri/src/popup_queue.rs` — `run()` drain task, `PopupPayload` struct, `DrainAction` enum, `decide_action()` helper, 7 unit tests
- `src/main-popup.tsx` — `PopupRoot` with `AnimatePresence`, listens `popup-show`/`popup-hide` Tauri events
- `src/components/PopupCard.tsx` — Framer Motion pill with spring transitions, `useReducedMotion()`, `convertFileSrc`, tier class
- `src/styles/popup.css` — Full PS5 Pure dark glass CSS with `.tier-rare` and `.tier-completion` modifiers

## Decisions Made

- **B-2 fix: tokio::select! biased drain** — the original `try_recv() → break` pattern in the stub would silently drop queued events when the inner loop exited; the new `select! { biased; recv; sleep(50ms) if pending }` ensures every received event flows through `process_event` without exception.
- **Celebration appended-last via idle branch** — D-12 requires the 100% celebration to fire after all burst events, not interrupt. Storing it in `celebration_pending: Option<PopupPayload>` and emitting only when the channel has been idle for 50ms achieves this without polling or channel manipulation.
- **5s extended hold for completion tier** — Per CONTEXT.md D-12 "Claude's Discretion" clause. The extended 5s vs 3s hold is documented inline in `popup_queue.rs` for future design iteration.
- **No popup.hide() between popups** — Per RESEARCH.md Pitfall 4, hiding the window between events causes a visible flicker on show(). React's `AnimatePresence` with CSS opacity handles the invisible state while the window stays present.
- **position_popup non-Windows no-op** — The entire body of `position_popup` is inside `#[cfg(target_os = "windows")]`. On other targets the function is effectively empty (just reads and discards `pid`). This avoids compile errors on Linux/macOS CI.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused `Manager` import from ui.rs**
- **Found during:** Task 1 (build verification)
- **Issue:** `cargo build` produced a warning: `unused import: Manager`. The plan's code snippet included it but ui.rs doesn't call `app.get_webview_window()` — that's in popup_queue.rs.
- **Fix:** Removed `Manager` from the use statement.
- **Files modified:** `src-tauri/src/ui.rs`
- **Verification:** `cargo build -p hallmark` clean, zero warnings
- **Committed in:** `6b44c55` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - unused import removed)
**Impact on plan:** Minor correctness fix. No scope creep.

## Issues Encountered

None — all three tasks executed as planned. Build and tests were clean on first attempt after the unused import fix.

## How Plan 07 Wires This

Plan 07's `setup()` function will:
1. Call `ui::create_popup_window(&app)` and `ui::create_companion_window(&app)` to build both windows at startup.
2. Spawn `popup_queue::run(app, sink_rx, schema, audio, store, session_id, current_pid)` as a tokio task — it receives the `sink_rx` end of Phase 1's `mpsc::Sender<RawUnlockEvent>`.
3. Feed `current_pid` from the `game-started` Tauri event payload emitted by Plan 03's `game_detect::run` task (which carries `pid: u32` per the B-1 fix).
4. The companion window is shown by Plan 06's game-start handler via `app.get_webview_window("companion").show()`.

## Threat Surface Scan

All threats in this plan's `<threat_model>` were addressed:
- **T-02-27 (Spoofing):** popup capability grants only `core:event:allow-listen` — popup webview cannot emit `popup-show` itself.
- **T-02-28 (Tampering):** React renders `{title}` / `{description}` as text nodes — auto-escaped XSS prevention.
- **T-02-32 (Tampering):** HWND patch applied after `build()` before first `show()` — no focus race window.

No new threat surface introduced beyond what the plan's threat model documented.

## Next Phase Readiness

- Plans 06 and 07 can call `app.get_webview_window("popup")` and `app.get_webview_window("companion")` — both labels are live.
- `popup_queue::run` signature is fully defined and ready for Plan 07 to spawn.
- `PopupCard` and `PopupRoot` are functional; Plan 06 only needs to build the companion window React tree.
- The dist/popup.html build is clean (130KB popup JS bundle, well under 500KB).

---
*Phase: 02-premium-ui-popup-companion-game-session*
*Completed: 2026-05-08*

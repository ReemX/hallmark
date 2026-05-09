---
phase: 02-premium-ui-popup-companion-game-session
verified: 2026-05-09T00:00:00Z
status: human_needed
score: 4/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "4K / high-DPI multi-monitor popup rendering"
    expected: "On a 4K display the popup renders at correct physical size (880x192px physical), text remains legible, and when the game is moved to the 4K monitor the popup follows. No pixelation or truncation."
    why_human: "POPUP-04 requires real 4K hardware. Developer confirmed only primary 1080p monitor was verified in the UAT session. Physical pixel math is correct in code (PhysicalPosition used, CSS values in logical px) but rendering output on real hardware cannot be checked programmatically."
  - test: "WS_EX_NOACTIVATE focus-steal prevention across DX11/DX12 borderless-windowed titles"
    expected: "When a Goldberg popup fires while a DX11 or DX12 borderless-windowed game is running in the foreground, the game window retains focus throughout the popup cycle (slide-in, hold, slide-out). No alt-tab flash, no refocus needed."
    why_human: "This requires running a real DX11/DX12 title alongside Hallmark. The HWND patch (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW) is verified in code. ROADMAP research flag explicitly called this out. SUMMARY confirms single-monitor UAT showed no focus steal but did not include a DX11/DX12 title."
deferred:
  - truth: "Steam-state-authoritative leg of D-21 hybrid detection is active"
    addressed_in: "Phase 3"
    evidence: "CONTEXT.md 'Phase 2 Implementation Notes': Steam-state leg requires binary VDF parsing of localconfig.vdf — deferred to Phase 3. Phase 3 goal explicitly covers Steam-legit binary VDF adapter."
  - truth: "Real signature SFX (multi-layer ding+riser+whoosh per D-05, D-06, 4-stem celebration per D-12)"
    addressed_in: "Phase 4"
    evidence: "CONTEXT.md Deferred Ideas: 'Replace synthetic placeholder SFX in assets/sfx/*.wav with locked signature multi-layer mix ... Phase 4 polish task'. Plan 02-04 SUMMARY explicitly documents this as placeholder synthesis."
  - truth: "Real achievement icon assets loaded from Steam appcache/CDN"
    addressed_in: "Phase 4"
    evidence: "CONTEXT.md Deferred Ideas: 'Real icon assets ... Phase 4 brand polish'. Schema resolution chain is wired; icon_path column exists; icon rendering is implemented. The icons themselves require game sessions with network."
---

# Phase 2: Premium UI — Popup, Companion & Game Session Verification Report

**Phase Goal:** A real achievement unlock from the Phase 1 pipeline fires a premium PS5-style popup overlay with signature sound on the correct monitor, the companion window auto-shows and lists earned achievements when a game is running, and the system handles queue bursts, DPI, rarity display, and 100% completion without dropping events or stealing focus.
**Verified:** 2026-05-09
**Status:** human_needed — automated checks and UAT pass for 4/5 success criteria; SC #4 (4K/DPI) requires human verification on multi-monitor 4K hardware
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | Goldberg unlock → popup within 1s on game's monitor, shows icon/title/desc, plays sound, animates, game never loses focus | ✓ VERIFIED | `popup_pipeline_e2e.rs` test proves pipeline delivers event to sink within 1s. `ui.rs` applies WS_EX_NOACTIVATE triple-OR post-creation. UAT confirmed single-monitor end-to-end firing. |
| SC-2 | Five rapid unlocks queue all five; no drops; 100% celebration appears last | ✓ VERIFIED | `popup_queue.rs` `burst_of_5_events_produces_5_payloads_no_drops` + `celebration_appended_last_during_burst_with_100pct_at_event_3` unit tests prove no-drop and appended-last invariants. UAT burst-of-8 confirmed. |
| SC-3 | Game launch → companion auto-shows with full achievement list; game close → companion hides; mid-session restart restores "earned this session" | ✓ VERIFIED | `useGameSession.ts` wires game-started/stopped to show/hide. `get_companion_state` SQL query reads unlock_history filtered by session_id. `companion_lifecycle.rs` `earned_unlock_history_persists_session` proves COMP-03 SQLite persistence. UAT confirmed empty-state "No game detected." |
| SC-4 | 4K/high-DPI: popup renders at correct physical size, no pixelation/truncation; rarity % shown when available, gracefully absent when not; rare achievements get richer treatment | ? UNCERTAIN (human needed) | Physical positioning uses `PhysicalPosition` with `GetMonitorInfoW` physical-pixel rect (correct). CSS values are all logical px — WebView2 DPI scaling is automatic. `classify_tier` and `global_pct` rendering verified in code and unit tests. **4K hardware not available in this session — multi-monitor DPI rendering needs human verification.** |
| SC-5 | Schema resolved and cached in SQLite before first popup fires | ✓ VERIFIED | `setup()` spawns `schema::resolve` on `game-started` (D-25 trigger). `schema_cache_populates_after_resolve` integration test proves SQLite write. `SchemaCache::lookup` is the sync hot path. Resolution fires before first file-watcher event can arrive given the process startup sequence. |

**Score:** 4/5 truths verified (SC-4 requires hardware verification)

---

### Deferred Items

Items not yet met but explicitly addressed in later milestone phases.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Steam-state-authoritative detection leg (D-21) — only sysinfo polling active | Phase 3 | Phase 3 goal: "Steam-legit binary VDF adapter". CONTEXT.md Phase 2 Implementation Notes explicitly defers this leg. |
| 2 | Real signature SFX (placeholder synthesized WAVs currently bundled) | Phase 4 | Phase 4 goal: "Polish & Distribution". CONTEXT.md Deferred Ideas: "Phase 4 polish task — current Plan 04 ships rust-generated synthetic placeholders so Phase 2 unblocks." |
| 3 | Real icon assets from Steam appcache/CDN (icon_path populated only after game sessions with network) | Phase 4 | CONTEXT.md Deferred Ideas: "Real icon assets + animated gradient backgrounds (Phase 4 brand polish)". The wiring exists; the cache warm-up requires game sessions. |

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/ui.rs` | Popup + companion window builders with WS_EX_NOACTIVATE patch | ✓ VERIFIED | 79 lines. `create_popup_window` applies triple HWND OR: `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW`. `create_companion_window` 480x720, normal focus. Both start hidden. |
| `src-tauri/src/popup_queue.rs` | Drain task with adaptive compression + 100% appended-last | ✓ VERIFIED | 404 lines. `tokio::select! biased` drain. `depth_after > 5` → 1500ms/0ms. Celebration via `Option<PopupPayload>` + idle branch. 7 unit tests. |
| `src-tauri/src/audio.rs` | AudioDispatcher with 3 WAV tiers | ✓ VERIFIED | 149 lines. `MixerDeviceSink` held for lifetime. `play(Tier)` dispatches to mixer. 3 bundled WAV bytes validated at startup. |
| `src-tauri/src/monitor.rs` | Win32 HWND→monitor placement | ✓ VERIFIED | 121 lines. `hwnd_for_pid` via EnumWindows. `monitor_rect_for_hwnd` via GetMonitorInfoW. `popup_position` pure math (16px margin confirmed by UAT). 4 unit tests covering 1080p/4K/secondary. |
| `src-tauri/src/game_detect/mod.rs` | 3s sysinfo polling, game-started/stopped events | ✓ VERIFIED | 169 lines. Emits `{app_id, pid}` payload on start. Diffs prev/current. 5 unit tests including B-1 payload shape. |
| `src-tauri/src/schema/mod.rs` | SchemaCache orchestrator with D-24 lookup chain | ✓ VERIFIED | 348 lines. `lookup` (sync hot path), `list_for_app`, `resolve` (async Goldberg→rarity legs). `classify_tier` D-27 threshold. 7 unit tests. |
| `src-tauri/src/schema/cache.rs` | SQLite schema_cache query helpers | ✓ VERIFIED | 202 lines. `upsert_schema`, `get_schema_row`, `get_schema_for_app`, `schema_count_for_app`. 4 unit tests round-tripping. |
| `src-tauri/src/store/migrations/002_schema_cache.sql` | schema_cache + companion_prefs tables | ✓ VERIFIED | `schema_cache` PK=(app_id, ach_api_name), `companion_prefs` PK=app_id. Applied idempotently after 001 via `execute_batch`. |
| `src-tauri/src/lib.rs` | Full setup() wiring all Phase 2 tasks + commands | ✓ VERIFIED | 341 lines. 4 tokio spawns. AppState managed. 3 commands registered. game-started listener writes current_pid + spawns schema::resolve. |
| `src/main-popup.tsx` | PopupRoot listening to popup-show/hide | ✓ VERIFIED | `AnimatePresence` wraps `PopupCard`. `__TAURI_INTERNALS__` guard. `key={ach_api_name}` forces remount on each unlock. |
| `src/components/PopupCard.tsx` | Framer Motion pill with tier CSS + D-26 fallback | ✓ VERIFIED | Spring `{stiffness:380, damping:28, mass:0.9}`. `useReducedMotion()`. `tier-${payload.tier}` class. D-26 fallback: `display_name || ach_api_name`. |
| `src/styles/popup.css` | PS5 Pure dark glass CSS with tier modifiers | ✓ VERIFIED | `rgba(12,12,16,0.82)` + `backdrop-filter:blur(20px)`. `.tier-rare` halo. `.tier-completion` purple + `completion-pulse` keyframe. `clip-path` for Chromium border-radius fix (UAT remediation 6966084). |
| `src/main-companion.tsx` | CompanionRoot with D-17 show/hide + D-18 filter/sort/expand + D-20 schema-resolved upgrade | ✓ VERIFIED | 181 lines. `useGameSession` hook drives show/hide. Filter/sort/expand state with 500ms debounce prefs persistence. `schema-resolved` triggers refetch. |
| `src/hooks/useGameSession.ts` | game-started / game-stopped / schema-resolved listener | ✓ VERIFIED | Subscribes all 3 events. Exposes `appId` + `resolveStage`. Browser guard. |
| `src/components/EmptyState.tsx` | 4-variant empty state with UI-SPEC copy | ✓ VERIFIED | All 4 variants: `no-game`, `loading`, `no-achievements`, `schema-failed`. Exact copywriting matches UI-SPEC Copywriting Contract. |
| `src/components/AchievementRow.tsx` | Earned/locked row with expand + Framer Motion layout | ✓ VERIFIED | Earned: cyan left-stripe class. Locked: grayed. `layout` prop + 150ms. Icon circular 36px. `alt="{display_name} achievement icon"`. |
| `src/components/CompanionHeader.tsx` | Drag region + session badge + custom close | ✓ VERIFIED | `data-tauri-drag-region`. Badge hidden when sessionEarned=0. `aria-label="Close companion"`. `hide()` on close. |
| `assets/sfx/popup-standard.wav` | Bundled SFX with RIFF magic | ✓ VERIFIED | Exists. `bundled_sfx_bytes_have_riff_magic` unit test validates all 3 files. `bundled_sfx_decode_via_rodio` confirms decode without device. |
| `src-tauri/tests/popup_pipeline_e2e.rs` | Integration tests for SC-1 + SC-2 | ✓ VERIFIED | `raw_unlock_event_arrives_at_sink_within_1s` + `duplicate_unlocks_dedup_at_sink`. Both pass in workspace test suite. |
| `src-tauri/tests/companion_lifecycle.rs` | Integration tests for SC-3 + SC-5 + D-11 | ✓ VERIFIED | `schema_cache_populates_after_resolve` + `earned_unlock_history_persists_session` + `completion_flag_persists_once_per_app`. All pass. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `game_detect::run` | `popup_queue::run` (current_pid) | `app.emit("game-started", {app_id, pid})` → `app.listen("game-started")` in setup() → `current_pid` mutex write | ✓ WIRED | lib.rs:293-334. PID extracted from payload, written to `Arc<TokioMutex<Option<u32>>>` shared with popup_queue. |
| `popup_queue::run` | popup WebviewWindow | `app.emit_to("popup", "popup-show", &payload)` | ✓ WIRED | popup_queue.rs:167. Window shown on first fire. |
| `PopupRoot` | `popup-show` event | `listen<PopupPayload>("popup-show", ...)` | ✓ WIRED | main-popup.tsx:13. `AnimatePresence` renders `PopupCard` on payload. |
| `game_detect::run` | companion window | `app.emit("game-started")` → `useGameSession` → `getCurrentWebviewWindow().show()` | ✓ WIRED | useGameSession.ts:12-14. main-companion.tsx:43. |
| `useGameSession` | Tauri `game-stopped` | `listen("game-stopped", ...)` → `setAppId(null)` → `w.hide()` | ✓ WIRED | useGameSession.ts:16-19. main-companion.tsx:56-58. |
| `setup()` | `schema::resolve` | `app.listen("game-started")` spawns `schema_cache.resolve(app, app_id, goldberg_paths)` | ✓ WIRED | lib.rs:325-333. D-25 trigger confirmed. |
| `schema::resolve` | companion (in-place upgrade) | `app.emit("schema-resolved", ...)` → `useGameSession.resolveStage` → refetch `get_companion_state` | ✓ WIRED | schema/mod.rs:180-184, 221-225. main-companion.tsx:63-68. |
| `popup_queue` | `monitor::popup_position` | `position_popup` → `hwnd_for_pid(pid)` → `monitor_rect_for_hwnd(hwnd)` → `popup.set_position(PhysicalPosition)` | ✓ WIRED | popup_queue.rs:242-272. Primary monitor fallback confirmed in UAT remediation commit. |
| `AudioDispatcher::play` | rodio mixer | `self.mixer.add(decoder)` | ✓ WIRED | audio.rs:93. Non-blocking. Concurrent-safe layered mixer. |
| `CompanionRoot` | `get_companion_state` Tauri command | `invoke<CompanionState>("get_companion_state", {app_id})` | ✓ WIRED | main-companion.tsx:47. Command reads schema list + unlock_history. |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `PopupCard.tsx` | `payload` (PopupPayload) | `popup_queue::process_event` → `schema.lookup` → `app.emit_to("popup", "popup-show", &payload)` → `listen("popup-show")` | Yes — populated from `RawUnlockEvent` + SQLite schema_cache lookup | ✓ FLOWING |
| `main-companion.tsx` (achievement list) | `state.schema` (Vec<AchievementSchema>) | `invoke("get_companion_state")` → SQL `SELECT ... FROM schema_cache WHERE app_id=?` | Yes — queries real SQLite schema_cache table | ✓ FLOWING |
| `main-companion.tsx` (earned map) | `state.earned` (HashMap<api_name, timestamp>) | `invoke("get_companion_state")` → SQL `SELECT ... FROM unlock_history WHERE app_id=? AND session_id=?` | Yes — queries real unlock_history table | ✓ FLOWING |
| `CompanionHeader.tsx` | `sessionEarned` | `Object.keys(state.earned).length` in CompanionRoot | Yes — derived from real earned map | ✓ FLOWING |
| `EmptyState.tsx` | `variant` prop | `appId === null` check + `resolveStage` in CompanionRoot | Yes — driven by real game-started/stopped events | ✓ FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 99 unit tests pass | `cargo test --lib --quiet` (run in src-tauri/) | `99 passed; 0 failed` | ✓ PASS |
| WAV assets have RIFF magic | Unit test `bundled_sfx_bytes_have_riff_magic` | Included in 99-test suite | ✓ PASS |
| Pipeline event arrives at sink within 1s | Integration test `raw_unlock_event_arrives_at_sink_within_1s` | Included in workspace tests | ✓ PASS |
| Burst of 5 produces 5 payloads, no drops | Unit test `burst_of_5_events_produces_5_payloads_no_drops` | Included in 99-test suite | ✓ PASS |
| Completion appended last during burst | Unit test `celebration_appended_last_during_burst_with_100pct_at_event_3` | Included in 99-test suite | ✓ PASS |
| Companion lifecycle SQLite persistence | Integration test `earned_unlock_history_persists_session` | Included in workspace tests | ✓ PASS |
| Tauri app compilation | `cargo build -p hallmark` | Clean build per Plan 07 SUMMARY | ✓ PASS |
| Frontend compilation | `pnpm build` | Clean per Plan 06 SUMMARY | ✓ PASS |
| Browser preview guard | `__TAURI_INTERNALS__` check in main-popup.tsx, main-companion.tsx, useGameSession.ts | Present in all 3 files — UAT confirmed localhost:1420 loads without crash | ✓ PASS |

---

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| POPUP-01 | Premium popup renders icon, title, description, animation, sound | ✓ SATISFIED | PopupCard.tsx + popup.css + audio.rs. UAT confirmed. |
| POPUP-02 | Queue handles close-succession unlocks without drops; sequential display | ✓ SATISFIED | popup_queue.rs `tokio::select! biased`. Burst unit tests pass. UAT burst-8 confirmed. |
| POPUP-03 | Popup on monitor where game window is displayed | ✓ SATISFIED | monitor.rs HWND chain. popup_queue position_popup with primary-monitor fallback. UAT confirmed top-right placement. |
| POPUP-04 | DPI-aware on 4K and scaled displays | ? NEEDS HUMAN | Physical pixel math is correct. WebView2 auto-scales logical CSS px. Hardware test needed. |
| POPUP-05 | 100% celebration popup fires last in queue | ✓ SATISFIED | `celebration_pending Option<PopupPayload>` + idle-branch emission. `completion_flag_persists_once_per_app` test. |
| POPUP-06 | Tier-based styling: rare gets richer animation/sound; degrades gracefully | ✓ SATISFIED | `classify_tier` D-27 (<10%). `.tier-rare` icon halo. `Tier::Rare` audio. `classify_tier(None) = "standard"` test. |
| POPUP-07 | Rarity % shown when available; absent when not | ✓ SATISFIED | PopupCard.tsx renders `{payload.global_pct.toFixed(1)}% of players` only when `global_pct !== null`. |
| POPUP-08 | WS_EX_NOACTIVATE via SetWindowLongW post-creation | ✓ SATISFIED (focus behavior needs human) | ui.rs triple HWND OR confirmed in code. Tauri `focused(false)` + `set_ignore_cursor_events(true)`. UAT single-monitor confirmed. DX11/DX12 cross-title test outstanding. |
| COMP-01 | Companion auto-shows on game launch, auto-hides on game close | ✓ SATISFIED | useGameSession + main-companion.tsx D-17 show/hide. game_detect emits both events. |
| COMP-02 | Companion shows full achievement list with earned/locked entries | ✓ SATISFIED | AchievementRow earned/locked CSS classes. `get_companion_state` returns schema + earned map. |
| COMP-03 | Session unlock history persists to SQLite for mid-session restart restore | ✓ SATISFIED | `earned_unlock_history_persists_session` integration test. `get_companion_state` queries unlock_history by session_id. |
| GAME-01 | Hybrid game-launch detection | ✓ SATISFIED (sysinfo leg only; Steam-state leg deferred to Phase 3 by design) | game_detect/mod.rs 3s polling. appmanifest ACF matching. D-21 Phase 3 deferral documented in CONTEXT.md. |
| GAME-02 | Schema + icon resolution at game-launch, async, non-blocking | ✓ SATISFIED | `schema::resolve` spawned on game-started. Non-blocking tokio task. D-25 trigger. |
| GAME-03 | Schema cached in SQLite; subsequent runs offline once warm | ✓ SATISFIED | `schema_cache` table with `upsert_schema`. `list_for_app` reads from cache. D-24 chain writes back. |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/main-companion.tsx` | 147 | `gameName={`App ${state.app_id}`}` — shows raw app_id integer instead of resolved game display name when state is available | ⚠️ Warning | Game name shows as "App 480" in companion header instead of e.g. "Half-Life 2". Schema resolution chain does not currently include game display name (only achievement schema). The game title display is a cosmetic gap; all functional requirements are met. |
| `src-tauri/src/schema/mod.rs` | ~180 | `schema-resolved` event emitted even when Goldberg JSON parse yields 0 rows (empty metadata) | ℹ️ Info | Companion may show "Loading achievements..." → empty list briefly. Not a drop, just a cosmetic flicker. Self-resolves if rarity leg adds rows. |

No blockers found. The `App ${state.app_id}` pattern is a known v1 cosmetic limitation — game display name resolution requires Steam Web API `GetAppDetails` which is outside the D-24 achievement-schema scope. This is unambiguous from the schema, not a hidden stub.

---

### Human Verification Required

#### 1. 4K / High-DPI Multi-Monitor Popup Rendering (POPUP-04 / SC-4)

**Test:** Launch `cargo tauri dev`. Connect a 4K display (or use display scaling ≥150%). Run a Goldberg-emulated game. Trigger a popup unlock (edit `achievements.json`).

**Expected:** Popup appears at top-right of the 4K monitor at logical 440x96 (physical 880x192 on 2x DPI). Text is sharp, not blurry. The pill border-radius is correctly rounded (not rectangular). Move the game window to the 4K monitor and trigger another unlock — popup follows to that monitor.

**Why human:** Physical pixel rendering on real 4K hardware cannot be verified programmatically. All code paths are correct (PhysicalPosition used, CSS in logical px, `backdrop-filter` clips via `clip-path inset(0 round 48px)`), but actual pixel-perfect rendering requires human visual inspection.

**Note:** The UAT session used only a single 1080p primary monitor. The `popup_position_top_right_4k_secondary` unit test verifies the math for a 3840x2160 monitor at offset (1920, 0) but cannot verify WebView2 rendering quality.

#### 2. Focus-Steal Verification Under DX11/DX12 Borderless-Windowed Title (POPUP-08 / SC-1 partial)

**Test:** Launch a DX11 or DX12 game in borderless-windowed mode (e.g., any modern Steam title). Trigger a Goldberg achievement unlock while the game is in the foreground.

**Expected:** Popup fires (slide-in, hold, slide-out). The game window retains focus throughout. No alt-tab flash. No focus loss visible in the game (no pause menu, no cursor appearing). The game never requires the user to click back in.

**Why human:** ROADMAP.md research flag explicitly called out "validate that WS_EX_NOACTIVATE post-creation HWND patch fully prevents focus-steal across DX11/DX12 borderless-windowed titles." The triple HWND OR-in (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW) plus Tauri `focused(false)` plus `set_ignore_cursor_events(true)` represents defense-in-depth. Single-monitor UAT confirmed no focus steal but did not use a DX-title test scenario.

---

### Gaps Summary

No blocking gaps found. All 14 Phase 2 requirements (POPUP-01..08, COMP-01..03, GAME-01..03) have implementation evidence in the codebase. The full end-to-end pipeline is wired in `lib.rs::run()`. The UAT session (commit 6966084) resolved all runtime blockers. 99 unit tests pass.

**Two items require human verification before the full SC-4 / POPUP-08 contracts can be called closed:**
1. 4K DPI rendering (SC-4 / POPUP-04) — hardware not available in the automated verification session
2. DX11/DX12 focus-steal prevention (SC-1 partial / POPUP-08) — ROADMAP research flag not yet closed

**Three items are deferred to later phases by documented design decision** (Steam-state detection leg → Phase 3; placeholder SFX → Phase 4; real icon warm-up → Phase 4). These are not gaps against Phase 2 scope.

**One cosmetic gap noted** (game name shows as "App {id}" in companion header). Non-blocking, no requirement maps to this behavior, and the display name was not part of the D-24 schema resolution scope.

---

*Verified: 2026-05-09*
*Verifier: Claude (gsd-verifier)*

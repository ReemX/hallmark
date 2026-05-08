# Phase 2: Premium UI — Popup, Companion & Game Session - Context

**Gathered:** 2026-05-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the entire user-facing UI layer on top of the Phase 1 detection pipeline:

1. The signature PS5-style popup overlay (external borderless always-on-top window) that fires within 1s of every unlock event from `run_pipeline`'s sink, with the locked signature look + sound + animation.
2. The popup queue and timing engine — handles burst-of-N unlocks without dropping events, with adaptive compression when queue depth exceeds 5, and the once-per-game 100% celebration as the queue tail.
3. The companion window (separate borderless rounded card) — auto-shows on game launch, auto-hides on game close, lists earned + locked achievements with v1 interactivity (filter / sort / tap-to-expand), persists size + position in SQLite.
4. Hybrid game-launch detection (Steam state + sysinfo + `appmanifest_*.acf`) emitting game-start / game-stop events that drive companion + popup-monitor selection.
5. Schema + icon resolution (display name, description, icon URL, rarity %) cached in SQLite (`schema_cache` + `icon_cache` tables — Phase 2 migration `002_*.sql`); resolves async at game-launch so popups fire without latency after first session with a game.

The mechanism layer (file watcher, SQLite store, RawUnlockEvent pipeline, cross-source dedup) is locked from Phase 1 and is consumed via `run_pipeline`'s `sink` mpsc receiver. Phase 2 does NOT modify Phase 1 detection contracts.

</domain>

<decisions>
## Implementation Decisions

### Popup Signature Look & Sound (POPUP-01, POPUP-06)
- **D-01 Anchor:** Top-right of game's monitor, ~25% from top edge, comfortable margin from screen edge. PS5-reference placement (NOT very-top edge).
- **D-02 Silhouette:** Wide horizontal pill (PS5 stadium shape), achievement icon on left, two-line text (title bold + description) on right.
- **D-03 Material:** Dark glass with translucent backdrop-blur, cool white/cyan accent stroke, white text. "PS5 Pure" direction. Locked — no theme system in v1.
- **D-04 Animation:** Slide-in-from-right with bounce-settle (PS5 spring), ~300ms in / ~300ms out, Framer Motion spring physics. Matches the chosen anchor side.
- **D-05 Sound (standard):** Layered — tonal ding + subtle riser/whoosh, ~900ms total, peak −8dBFS so it sits under voice/dialogue, short attack / fast release. Bundled WAV/OGG, played via `rodio` (kira fallback if WASAPI shared-mode latency >30ms — research flag carries from ROADMAP.md).
- **D-06 Sound (rare-tier per POPUP-06):** Same base + third sparkle/choir stab layer, ~1100ms total, peak −5dBFS. Plays when achievement's global unlock rate is <10% AND rarity data is available.
- **D-07 Tier degradation:** When rarity data is unavailable (POPUP-06 graceful-degrade clause), popup uses the standard look + standard sound. No rare-tier upgrade is applied.

### Popup Queue Timing & 100% Rule (POPUP-02, POPUP-05)
- **D-08 Per-popup timing:** ~300ms slide-in + 3000ms on-screen hold + ~300ms slide-out = ~3.6s per popup at standard pace.
- **D-09 Gap between popups:** 200ms gap (after slide-out completes, before next slide-in begins).
- **D-10 Burst-cap policy:** Adaptive compression. When queue depth >5, compress hold to 1500ms and gap to 0ms. Resume 3000ms / 200ms when depth ≤5. NO events dropped — POPUP-02 holds.
- **D-11 100% celebration trigger:** Fires once per (app_id) ever — persisted as a flag in SQLite `settings` (or a dedicated column on a future `game_completion` row, planner's call). Re-installing Hallmark with a wiped DB would re-trigger once; re-installing the game with the DB intact does not.
- **D-12 100% celebration queue position:** When 100% transitions during a burst, the celebration popup is appended LAST to the queue (per ROADMAP.md SC #2). It does NOT jump ahead. It uses its own variant (richer than standard rare, e.g. extended on-screen hold + 4-layer mix).

### Companion Window UX (COMP-01, COMP-02)
- **D-13 Window chrome:** Borderless rounded card with custom drag region. No native title bar. Custom close button. Tauri `decorations: false`. Consistent visual language with the popup.
- **D-14 Default size:** 480 × 720 portrait card. ~8 achievements visible at default scroll.
- **D-15 Default position:** Centered on the primary monitor on first run (NOT the game monitor — companion is alt-tab surface, not in-game overlay). SQLite persists size + position after first user move.
- **D-16 On-top behavior:** Normal window (NOT always-on-top). Companion is the alt-tab / between-rounds surface. Popup is the in-the-moment surface. Companion does not float over fullscreen gameplay.
- **D-17 Auto-show / auto-hide:** Auto-shows on game-start event from hybrid detection (D-21). Auto-hides on game-stop event. Hide ≠ destroy — window stays in tray / restorable; SQLite `sessions` table records the session continuity (COMP-03).
- **D-18 v1 interactivity:** (a) Filter chip: All / Earned / Locked. (b) Sort toggle: Earned-first (default) / A–Z. (c) Tap a row to expand: full description + unlock timestamp on earned entries. Filter + sort state persists per-game in SQLite.
- **D-19 v1 explicitly skipped:** Text search by title (premature for v1 — few games have >100 achievements). Sort by rarity (rarity data is unreliable per POPUP-07 — would create empty/inconsistent sort orders).
- **D-20 Empty state during schema fetch:** Companion shows the achievement skeleton from SQLite cache if cached; otherwise shows a "loading achievements…" inline state. Earned entries from `unlock_history` are visible immediately even if schema fetch is pending (display api_name as title until display_name resolves, then upgrade in place).

### Hybrid Game-Launch Detection (GAME-01)
- **D-21 Precedence:** Steam state (when Steam is running and reports a current app) is authoritative. Falls back to `sysinfo` polling at 2–3s interval scanning known-game process names + `appmanifest_*.acf` matching for Goldberg-emulated and non-Steam launches. CLAUDE.md "Hybrid game-launch detection" key decision is the locked policy.
- **D-22 Conflict resolution:** If Steam state and sysinfo report different apps simultaneously, Steam state wins (it's the user's authoritative "playing now"). Log the conflict at `tracing::warn` for diagnostics.
- **D-23 Game-window HWND lookup for popup-monitor placement (POPUP-03):** When a game-start event fires, resolve the game's primary HWND via process PID → `EnumWindows` filter, then `MonitorFromWindow` to get the target monitor. Cache HWND for the session; refresh if HWND becomes invalid mid-session. If HWND lookup fails, fall back to primary monitor.

### Schema + Icon Resolution Source (GAME-02, GAME-03, POPUP-07)
- **D-24 Lookup chain:** SQLite `schema_cache` + `icon_cache` (per appid) → local Steam appcache (`%STEAM%\appcache\librarycache` for icons; `%STEAM%\appcache\stats\schema_*.bin` if parseable for schema) → Steam public Web API fallback. Cache the resolved result back into SQLite for offline use thereafter.
- **D-25 Trigger:** Resolution kicks off async on game-start event, NOT on first popup. Popup queue uses whatever's cached at fire-time and upgrades the popup content in place if resolution completes during the 3s hold (Framer Motion-friendly).
- **D-26 Popup fallback when schema unresolved:** Popup shows api_name as title, no description, no icon (or generic placeholder icon), no rarity. POPUP-01 minimum bar (icon + title + description) is degraded gracefully — better to fire a fast-but-bare popup than to delay-or-drop the unlock moment.
- **D-27 Rarity tier threshold (POPUP-06):** Binary tier — global unlock rate <10% qualifies for the rare-tier richer animation + 3-layer sound (D-06). Steam's own UI uses <10% as the rarity cutoff. When rarity data is unavailable, degrades to standard tier (D-07).
- **D-28 Outbound network policy:** Schema/icon Web API fetch IS allowed (PROJECT.md scope explicitly permits "schema/icon fetch and update check"). Telemetry, analytics, crash reports remain out of scope.

### Claude's Discretion
- Exact icon framing within the pill (size, padding, halo on rare-tier) — Claude/researcher to design within "PS5 Pure" direction.
- Exact typography stack (font-family, weights, sizes) — pick a system font + premium fallback chain; no licensed-font dependency in v1.
- Bundled SFX asset format (WAV vs OGG vs FLAC) — researcher's call based on rodio loader behavior + bundle size; both supported by rodio 0.22.
- Icon-disc shape inside the pill — circular vs squircle. Default to circular unless researcher finds it conflicts with achievement icon aspect ratios.
- 100% celebration variant specifics (animation duration, sound stem count) — design iteration after standard + rare variants are in place.
- Whether `schema_cache` and `icon_cache` are one table or two — planner decides based on access pattern (icons are blob-or-path; schema is structured).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level scope and stack
- `.planning/PROJECT.md` — Core value, constraints (signature style locked, external overlay, local-only, customization stance), Key Decisions table.
- `.planning/REQUIREMENTS.md` — Phase 2 requirements POPUP-01..08, COMP-01..03, GAME-01..03; v2 deferred items (POPUP-V2-*, QOL-V2-*).
- `.planning/ROADMAP.md` — Phase 2 entry: success criteria #1–5, research flag (WASAPI latency + WS_EX_NOACTIVATE post-creation HWND patch), UI hint "yes".
- `CLAUDE.md` — Stack pins (Tauri 2.11, React 19, Vite 6, Framer Motion 12, rodio 0.22, sysinfo 0.39, windows-rs 0.58, kira 0.12 fallback). Stack Patterns section: popup overlay window config, companion window config, file watcher pattern, audio playback pattern, process scanner pattern. Steam achievement schema reference. Overlay window technical details. "What NOT to Use" section (FMOD/irrKlang/WMI/ETW excluded).

### Phase 1 contracts to consume (DO NOT modify)
- `src-tauri/src/sources/mod.rs` — `RawUnlockEvent` struct (`app_id: u64`, `ach_api_name: String`, `timestamp: u64`, `source: SourceKind`); `SourceKind` enum (Goldberg only at Phase 2; Phase 3 adds variants); `SourceAdapter` trait shape.
- `src-tauri/src/watcher/mod.rs` — `run_pipeline(raw_rx, store, session_id, sink, dedup_ttl)` is the integration seam. Phase 2's popup-queue consumer subscribes to `sink: mpsc::Receiver<RawUnlockEvent>`. `run_watcher` orchestration is unchanged.
- `src-tauri/src/store/mod.rs` — `SqliteStore::with_conn(closure)` is the public extension API. `record_unlock` returns `bool` (true = inserted, false = dedup hit). `count_unlocks` for diagnostics.
- `src-tauri/src/store/migrations/001_initial.sql` — Existing tables: `unlock_history`, `sessions`, `settings`. Phase 2 adds `002_*.sql` introducing `schema_cache` + `icon_cache` (planner decides one or two tables) plus a `game_completion` row in `settings` (or dedicated table) for 100% celebration flag (D-11).
- `src-tauri/src/lib.rs::run()` — Tauri builder skeleton. Phase 2 extends `setup()` to attach popup-queue + companion + game-detection tokio tasks. `tauri.conf.json` `windows: []` and `security.csp: null` are Phase 1 placeholders — Phase 2 adds popup window, companion window, and a CSP appropriate to the WebView frontend.
- `dist/index.html` — Placeholder; Phase 2 replaces with the real Vite-built React frontend.

### Existing project decisions (already shipped, locked)
- Phase 1 plan summaries (`.planning/phases/01-detection-pipeline-foundation/01-0X-SUMMARY.md`, `01-RESEARCH.md`, `01-VERIFICATION.md`) — for the detection pipeline contracts and known pitfalls only. Phase 2 does NOT plan against these as scope.

### Research flags carrying into plan-phase (HIGH priority)
1. **WASAPI shared-mode rodio latency** on representative gaming hardware — measure end-to-end audio dispatch latency. Threshold: if ≥30ms, fall back to `kira` 0.12 (CLAUDE.md alternatives table). Empirical, must run on hardware.
2. **`WS_EX_NOACTIVATE` post-creation HWND patch** — verify across DX11 + DX12 borderless-windowed titles that the patch fully prevents focus-steal (Tauri issues #7519, #11566 are blocked on this).
3. **Steam Web API anonymous-key requirements** — `ISteamUserStats/GetSchemaForGame` + `GetGlobalAchievementPercentagesForApp` exact key requirements as of 2026-05; whether community-key endpoints exist; rate limits. Researcher must validate before D-24 fallback path is wired.
4. **Steam appcache schema-file empirical format** — does `%STEAM%\appcache` actually contain a parseable schema file, or only `librarycache` icons + `stats\UserGameStats_*.bin` unlock state? If schema is NOT in appcache, D-24's "appcache for schema" leg becomes "appcache for icons; Web API for schema" — that's a research-time refinement, not a re-discussion item.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`run_pipeline` sink consumer slot** (`src-tauri/src/watcher/mod.rs:350`) — already returns kept events to a `mpsc::Sender<RawUnlockEvent>` named `sink`. Phase 2's popup queue is the new consumer of that mpsc receiver. No changes to the producer side.
- **`SqliteStore::with_conn` closure API** (`src-tauri/src/store/mod.rs:103`) — existing helper for typed query helpers; Phase 2's schema/icon cache queries plug in here without exposing the connection mutex publicly.
- **`SqliteStore::open` with idempotent migrations** (`src-tauri/src/store/mod.rs:26`) — appends `002_*.sql` via `include_str!` + `execute_batch`; Phase 1 left a clear pattern (see `INITIAL_MIGRATION_SQL` const).
- **`init_tracing` + `init_tracing_for_tests`** — already initialized; Phase 2 popup/companion modules can `tracing::info!` / `tracing::warn!` for free.
- **`tauri::Builder::default().setup(|_app| ...)` extension point** (`src-tauri/src/lib.rs:67`) — Phase 1 explicitly left this as the place to attach background tokio tasks. Phase 2 adds popup-queue task + game-detection task + companion-state task here.

### Established Patterns
- **One central event-loop, fan-out via mpsc** — Phase 1 set the pattern with `WatcherCore`. Phase 2 popup-queue should follow it: one popup-queue consumer task owns the queue state; rendering is via Tauri IPC events to the popup webview.
- **Sync→async bridges via `blocking_send` / `tokio::spawn_blocking`** — used in `run_watcher` for the notify callback bridge. Same pattern applies for the rodio audio dispatch from the queue task (rodio's API is sync).
- **`Arc<Mutex<...>>` for shared mutable state across tasks** — `SqliteStore::conn: Mutex<Connection>` is the local pattern. Phase 2 queue state + companion-state should follow.
- **`pub mod` ladder** — `lib.rs` exposes `paths`, `sources`, `store`, `watcher`. Phase 2 adds `pub mod ui` (popup + companion frontend bridge), `pub mod audio` (rodio dispatch), `pub mod game_detect` (sysinfo + Steam state hybrid), `pub mod schema` (resolution + cache).
- **Test fixtures in `tests/fixtures` + integration tests in `tests/`** — Phase 1 wrote external integration tests at the workspace root `tests/` dir; Phase 2 follows for end-to-end popup-fires-from-real-event tests.

### Integration Points
- **Phase 1 → Phase 2 seam:** `sink: mpsc::Receiver<RawUnlockEvent>` returned by `run_pipeline`. Phase 2 popup-queue task `recv()`s here; everything else is Phase 2's internal architecture.
- **Tauri builder `setup()` closure** — single insertion point for all Phase 2 background tasks.
- **`tauri.conf.json`** — Phase 2 adds the popup window + companion window definitions, CSP, and any required permissions (e.g. `allowlist` for HTTP fetch to Steam Web API).
- **`dist/`** — Phase 2 introduces the React frontend (Vite build outputs here). `frontendDist: "../dist"` in `tauri.conf.json` already points to it.
- **`Cargo.toml`** — Phase 2 adds `rodio = "0.22"` (with `wav` + `vorbis` features), `sysinfo = "0.39"`, `windows = "0.58"` with `Win32_UI_WindowsAndMessaging` + `Win32_Graphics_Gdi` + `Win32_Foundation` features for HWND/monitor APIs. `package.json` (new) introduces React 19, Vite 6, Framer Motion 12.

</code_context>

<specifics>
## Specific Ideas

- **PS5 placement reference, observed by user from YouTube footage:** popup is anchored at the upper-corner area but not at the very top — roughly a quarter of the way down from the top edge. Side: top-right (D-01). This is the locked Hallmark anchor and overrides any "top-edge" interpretation.
- **PS5 trophy-popup feel** as the reference for D-02 (wide pill), D-03 (dark glass + cool accent), D-04 (slide + bounce-settle), and the timing in D-08 (3s on-screen hold).
- **"Most satisfying and rewarding for the player without majorly disturbing his play session"** — user's framing for the sound character (D-05, D-06). Translated into concrete: layered (not single, for richness), peak attenuated below voice/dialogue (−8dBFS standard / −5dBFS rare), short attack + fast release so it doesn't bleed into game audio.
- **Steam's own <10% rarity threshold** as the v1 tier cutoff (D-27) — matches the convention users already see in Steam's own UI.

</specifics>

<deferred>
## Phase 2 Implementation Notes

### D-21 Steam-state-authoritative leg — DEFERRED to Phase 3

D-21 specifies the hybrid game-launch detection precedence: Steam state (when Steam reports a current app) is authoritative, with sysinfo polling + `appmanifest_*.acf` matching as the fallback. Phase 2 implements ONLY the sysinfo + appmanifest leg.

The Steam-state-authoritative leg requires binary VDF parsing of Steam's `localconfig.vdf` "currently playing" field. RESEARCH.md Section K confirms there is no public Steam IPC for this in 2026 — every available signal requires either (a) binary VDF parsing of local Steam config files (Phase 3 territory, paired with the legit Steam unlock adapter) or (b) third-party hacks excluded by PROJECT.md scope.

**What this means for Phase 2:**
- `game_detect/mod.rs` runs the sysinfo polling loop only; the D-22 conflict-resolution hook is logged at `tracing::trace!` but is a no-op until Phase 3 wires the binary-VDF leg.
- `game-started` events use sysinfo'-derived `app_id` + `pid` (Plan 03 emits the pair so Plan 07's listener can populate `current_pid` for popup monitor placement — POPUP-03 functional routing).
- Goldberg-emulated and non-Steam launches are correctly identified by Phase 2's leg; legitimate Steam-without-Goldberg launches are also identified via `steamapps/common/<installdir>` matching.
- The only behavioral gap vs full D-21 is conflict resolution between Steam'-state and sysinfo when they disagree. Phase 2 logs the disagreement at `tracing::warn` but currently has no Steam-state signal to compare against, so this is a no-op in practice.

## Deferred Ideas

- **Ultra-rare third tier** (e.g. <2% global unlock rate gets a fourth-stem celebratory mix) — captured under POPUP-V2 in REQUIREMENTS.md if a future iteration revisits tiering.
- **Sort-by-rarity in companion** — gated on rarity data being reliable; revisit when a larger sample of `GetGlobalAchievementPercentagesForApp` results is in the cache and we can measure coverage.
- **Companion text search by title** — defer until users hit a game with >100 achievements where filter+sort isn't enough.
- **Click-through always-on-top toggle for companion** — defer; matches future v2 streamer/privacy-mode work (QOL-V2-02).
- **Custom theme system / sound replacement** — explicitly out of scope per PROJECT.md "signature style locked"; tracked there, not re-decided here.
- **Replace synthetic placeholder SFX in `assets/sfx/*.wav` with locked signature multi-layer mix** (D-05 ding+riser+whoosh, D-06 sparkle/choir stab, D-12 4-stem celebration) before any public release. Phase 4 polish task — current Plan 04 ships rust-generated synthetic placeholders so Phase 2 unblocks.

</deferred>

---

*Phase: 2-Premium UI — Popup, Companion & Game Session*
*Context gathered: 2026-05-08*

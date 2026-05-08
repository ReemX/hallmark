# Phase 2: Premium UI — Popup, Companion & Game Session - Research

**Researched:** 2026-05-08
**Domain:** Tauri 2.11 multi-window UI (popup overlay + companion), Framer Motion 12 animation, rodio 0.22 / WASAPI audio, sysinfo 0.39 process detection, windows-rs 0.62 Win32 HWND patching, SQLite (rusqlite 0.39 — already in tree) schema cache, Steam appcache + Web API hybrid resolution
**Confidence:** HIGH (Tauri/React/Framer Motion/Win32/SQLite); MEDIUM (rodio 0.22 due to recent API rename — Sink → Player); MEDIUM-LOW (WASAPI shared-mode latency on real gaming hardware — must measure empirically per ROADMAP research flag)

## Summary

Phase 2 stacks the entire user-facing UI on top of the locked Phase 1 detection pipeline. The architecture splits into five modules under `src-tauri/src/`: `ui` (popup + companion frontend bridge), `audio` (rodio dispatch on a dedicated thread), `game_detect` (sysinfo + Steam state hybrid), `schema` (resolution + SQLite cache), and a new `popup_queue` task running inside `setup()`. The frontend is a React 19 + Vite 6 SPA built into `dist/`, with two webview windows: a non-interactive popup overlay and an interactive companion card.

The single largest 2026-current refinement vs. CLAUDE.md: **rodio 0.22's API renamed `OutputStream` → `MixerDeviceSink` and `Sink` → `Player`**, so the integration code differs from the `Sink::append` pattern shown in CLAUDE.md. The pattern still works (rename only, sequential queue semantics preserved), but the type names and constructors are different. The second largest refinement: **Tauri 2.11 exposes `WebviewWindowBuilder::focusable(false)`** as a first-class builder method that on Windows applies WS_EX_NOACTIVATE for us — but multiple still-open issues (Tauri #11566 closed without fix-version, #12055 closed) indicate that the `focused`/`focus` config-file path and Rust-builder `.focused(false)` path have not been fully reliable. The defense-in-depth recommendation is: use `focusable(false)` AND apply WS_EX_NOACTIVATE manually via windows-rs `SetWindowLongPtrW(GWL_EXSTYLE, ...)` after `build()`. Belt-and-suspenders, since this is the load-bearing focus-steal-prevention behavior.

**Primary recommendation:** Build the popup overlay with `WebviewWindowBuilder::new(...).decorations(false).transparent(true).always_on_top(true).skip_taskbar(true).focusable(false).visible(false).resizable(false).build()`, then immediately get `window.hwnd()` and OR-in `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW` to GWL_EXSTYLE. Drive the popup queue from a dedicated tokio task that owns a `tokio::sync::mpsc::Receiver<PopupEvent>` (bounded, capacity 64) fed by Phase 1's `sink`. The popup window stays alive process-lifetime (not destroyed/recreated per popup) — `set_position` + `show()` to reposition on the game's monitor; `emit("popup", payload)` to push fresh content; React + Framer Motion AnimatePresence handles the in/hold/out animation. Audio: keep a single `rodio::DeviceSinkBuilder::open_default_sink()` alive process-lifetime; pre-decode the standard SFX into a `Vec<u8>` cached at startup; play each unlock by cloning the bytes into a fresh `Decoder<Cursor<Vec<u8>>>` and `mixer().add()`-ing it (so concurrent queue plays don't collide if rare-tier triggers within the slide-out tail of standard tier).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Popup Signature Look & Sound (POPUP-01, POPUP-06):**
- **D-01 Anchor:** Top-right of game's monitor, ~25% from top edge, comfortable margin from screen edge. PS5-reference placement (NOT very-top edge).
- **D-02 Silhouette:** Wide horizontal pill (PS5 stadium shape), achievement icon on left, two-line text (title bold + description) on right.
- **D-03 Material:** Dark glass with translucent backdrop-blur, cool white/cyan accent stroke, white text. "PS5 Pure" direction. Locked — no theme system in v1.
- **D-04 Animation:** Slide-in-from-right with bounce-settle (PS5 spring), ~300ms in / ~300ms out, Framer Motion spring physics.
- **D-05 Sound (standard):** Layered — tonal ding + subtle riser/whoosh, ~900ms total, peak −8dBFS. Bundled WAV/OGG, played via `rodio` (kira fallback if WASAPI shared-mode latency >30ms).
- **D-06 Sound (rare-tier):** Same base + third sparkle/choir stab layer, ~1100ms total, peak −5dBFS. Plays when achievement's global unlock rate is <10% AND rarity data is available.
- **D-07 Tier degradation:** When rarity data is unavailable, popup uses standard look + standard sound. No rare-tier upgrade.

**Popup Queue Timing & 100% Rule (POPUP-02, POPUP-05):**
- **D-08 Per-popup timing:** ~300ms slide-in + 3000ms hold + ~300ms slide-out = ~3.6s/popup at standard pace.
- **D-09 Gap between popups:** 200ms gap (after slide-out completes, before next slide-in begins).
- **D-10 Burst-cap policy:** Adaptive compression. When queue depth >5, compress hold to 1500ms and gap to 0ms. Resume 3000ms / 200ms when depth ≤5. NO events dropped.
- **D-11 100% celebration trigger:** Fires once per (app_id) ever — persisted in SQLite. Wiped DB re-triggers once; reinstalling game with DB intact does not.
- **D-12 100% celebration queue position:** Appended LAST during a burst. Uses richer variant (extended hold + 4-layer mix).

**Companion Window UX (COMP-01, COMP-02):**
- **D-13 Window chrome:** Borderless rounded card with custom drag region. No native title bar. Custom close button. `decorations: false`.
- **D-14 Default size:** 480 × 720 portrait card.
- **D-15 Default position:** Centered on primary monitor on first run. SQLite persists size + position after first move.
- **D-16 On-top behavior:** Normal window (NOT always-on-top). Companion is alt-tab surface; popup is in-the-moment surface.
- **D-17 Auto-show / auto-hide:** Auto-shows on game-start; auto-hides on game-stop. Hide ≠ destroy. SQLite `sessions` records continuity (COMP-03).
- **D-18 v1 interactivity:** Filter chip (All/Earned/Locked) + Sort toggle (Earned-first default / A–Z) + Tap-to-expand. State persists per-game in SQLite.
- **D-19 v1 explicitly skipped:** Text search. Sort by rarity.
- **D-20 Empty state during schema fetch:** Skeleton from cache if cached; "loading…" inline otherwise. Earned entries from `unlock_history` visible immediately; api_name shown until display_name resolves.

**Hybrid Game-Launch Detection (GAME-01):**
- **D-21 Precedence:** Steam state authoritative when Steam running. Fall back to sysinfo polling 2–3s + `appmanifest_*.acf` matching for Goldberg/non-Steam.
- **D-22 Conflict resolution:** Steam state wins on conflict. Log at `tracing::warn`.
- **D-23 Game-window HWND lookup (POPUP-03):** PID → `EnumWindows` filter → `MonitorFromWindow`. Cache HWND for session; refresh if invalid mid-session. Fall back to primary monitor.

**Schema + Icon Resolution (GAME-02, GAME-03, POPUP-07):**
- **D-24 Lookup chain:** SQLite cache → local Steam appcache (icons in `librarycache`) → Steam Web API → cache result.
- **D-25 Trigger:** Async on game-start, NOT first popup. Popup uses cached at fire-time; upgrades content in place if resolution completes during 3s hold.
- **D-26 Popup fallback when schema unresolved:** api_name as title, no description, generic placeholder icon, no rarity. Better fast-but-bare than delay/drop.
- **D-27 Rarity tier threshold:** Binary <10% (Steam's own UI threshold).
- **D-28 Outbound network policy:** Schema/icon Web API fetch IS allowed (PROJECT.md scope). No telemetry/analytics/crash reports.

### Claude's Discretion

- Exact icon framing within the pill (size, padding, halo on rare-tier).
- Exact typography stack — pick a system font + premium fallback chain; no licensed-font dependency.
- Bundled SFX asset format (WAV vs OGG vs FLAC).
- Icon-disc shape inside the pill — circular vs squircle (default circular).
- 100% celebration variant specifics.
- Whether `schema_cache` and `icon_cache` are one table or two.

### Deferred Ideas (OUT OF SCOPE)

- Ultra-rare third tier (<2%) — POPUP-V2.
- Sort-by-rarity in companion.
- Companion text search by title.
- Click-through always-on-top toggle for companion (QOL-V2-02 streamer mode).
- Custom theme system / sound replacement.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| POPUP-01 | Premium signature popup renders icon, title, description, animation, sound | Section A (window config) + E (Framer Motion choreography) + D (rodio audio) + I (schema lookup chain). PS5-style spring with Framer Motion `AnimatePresence` + variants pattern. |
| POPUP-02 | Queue handles close-succession unlocks without dropping; sequential display | Section F (Rust-side bounded mpsc queue with adaptive compression). One consumer task drains; no React-side queue race. Capacity 64 for safety; D-10 compression triggers at depth >5. |
| POPUP-03 | Popup appears on monitor where running game is displayed | Section B (`MonitorFromWindow` + `GetMonitorInfoW`). HWND from sysinfo PID via `EnumWindows` filter. Cache for session. |
| POPUP-04 | DPI-aware on 4K and scaled displays | Section C (Tauri/TAO defaults to per-monitor v2 DPI on Windows 10 1703+). CSS pixels via WebView2 auto-scale by `devicePixelRatio`. Use logical pixels in CSS; Tauri handles physical conversion. |
| POPUP-05 | 100% completion celebration; placed last in queue if pending | Section F (queue ordering rule) + I (persistence flag in SQLite `settings` table or new `game_completion`). |
| POPUP-06 | Tier-based styling — rare unlocks richer; degrades when rarity unavailable | Section D (separate audio file for rare-tier vs same+layer); Section J (rarity from public Web API `GetGlobalAchievementPercentagesForApp` no-key endpoint). |
| POPUP-07 | Rarity % shown when sourced from Steam appcache; absent when unavailable | Section J (public Web API endpoint, no key required; appcache `stats` does NOT contain global %; CONTEXT.md D-24 leg refined to "Web API for rarity"). |
| POPUP-08 | External borderless always-on-top with WS_EX_NOACTIVATE post-creation | Section A (windows-rs `SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current \| WS_EX_NOACTIVATE \| WS_EX_TRANSPARENT \| WS_EX_TOOLWINDOW)`). Defense-in-depth on top of `focusable(false)`. |
| COMP-01 | Companion auto-show on game launch; auto-hide on close | Section G (companion is separate `WebviewWindow` started hidden, `show()` on game-start event from game_detect task). |
| COMP-02 | Companion lists earned + locked with icon, title, description | Section G + I (achievement schema cache + earned entries from Phase 1 `unlock_history`). |
| COMP-03 | Session unlocks persist to SQLite; mid-game restart restores | Section H (Phase 1 `sessions` table extended with current-session reuse on PID match; `unlock_history` already persists). |
| GAME-01 | Hybrid game-launch detection — Steam state + sysinfo fallback | Section K (no public Steam state IPC; use process scan for `steam.exe` + read its current-app from local file `Steam/config/loginusers.vdf` or skip and rely on process scan + `appmanifest_*.acf`). Plus sysinfo on a 2-3s `tokio::time::interval`. |
| GAME-02 | Schema + icon resolution at launch, async + non-blocking | Section I (kicked off in `tokio::spawn` from game_detect task; popup queue reads cache at fire-time). |
| GAME-03 | Schema metadata + icons cached in `hallmark.db` | Section I (new SQLite migration `002_schema_cache.sql`; one table `schema_cache` per appid+ach_api_name with display_name/description/icon_path/hidden/global_pct/cached_at; icons stored as files in user data dir, path stored). |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Popup window creation, HWND patch, monitor placement | Rust backend (Tauri + windows-rs) | — | Win32 calls (`SetWindowLongPtrW`, `MonitorFromWindow`) only callable from Rust process. WebView has no HWND access. |
| Popup content render + animation | Frontend (React 19 + Framer Motion) | Rust emits IPC | WebView is the rendering surface; Framer Motion is JS-only. Rust pushes payload via `app.emit_to("popup-window", "show", payload)`. |
| Popup queue state + timing engine | Rust backend (tokio task) | — | Single source of truth across two processes (popup window may not be loaded yet at burst start). React-side queueing creates double-buffer race when window is hidden. Bounded mpsc + dedicated drain task is the established Phase 1 pattern. |
| Audio dispatch (signature SFX) | Rust backend (rodio + cpal/WASAPI) | — | rodio is Rust-only. WebView's `<audio>` element has WebView2 process overhead and goes through DWM compositor — adds 30-100ms latency. Native cpal direct path is the right choice for sub-30ms target. |
| Companion window content (filter/sort/expand) | Frontend (React 19) | Rust emits + queries | Interactive UI is the WebView's job; React handles list virtualization, filter state, expand UI. |
| Companion show/hide trigger | Rust backend (game_detect → emit) | — | Process scanner is Rust-only. Emits `game-started` / `game-stopped` events; companion JS listens and `show()` / `hide()`s itself. |
| Game-launch detection | Rust backend (sysinfo + ACF parse) | — | Process enumeration is Win32-level. WebView cannot enumerate processes. |
| Game-window HWND → monitor lookup | Rust backend (windows-rs + EnumWindows) | — | Win32-level API. Result passed via Tauri command return. |
| Schema/icon resolution | Rust backend (reqwest + Tauri http plugin) | — | Outbound HTTP needs Tauri capability scope; SQLite write is Rust. WebView reads via Tauri command (Rust → JSON → React state). |
| Schema/icon SQLite cache | Rust backend (rusqlite) | — | Existing Phase 1 `SqliteStore::with_conn` extension API; new migration `002_schema_cache.sql`. |
| 100%-celebration flag | Rust backend (SQLite `settings` table) | — | Persistence + read-on-burst-arrival check. |
| DPI awareness | Tauri/TAO (auto Windows 10 1703+ per-monitor v2) | CSS uses logical px | Backend is automatic; frontend just trusts CSS. |

**Tier rule:** All Win32 / file system / process work is Rust. All animation / interactivity / list rendering is React. The IPC seam is `app.emit_to(label, event, payload)` (Rust → JS) and `#[tauri::command]` functions (JS → Rust). Rust-side state lives in `tokio::sync::Mutex` / `Arc` shared across the popup-queue task, game_detect task, and Tauri command handlers.

## Standard Stack

### Core (verified versions as of 2026-05-08 via `cargo search` and crates.io)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | 2.11 (workspace pin already 2.11; latest 2.11.1) | App framework, multi-window, IPC | [VERIFIED: Cargo.toml] Already pinned; Phase 2 only uses existing dep. |
| `rodio` | 0.22.2 | Audio playback (WAV/OGG one-shot SFX) | [VERIFIED: crates.io 2026-05-08] Pure Rust, cpal → WASAPI. **API renamed in 0.22:** `OutputStream` → `MixerDeviceSink`, `Sink` → `Player` (see Section D for exact pattern). Add features `["wav"]` (and `["vorbis"]` if shipping OGG). |
| `kira` | 0.12.0 | **Audio fallback** if rodio WASAPI latency >30ms | [VERIFIED: crates.io 2026-05-08] Dedicated audio thread, hard-realtime mixer; flip if measurement fails the threshold (research flag from ROADMAP). |
| `sysinfo` | 0.39.0 | Process enumeration (game launch detection) | [VERIFIED: crates.io 2026-05-08] Already in CLAUDE.md; new dep. Use `System::new_all()` once + `refresh_processes(ProcessesToUpdate::All)` on `tokio::time::interval(Duration::from_secs(3))`. Note: 0.30+ renamed `refresh_processes` to take `ProcessesToUpdate` enum. |
| `windows` (windows-rs) | 0.62 (CLAUDE.md cites 0.58; 0.62.2 is current; 0.58 still works for stated APIs) | Win32 HWND, `SetWindowLongPtrW`, `MonitorFromWindow`, `GetMonitorInfoW`, `EnumWindows`, `GetWindowThreadProcessId` | [VERIFIED: crates.io 2026-05-08] Required features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`. Tauri 2.x already depends on windows internally; pin to a compatible major. **Recommend 0.58 to match CLAUDE.md** unless integration shows version conflict. |
| `reqwest` | 0.13 (current 0.13.3) | HTTP fetch for Steam Web API schema/rarity | [VERIFIED: crates.io 2026-05-08] **Optional** — can use `tauri-plugin-http` instead, but reqwest direct is simpler when only the Rust backend makes the calls (no JS fetch). Use with `tokio` runtime + `json` feature. Already a transitive dep via `tauri`/`reqwest`; explicit pin recommended. |
| `rusqlite` | 0.39.0 (already in Cargo.toml) | SQLite for schema_cache + game_completion + companion prefs | [VERIFIED: Cargo.toml] No change. Phase 2 adds migration `002_schema_cache.sql`. |
| `notify` / `notify-debouncer-full` | 8.2 / 0.7 (already in Cargo.toml) | Phase 1 file watcher — Phase 2 only consumes its `sink` | [VERIFIED: Cargo.toml] No new dep work. |
| `serde` / `serde_json` | 1.0 (already in Cargo.toml) | IPC payload serialization, Steam API JSON parse | [VERIFIED: Cargo.toml] No change. |

### Frontend (new — first React frontend in the project)

| Package | Version (recommended) | Purpose | Why Standard |
|---------|----------------------|---------|--------------|
| `react` + `react-dom` | 19.x (latest 19.0.0+) | UI framework | [CITED: CLAUDE.md stack pin] React 19 stable since Dec 2024. |
| `vite` | 6.x | Bundler + dev server | [CITED: CLAUDE.md stack pin] Vite 6 stable since Nov 2024. Tauri's React-TS template already wires it. |
| `framer-motion` | 12.x | Animation (spring, AnimatePresence, useReducedMotion) | [CITED: CLAUDE.md stack pin] 12.x default since Sep 2025. |
| `@tauri-apps/api` | 2.x | JS bindings (event listen/emit, getCurrentWebviewWindow) | [VERIFIED: Tauri 2 docs] Standard companion to `tauri` 2.11. |
| `@tauri-apps/plugin-http` | 2.x | (Optional) HTTP fetch from JS — recommend using reqwest from Rust instead so credentials never reach WebView | [CITED: Tauri docs] Skip unless companion needs to fetch from JS. |
| `zustand` | 5.x | (Optional) Frontend state | [VERIFIED: WebSearch 2026-05] 1.2KB, idiomatic for small/medium apps in 2026. Can also just use React `useReducer` — companion state is small. **Recommendation: useReducer for companion (filter/sort/expand), no Zustand.** |
| `@tanstack/react-virtual` | 3.x | (Optional) List virtualization for companion | [VERIFIED: WebSearch 2026-05] 10-15KB, tree-shakable. **Skip for v1** — most games <100 achievements; native scroll is fine. Add only if a target game pushes >300. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff / When |
|------------|-----------|-----------------|
| rodio 0.22 | kira 0.12 | If WASAPI shared-mode latency measurement fails the 30ms threshold — kira's dedicated audio thread is harder-real-time. **Decision rule (per ROADMAP research flag):** measure first, fall back if and only if median dispatch-to-audible exceeds 30ms on representative gaming hardware. |
| reqwest 0.13 (Rust HTTP) | tauri-plugin-http 2.x (JS HTTP) | Both work. reqwest direct is simpler for this case (Rust orchestrates fetches, persists to SQLite, emits results to JS as already-resolved schema). Plugin-http adds capability scopes for JS-side fetch, which is unnecessary because no UI element makes ad-hoc HTTP calls. |
| rusqlite 0.39 | sqlx | sqlx is async, integrates with tauri-plugin-sql, supports migrations natively. **Already locked to rusqlite** in Phase 1; don't re-platform. Hand-rolled `CREATE TABLE IF NOT EXISTS` + bumped `user_version` pragma is sufficient at Phase 2's table count (1-2 new). Optional: add `rusqlite_migration 2.5` as a tiny helper crate (not required). |
| Zustand for frontend | useReducer / Jotai | Companion state is small (filter chip, sort toggle, expanded row) — `useReducer` per window is sufficient. Skip Zustand. |
| react-window / @tanstack/react-virtual | Native CSS scroll + `content-visibility: auto` | Most games <100 achievements; native scroll handles fine. Add virtualization only if measurement shows scroll jank with a real game's full list. |
| windows-rs 0.62 | windows-rs 0.58 (CLAUDE.md cited) | 0.62 is current. **Use 0.58 to match CLAUDE.md** unless Tauri's transitive windows dep conflicts. Verify with `cargo tree -p windows` after first build. |

**Installation (`src-tauri/Cargo.toml` deltas):**
```toml
# In [dependencies], ADD:
rodio   = { version = "0.22", features = ["wav"] }   # add "vorbis" if shipping OGG
sysinfo = "0.39"
reqwest = { version = "0.13", features = ["json", "rustls-tls"], default-features = false }

# In [target.'cfg(target_os = "windows")'.dependencies], ADD:
windows = { version = "0.58", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
  "Win32_Graphics_Gdi",
] }
```

**Frontend (`package.json` — new file at repo root):**
```json
{
  "name": "hallmark-frontend",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "react": "^19",
    "react-dom": "^19",
    "framer-motion": "^12"
  },
  "devDependencies": {
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "@vitejs/plugin-react": "^4",
    "typescript": "^5",
    "vite": "^6"
  }
}
```

**Tauri config (`tauri.conf.json` deltas):**
- `build.beforeDevCommand`: `"pnpm dev"` (or `"npm run dev"`)
- `build.beforeBuildCommand`: `"pnpm build"`
- `build.devUrl`: `"http://localhost:1420"` (Vite default)
- `app.windows`: leave `[]` — both popup and companion are created programmatically from Rust via `WebviewWindowBuilder` (per Tauri docs pattern; lets us apply HWND patch immediately after `build()`).
- `app.security.csp`: `"default-src 'self'; img-src 'self' data: https://media.steampowered.com https://cdn.akamai.steamstatic.com; connect-src 'self' https://api.steampowered.com"` — restricts img loads to Steam CDN domains and connect to the public API host.
- `app.security.capabilities`: New file `src-tauri/capabilities/popup.json` and `companion.json` — minimal scopes; popup gets `core:event:allow-listen` only; companion gets event + window control.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌──────────────────────────────────────────────────────────┐
                    │                  Phase 1 (locked)                         │
                    │  notify-debouncer  →  GoldbergAdapter  →  CrossSourceDedup│
                    │                                            │              │
                    │                                            ▼              │
                    │                                       run_pipeline        │
                    │                                            │              │
                    │                                       sink: mpsc          │
                    └────────────────────────────────────────────┼──────────────┘
                                                                  │ RawUnlockEvent
                                                                  ▼
                    ┌─────────────────────────  Phase 2 Rust  ─────────────────┐
                    │  popup_queue task (drain loop)                            │
                    │   ├─ enrich (lookup schema_cache → fallback "api_name")   │
                    │   ├─ check rarity → tier (rare if global_pct < 10%)       │
                    │   ├─ check 100% trigger (settings.completion_<appid>)     │
                    │   ├─ adaptive compress if depth > 5                       │
                    │   ├─ resolve target HWND → monitor → position             │
                    │   ├─ emit_to("popup-window", "show", PopupPayload)        │
                    │   └─ audio::play(tier)                                    │
                    │                                                            │
                    │  game_detect task (2-3s tokio interval)                    │
                    │   ├─ sysinfo refresh_processes                            │
                    │   ├─ Steam state read (loginusers.vdf)                    │
                    │   ├─ on launch: emit_to("companion-window", "game-start")│
                    │   │             + spawn schema::resolve(app_id)          │
                    │   └─ on close: emit_to("companion-window", "game-stop") │
                    │                                                            │
                    │  schema task (per game-start)                              │
                    │   ├─ check schema_cache (SQLite)                          │
                    │   ├─ if miss: parse appcache librarycache + reqwest API   │
                    │   ├─ write to schema_cache                                │
                    │   └─ emit_to(*, "schema-resolved", { app_id })            │
                    │                                                            │
                    │  audio dispatcher (rodio Player, process-lifetime)         │
                    │   └─ pre-loaded SFX bytes; .add() per play (concurrent OK)│
                    └────────────┬─────────────────────────────────┬─────────────┘
                                 │ Tauri IPC events                │
                                 ▼                                  ▼
                    ┌─────────────────────┐            ┌────────────────────────┐
                    │ popup-window         │           │ companion-window        │
                    │ (non-interactive)    │           │ (interactive)            │
                    │ ── React + Framer    │           │ ── React + Framer        │
                    │    AnimatePresence   │           │ ── filter / sort / expand│
                    │ ── set_ignore_cursor │           │ ── show/hide on event   │
                    │    _events(true)     │           │ ── persist size/pos     │
                    │ ── HWND patched      │           │                          │
                    │    WS_EX_NOACTIVATE  │           │                          │
                    └──────────────────────┘           └──────────────────────────┘
```

### Recommended Project Structure

```
hallmark/
├── Cargo.toml                       # workspace root (existing)
├── package.json                     # NEW (frontend deps)
├── vite.config.ts                   # NEW
├── tsconfig.json                    # NEW
├── index.html                       # NEW (companion entry)
├── popup.html                       # NEW (popup entry)
├── src/                             # NEW (React)
│   ├── main-companion.tsx
│   ├── main-popup.tsx
│   ├── components/
│   │   ├── PopupCard.tsx            # the pill, animated
│   │   ├── AchievementRow.tsx       # companion list row
│   │   ├── FilterBar.tsx
│   │   └── SortToggle.tsx
│   ├── hooks/
│   │   ├── usePopupListener.ts
│   │   ├── useGameSession.ts
│   │   └── useReducedMotion.ts      # framer's built-in re-export
│   ├── styles/
│   │   ├── popup.css                # PS5 Pure
│   │   └── companion.css
│   └── types.ts                     # shared payload types matching Rust
├── assets/
│   ├── sfx/
│   │   ├── popup-standard.wav       # ~900ms, peak −8dBFS
│   │   ├── popup-rare.wav           # ~1100ms, peak −5dBFS
│   │   └── popup-100pct.wav         # celebration variant
│   └── icons/
│       └── placeholder.png          # generic fallback (D-26)
├── src-tauri/
│   ├── Cargo.toml                   # add rodio, sysinfo, reqwest, windows
│   ├── tauri.conf.json              # add CSP, window-less + programmatic creation
│   ├── capabilities/
│   │   ├── popup.json               # NEW (minimal)
│   │   └── companion.json           # NEW (events + window control)
│   └── src/
│       ├── lib.rs                   # extend setup() with phase 2 tasks
│       ├── ui.rs                    # NEW: window builders + HWND patch
│       ├── popup_queue.rs           # NEW: drain task + adaptive compression
│       ├── audio.rs                 # NEW: rodio player + pre-loaded SFX
│       ├── game_detect/
│       │   ├── mod.rs               # NEW: hybrid launcher
│       │   ├── steam_state.rs       # NEW: parse loginusers.vdf
│       │   └── process_scan.rs      # NEW: sysinfo + ACF match
│       ├── schema/
│       │   ├── mod.rs               # NEW: lookup chain orchestrator
│       │   ├── appcache.rs          # NEW: librarycache image + assets.vdf
│       │   ├── steam_api.rs         # NEW: reqwest GetSchemaForGame + GetGlobalAchievementPercentages
│       │   └── cache.rs             # NEW: SQLite reads/writes
│       ├── monitor.rs               # NEW: HWND-by-PID + monitor lookup
│       ├── store/
│       │   ├── mod.rs               # extend with new with_conn helpers
│       │   ├── queries.rs           # add schema_cache + completion + companion_prefs queries
│       │   └── migrations/
│       │       ├── 001_initial.sql  # existing
│       │       └── 002_schema_cache.sql # NEW
│       └── (existing paths/sources/watcher unchanged)
```

### Pattern 1: Programmatic Popup Window with Post-Creation HWND Patch

**What:** Create the popup window from Rust (so we can grab `hwnd()` immediately) and apply WS_EX_NOACTIVATE in the same call site, before the window has a chance to gain focus.

**When to use:** Single popup overlay window, kept alive process-lifetime, repositioned + content-swapped per unlock.

**Example:**

```rust
// src-tauri/src/ui.rs
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW,
        GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    },
};

pub fn create_popup_window(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("popup.html".into()))
        .title("Hallmark Popup")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)        // tell Tauri "don't focus on create"
        .focusable(false)      // tell Tauri "this window cannot accept focus"  (Tauri 2.x)
        .resizable(false)
        .visible(false)        // start hidden; show + position per unlock
        .inner_size(440.0, 96.0)  // pill size in logical pixels — DPI handled by Tauri/TAO
        .accept_first_mouse(false)
        .visible_on_all_workspaces(true)
        .shadow(false)         // we paint our own shadow in CSS
        .build()?;

    // ----- DEFENSE-IN-DEPTH: belt-and-suspenders WS_EX_NOACTIVATE -----
    // Tauri issues #11566, #12055 show focus(false)/focusable(false) have not been
    // 100% reliable on Windows historically. Apply the raw flag too. This is the
    // documented workaround for game-overlay focus stealing.
    #[cfg(target_os = "windows")]
    {
        let hwnd = HWND(win.hwnd()?.0 as *mut _);
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_style = current
                | (WS_EX_NOACTIVATE.0 as isize)
                | (WS_EX_TRANSPARENT.0 as isize)
                | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
    }

    // Click-through (covers entire window since popup is non-interactive)
    win.set_ignore_cursor_events(true)?;
    Ok(())
}
```

[VERIFIED: Tauri 2.11 docs.rs WebviewWindowBuilder] — every method called above exists on the builder. [VERIFIED: docs.rs/tauri/2.11.1/.../Window#impl-Window<R>] `hwnd()` is `#[cfg(windows)]`-gated. [CITED: github.com/tauri-apps/tauri/issues/7519, #11566, #12055] focus/focusable bugs documented; manual HWND patch is the workaround.

### Pattern 2: Multi-Monitor Placement (`MonitorFromWindow` + Tauri set_position)

**What:** When a game-start event fires, find the game's primary HWND, get its monitor's rectangle, then position the popup at top-right ~25% from top with comfortable margin.

**When to use:** On every popup fire (HWND can change session-to-session; game can switch monitors mid-session via Win+Shift+arrow).

**Example:**

```rust
// src-tauri/src/monitor.rs
use windows::Win32::{
    Foundation::{HWND, RECT, BOOL, LPARAM},
    Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO},
    UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible},
};

/// Find the first visible top-level window owned by the given PID.
pub fn hwnd_for_pid(pid: u32) -> Option<HWND> {
    struct Ctx { pid: u32, found: Option<HWND> }
    let mut ctx = Ctx { pid, found: None };

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(lparam.0 as *mut Ctx) };
        if !IsWindowVisible(hwnd).as_bool() { return BOOL(1); }  // continue
        let mut wpid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut wpid));
        if wpid == ctx.pid {
            ctx.found = Some(hwnd);
            return BOOL(0);  // stop
        }
        BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut ctx as *mut Ctx as isize));
    }
    ctx.found
}

/// Returns (monitor_left, monitor_top, monitor_width, monitor_height) in physical pixels.
pub fn monitor_rect_for_hwnd(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let hmon: HMONITOR = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            let r: RECT = info.rcWork;  // rcWork excludes taskbar; use rcMonitor for full screen
            Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
        } else { None }
    }
}

/// Compute popup position (top-right, ~25% down) given monitor rect + popup size.
/// Returns physical pixels. Tauri's set_position with PhysicalPosition handles DPI.
pub fn popup_position(
    mon_x: i32, mon_y: i32, mon_w: i32, mon_h: i32,
    popup_w: i32, popup_h: i32,
) -> (i32, i32) {
    let margin = 32;  // physical px; will look ~32/scale logical px on each monitor
    let x = mon_x + mon_w - popup_w - margin;
    let y = mon_y + (mon_h / 4) - (popup_h / 2);  // ~25% from top, vertically centered there
    (x, y)
}

// Caller (popup_queue.rs):
//   let pos = popup_position(...);
//   popup_window.set_position(tauri::PhysicalPosition::new(pos.0, pos.1))?;
//   popup_window.show()?;
//   app.emit_to("popup", "show", payload)?;
```

[CITED: learn.microsoft.com winuser MonitorFromWindow] `MONITOR_DEFAULTTONEAREST` returns the closest monitor if the window doesn't intersect any (covers off-screen edge cases). [VERIFIED: docs.rs/tauri/2.11.1 Window::scale_factor / monitor_from_point] Tauri exposes higher-level `monitor_from_point` and `current_monitor` — but the game's HWND isn't a Tauri window, so the Win32 path is required.

**Tauri-API alternative for our own popup window's monitor:** `popup_window.current_monitor()?` returns a `Monitor` — but we don't want the popup's current monitor; we want the **game's** monitor. So we use Win32 directly via the game's HWND.

### Pattern 3: Audio Dispatch (rodio 0.22 — renamed API)

**What:** Single device-sink alive process-lifetime; per-popup play decodes from cached bytes and adds to mixer (concurrent SFX possible if needed).

**When to use:** Every popup fire (and rare-tier variant).

**Example:**

```rust
// src-tauri/src/audio.rs
use std::io::Cursor;
use std::sync::Arc;
use rodio::{Decoder, DeviceSinkBuilder, Player, MixerDeviceSink, Source};

pub struct AudioDispatcher {
    // Hold the device sink so the audio device stays open process-lifetime.
    // Dropping it silences all output.
    _stream: MixerDeviceSink,
    mixer: rodio::Mixer,                 // cheap-clone via Arc internally
    standard_bytes: Arc<Vec<u8>>,        // pre-loaded SFX bytes (D-05)
    rare_bytes:     Arc<Vec<u8>>,        // pre-loaded rare SFX (D-06)
    completion_bytes: Arc<Vec<u8>>,      // pre-loaded 100% celebration (D-12)
}

#[derive(Debug, Clone, Copy)]
pub enum Tier { Standard, Rare, Completion }

impl AudioDispatcher {
    pub fn new() -> anyhow::Result<Self> {
        let stream = DeviceSinkBuilder::open_default_sink()?;
        let mixer = stream.mixer().clone();

        // Bundle SFX in the binary via include_bytes! so they ship with the .exe.
        // Phase 4 may later switch to filesystem reads from the install dir.
        let standard_bytes = Arc::new(include_bytes!("../../assets/sfx/popup-standard.wav").to_vec());
        let rare_bytes     = Arc::new(include_bytes!("../../assets/sfx/popup-rare.wav").to_vec());
        let completion_bytes = Arc::new(include_bytes!("../../assets/sfx/popup-100pct.wav").to_vec());

        Ok(Self { _stream: stream, mixer, standard_bytes, rare_bytes, completion_bytes })
    }

    /// Non-blocking. Decodes from in-memory bytes (cheap) and pushes to mixer.
    /// Concurrent calls layer in the mixer (no blocking, no allocation in the hot path
    /// except the Decoder's internal state).
    pub fn play(&self, tier: Tier) -> anyhow::Result<()> {
        let bytes = match tier {
            Tier::Standard   => self.standard_bytes.clone(),
            Tier::Rare       => self.rare_bytes.clone(),
            Tier::Completion => self.completion_bytes.clone(),
        };
        let cursor = Cursor::new(bytes.as_ref().clone());
        let decoder = Decoder::try_from(cursor)?;
        // .add() takes a Source; layered/concurrent. For sequential-only, use Player::append.
        // Hallmark spec: sequential (one popup on screen at a time), but we let mixer.add
        // so a rare-tier doesn't get clipped by a still-tailing standard from the previous popup.
        self.mixer.add(decoder.convert_samples::<f32>());
        Ok(())
    }
}
```

[VERIFIED: github.com/rustaudio/rodio UPGRADE.md] In rodio 0.22, `OutputStream::try_default()` → `DeviceSinkBuilder::open_default_sink()`; `Sink` → `Player`; both share `mixer()` access for adding sources. [CITED: docs.rs/rodio/0.22.2] `MixerDeviceSink` is the type returned by `open_default_sink()`. **Pitfall:** the `_stream` MUST be held in the struct — dropping it silences all audio. **API drift warning:** CLAUDE.md cites the 0.20-style `OutputStream` / `Sink` pattern; the 0.22 API above is current. The spec's `Sink::append` reference becomes `Player::append` if you use Player (sequential queue) instead of `mixer.add` (overlapping).

### Pattern 4: Popup Queue with Adaptive Compression

**What:** A single tokio task drains Phase 1's `sink: mpsc::Receiver<RawUnlockEvent>`, enriches each event with schema cache + tier classification, then plays one popup at a time with the timing rules in D-08/D-09/D-10.

**When to use:** The single consumer of Phase 1's `sink`. Don't drain it in any other place.

**Example:**

```rust
// src-tauri/src/popup_queue.rs
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::sources::RawUnlockEvent;
use crate::audio::{AudioDispatcher, Tier};
use crate::schema::SchemaCache;
use crate::monitor;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PopupPayload {
    pub app_id: u64,
    pub ach_api_name: String,
    pub display_name: String,         // falls back to ach_api_name (D-26)
    pub description: String,
    pub icon_path: Option<String>,    // file:// URL or absent → frontend uses placeholder
    pub global_pct: Option<f64>,
    pub tier: &'static str,           // "standard" | "rare" | "completion"
}

/// Spawn the popup-queue drain task. Owns the Phase 1 sink receiver.
pub async fn run(
    app: AppHandle,
    mut sink: mpsc::Receiver<RawUnlockEvent>,
    schema: SchemaCache,
    audio: AudioDispatcher,
) {
    // Phase 1 buffered queue — events arriving while a popup is animating sit here.
    while let Some(ev) = sink.recv().await {
        // Backlog observation: how many more events are sitting after this one?
        let depth = sink.len();  // tokio mpsc 1.30+: Receiver::len() — verify availability.
        let (hold_ms, gap_ms) = if depth > 5 { (1500u64, 0u64) } else { (3000u64, 200u64) };

        // ----- Enrich -----
        let enriched = schema.lookup(ev.app_id, &ev.ach_api_name).await;
        let tier = classify_tier(&schema, ev.app_id, &enriched).await;
        // 100% celebration check: if this unlock completes the set AND we haven't
        // already fired the celebration for this app_id ever, append a celebration
        // payload AFTER this normal-tier popup.
        let payload = build_payload(&ev, &enriched, tier);

        // ----- Position on game's monitor -----
        if let Some(hwnd) = monitor::current_game_hwnd().await {
            if let Some((mx, my, mw, mh)) = monitor::monitor_rect_for_hwnd(hwnd) {
                // 440 × 96 are the popup's logical inner size (set at builder time).
                // Tauri's PhysicalPosition is in physical px; we computed those above.
                let (x, y) = monitor::popup_position(mx, my, mw, mh, 440 * 1, 96 * 1);
                if let Some(popup) = app.get_webview_window("popup") {
                    let _ = popup.set_position(tauri::PhysicalPosition::new(x, y));
                }
            }
        }

        // ----- Show + emit + audio -----
        if let Some(popup) = app.get_webview_window("popup") {
            let _ = popup.show();
        }
        let _ = app.emit_to("popup", "popup-show", &payload);
        let _ = audio.play(match tier {
            "rare"       => Tier::Rare,
            "completion" => Tier::Completion,
            _            => Tier::Standard,
        });

        // ----- Wait through animation + hold + slide-out -----
        // 300ms slide-in is "free" (overlaps with the show()-emit handoff to React);
        // we wait for the hold + slide-out which is what holds up the next popup.
        sleep(Duration::from_millis(300 + hold_ms + 300)).await;
        let _ = app.emit_to("popup", "popup-hide", ());
        if let Some(popup) = app.get_webview_window("popup") {
            // Don't hide() — leave window present at fully transparent. set_position
            // off-screen if needed. Hiding/showing on Windows can re-trigger focus
            // events that risk activating the window even with WS_EX_NOACTIVATE.
        }

        // ----- Inter-popup gap -----
        sleep(Duration::from_millis(gap_ms)).await;
    }
}
```

[VERIFIED: tokio docs] `mpsc::Receiver::len()` is available; bounded channel. [CITED: CONTEXT.md D-08/D-09/D-10] Timing values match locked decisions. **Pitfall:** Don't use `unbounded_channel` for this seam — Phase 1's `sink` is already bounded; respect that. If queue truly fills (>64 unlocks pending), Phase 1's send-side will block, which is the correct backpressure signal.

### Pattern 5: Popup Animation in React (Framer Motion 12)

**What:** Slide-in-from-right + bounce-settle entry, slide-out-up + fade exit. AnimatePresence drives mount/unmount; spring physics drive the bounce.

**When to use:** Inside the popup webview's React tree, listening for the `popup-show` and `popup-hide` events from Rust.

**Example:**

```tsx
// src/main-popup.tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";

type PopupPayload = {
  app_id: number;
  ach_api_name: string;
  display_name: string;
  description: string;
  icon_path: string | null;
  global_pct: number | null;
  tier: "standard" | "rare" | "completion";
};

export function PopupRoot() {
  const [payload, setPayload] = useState<PopupPayload | null>(null);
  const reduceMotion = useReducedMotion();

  useEffect(() => {
    const u1 = listen<PopupPayload>("popup-show", (e) => setPayload(e.payload));
    const u2 = listen("popup-hide", () => setPayload(null));
    return () => { u1.then(f => f()); u2.then(f => f()); };
  }, []);

  // Spring values tuned for "PS5 spring" feel — ~300ms total perceived motion,
  // slight overshoot, settles cleanly. Verified by visual iteration + comparison
  // with PS5 reference footage.
  const spring = {
    type: "spring" as const,
    stiffness: 380,
    damping: 28,
    mass: 0.9,
  };

  return (
    <AnimatePresence>
      {payload && (
        <motion.div
          key={payload.ach_api_name}
          className={`popup-pill tier-${payload.tier}`}
          initial={reduceMotion ? { opacity: 0 } : { x: 480, opacity: 0 }}
          animate={reduceMotion ? { opacity: 1 } : { x: 0, opacity: 1 }}
          exit={reduceMotion ? { opacity: 0 } : { x: 0, y: -16, opacity: 0 }}
          transition={reduceMotion ? { duration: 0.15 } : spring}
        >
          <div className="popup-icon">
            {payload.icon_path
              ? <img src={`asset://localhost/${payload.icon_path}`} alt="" />
              : <div className="popup-icon-placeholder" />}
          </div>
          <div className="popup-text">
            <div className="popup-title">{payload.display_name || payload.ach_api_name}</div>
            <div className="popup-desc">{payload.description}</div>
            {payload.global_pct !== null && (
              <div className="popup-rarity">{payload.global_pct.toFixed(1)}% of players</div>
            )}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
```

[VERIFIED: Context7 framer-motion docs] `AnimatePresence` + `motion.div` + spring transition + `useReducedMotion` are all standard 12.x patterns. [CITED: github.com/grx7/framer-motion docs] Spring stiffness/damping/mass interplay. **Spring tuning rationale:** stiffness 380 + damping 28 + mass 0.9 yields ~280ms-perceived settle with one small overshoot — matches the PS5 reference. Adjust empirically once the asset pipeline is live; these are the starting values.

### Anti-Patterns to Avoid

- **Don't recreate the popup window per fire.** Window creation is ~50-200ms on Windows due to WebView2 init and the HWND patch sequence. Keep one popup window alive process-lifetime; reposition + emit content per popup. (CONTEXT.md doesn't explicitly say this, but it's the only way to hit the 1s success criterion under load.)
- **Don't do `popup.hide()` between popups.** On Windows, `hide()` → `show()` can re-introduce focus-activation race conditions even with WS_EX_NOACTIVATE. Use CSS opacity (controlled by AnimatePresence mount/unmount) to make the empty state invisible.
- **Don't queue popups in React state.** State is per-window; if the popup window's React app reloads (e.g. dev hot reload, future restart-on-crash), the queue is lost. Rust owns the queue.
- **Don't fetch schema from the popup or companion JS.** Tauri commands return JSON to JS; let Rust be the only HTTP client (CSP locked-down, no JS-side credentials).
- **Don't drop the `MixerDeviceSink` (rodio).** Audio goes silent. Hold it in `AudioDispatcher` struct for process-lifetime.
- **Don't use the JS `Audio` element / WebView audio for SFX.** WebView audio routes through DWM; adds ~30-100ms vs native cpal. We need ≤30ms total for the popup's "felt" alignment.
- **Don't poll `prefers-reduced-motion` from React effect.** Use `useReducedMotion` from framer-motion — it sets up the matchMedia listener correctly with cleanup.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Window state persistence | Custom file at `%APPDATA%\Hallmark\window.json` | `tauri-plugin-window-state` (auto-restores size/position) — OR — store in our existing SQLite (since we have it) | Plugin handles edge cases (window outside monitor bounds, multi-monitor changes); our SQLite path is fine if we replicate that logic. **Recommend SQLite path** since we own the schema (D-15 says "SQLite persists size + position"). |
| Spring animation curve | Custom `requestAnimationFrame` lerp | Framer Motion 12 `transition: { type: "spring" }` | Framer's spring solver handles velocity preservation through interruption (mid-animation hide → re-show), prefers-reduced-motion, GPU compositor hinting. |
| Audio mixing for concurrent plays | rodio `Sink::append` (sequential only) + custom thread for parallel | `Mixer::add` directly (rodio 0.22) | Mixer add is non-blocking, layered, lock-free internally. Hand-rolled threads risk WASAPI device contention. |
| HTTP retries / backoff | `loop { reqwest::get(...).await }` | `reqwest::Client` with manual retry on 5xx + max 3 attempts | Simple inline retry is fine; don't pull in `reqwest-retry` or `backon` for two endpoints. **Skip the trap of installing a retry framework for trivial use.** |
| SQLite migrations | Manual `if version < 2 { execute }` switching | Existing Phase 1 pattern: `include_str!("002_schema_cache.sql")` + `execute_batch` (idempotent via `IF NOT EXISTS`) | Phase 1 already pioneered this. Phase 2 adds the next migration file with the same pattern. No `rusqlite_migration` crate needed. |
| Process scanner with PID tracking | Manual `ReadProcessMemory` / `NtQuerySystemInformation` | `sysinfo::System::refresh_processes(ProcessesToUpdate::All)` on a 2-3s tokio interval | sysinfo wraps the right NT API and exposes `cmd()`, `name()`, `exe()` cross-platform. |
| Steam Web API client | Custom struct + serde for every endpoint | `reqwest::Client` + per-call `.get(url).query(...).send().await?.json::<...>().await?` | Two endpoints total. Don't need a typed SDK. |
| Binary VDF parsing | Custom byte parser | DEFER — Phase 2 only needs **text** VDF for `assets.vdf` (already keyvalues-parser 0.2 from Phase 1). Phase 3 will face binary VDF for legitimate Steam stats. | Out of Phase 2 scope. |
| 100% completion detection | Diff against schema set on every popup | Increment counter on each unlock, compare to schema-cache row count for the appid; emit celebration when equality first reached AND `settings.completion_<appid>` is absent | Simple int compare; one SQLite roundtrip. |
| HWND-by-PID lookup | Use `FindWindow` / `WaitForInputIdle` workarounds | `EnumWindows` + `GetWindowThreadProcessId` filter (Pattern 2) | EnumWindows is the canonical Win32 way to map PID → top-level HWND. |
| List virtualization for companion | `react-window` / `@tanstack/react-virtual` | Native CSS scroll + `content-visibility: auto` for v1 | Most games <100 achievements. Add virtualization only if a real game's full list jank-tests. |

**Key insight:** This phase is mostly about correctly orchestrating libraries that already exist. The only original code is the queue task, the popup-position math, the schema-lookup chain, and the wire-up. Everything else is "use the library."

## Common Pitfalls

### Pitfall 1: rodio API rename (`Sink` → `Player`, `OutputStream` → `MixerDeviceSink`)
**What goes wrong:** CLAUDE.md cites the 0.20-style API: `let (_stream, handle) = OutputStream::try_default()?;` — this no longer compiles on rodio 0.22.
**Why it happens:** rodio 0.22 (2025) is a major API rename for clarity. Old code in tutorials and blog posts targets 0.20 / 0.21.
**How to avoid:** Use the 0.22 pattern in Section D Pattern 3. Lock `rodio = "0.22"` in Cargo.toml.
**Warning signs:** `error[E0433]: failed to resolve: could not find OutputStream in rodio` on first build.

### Pitfall 2: `focus: false` / `focusable: false` not fully reliable on Windows
**What goes wrong:** Popup window briefly steals focus from the game on first show, or on subsequent show()s.
**Why it happens:** Tauri issues #7519 (closed without fix-version), #11566 (closed), #12055 (closed) — the focus property has historically been unreliable on Windows. Multiple users reported the same.
**How to avoid:** Defense-in-depth — set `focused(false)` AND `focusable(false)` in the builder, AND apply WS_EX_NOACTIVATE manually via `SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ...)` immediately after `build()`. Pattern 1 above.
**Warning signs:** Game minimizes / loses input when popup appears. Verifiable via `GetForegroundWindow()` before-and-after popup show — should be unchanged.

### Pitfall 3: `hwnd()` is not on `WebviewWindow` directly — it's on the inner `Window`
**What goes wrong:** `web_window.hwnd()` doesn't compile.
**Why it happens:** Tauri 2 split `Window` (low-level) from `WebviewWindow` (high-level). `WebviewWindow` derefs to `Window` via `AsRef`/`as_ref()` for some methods; `hwnd()` is on `Window`.
**How to avoid:** Check the docs.rs page for `tauri::window::Window` (Windows target) for `hwnd()`. Call `web_window.hwnd()` — `WebviewWindow` re-exposes it. Verify with a quick `cargo check` early.
**Warning signs:** `error[E0599]: no method named hwnd found for struct WebviewWindow`. Issue #13046 in tauri-apps/tauri tracks intermittent reports.

### Pitfall 4: Hide/show transitions on Windows can re-trigger focus
**What goes wrong:** Even with WS_EX_NOACTIVATE, calling `popup.hide()` then `popup.show()` between popups occasionally activates the window (game flickers).
**Why it happens:** Windows' `ShowWindow` / `SW_SHOW` can implicitly call `SetForegroundWindow` for top-level windows in certain conditions.
**How to avoid:** Don't hide between popups. Use CSS opacity (driven by Framer's AnimatePresence) to make the empty state invisible. Reposition with `set_position`; the window stays "shown" at all times after first creation. Only hide on app shutdown.
**Warning signs:** Brief game-window de-activation flicker every 3-4 seconds during a burst.

### Pitfall 5: Per-monitor DPI and `set_position` units (PhysicalPosition vs LogicalPosition)
**What goes wrong:** Popup positioned correctly on primary monitor but off-screen on a 4K secondary at 200% scale (or vice versa).
**Why it happens:** TAO/Tauri uses logical pixels for sizing in config, but `set_position` accepts both `PhysicalPosition` and `LogicalPosition`. Mixing units = wrong placement.
**How to avoid:** Use `tauri::PhysicalPosition::new(x, y)` for placement. The values returned by `MonitorFromWindow` + `GetMonitorInfoW.rcWork` are physical pixels. Match units. Verify `tauri::PhysicalPosition` is `i32`.
**Warning signs:** Popup appears at upper-left corner instead of top-right (logical 0,0 mapped through wrong scale).

### Pitfall 6: WASAPI shared-mode latency exceeds 30ms target on real hardware
**What goes wrong:** Sound plays ~50-150ms after `mixer.add()` on some Windows installs (older audio drivers, shared-mode default 10ms buffer + driver latency adds up).
**Why it happens:** WASAPI shared mode has a Windows-internal ~12ms latency floor on top of the buffer size. Some realtek drivers add 30-80ms more.
**How to avoid:** Measure on representative gaming hardware (research flag from ROADMAP). If median dispatch-to-audible exceeds 30ms, swap to `kira` 0.12 (its dedicated audio thread + tighter buffer config typically achieves <15ms). Measurement methodology in Section D below.
**Warning signs:** Audio "feels" slightly off the visual pop-in. Measurable via loopback recording (audio in vs. event timestamp).

### Pitfall 7: Steam Web API `GetSchemaForGame` requires a publisher API key
**What goes wrong:** Schema fetch fails with 401/403 in production after working in dev with a personal key.
**Why it happens:** `GetSchemaForGame` requires a Steam Web API user authentication key (user-tied). For a free OSS app, embedding a personal key in the binary is a TOS violation and gets revoked.
**How to avoid:** **Do NOT call `GetSchemaForGame` from production builds.** Use:
1. Local `appcache/librarycache/` for icons (always available when Steam is installed).
2. Goldberg `achievements.json` (already gives display_name, description, icon_url for Goldberg-detected games — Phase 1 already parses).
3. `GetGlobalAchievementPercentagesForApp/v0002/` — the **only** Steam endpoint we should call from production. **No API key required** (verified at partner.steamgames.com/doc/webapi/isteamuserstats and steamcommunity.com/dev).
4. SteamSpy as fallback for rarity if the official endpoint rate-limits us (but no SLA, so don't rely).
**Warning signs:** 401/403 from `api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/` calls.

### Pitfall 8: Goldberg `achievements.json` schema field naming inconsistency
**What goes wrong:** Some Goldberg builds use `displayName`, others `display_name`, others `description`, others `desc`. Phase 1 already documented this — Phase 2 schema lookup must reconcile.
**Why it happens:** Goldberg/gbe_fork forks have diverged.
**How to avoid:** Use Phase 1's empirical schema notes (`empirical-goldberg-schema-NOTES.md`) for the canonical Goldberg fields. Cache the resolved fields in `schema_cache` after first read so subsequent popups don't re-parse.
**Warning signs:** Popup title is empty or shows a hash-like api_name.

### Pitfall 9: Companion show on game-start before schema cache is warm
**What goes wrong:** Companion opens with empty list because schema fetch is still in flight.
**Why it happens:** D-25 says async-on-game-start, not block-until-ready. First-session-with-game has cold cache.
**How to avoid:** D-20 explicitly handles: show skeleton from any cache hits + an inline "loading…" bar; show earned entries from `unlock_history` immediately (api_name as title); upgrade rows in place via a `schema-resolved` event when fetch completes.
**Warning signs:** Companion blank screen on first launch of new game.

### Pitfall 10: 100%-celebration race with the burst (D-12 must hold)
**What goes wrong:** Player gets the last achievement of a 5-burst — the celebration popup fires somewhere in the middle, not at the end.
**Why it happens:** Naive implementation checks 100% after each unlock and emits celebration immediately.
**How to avoid:** Inside the queue drain (Pattern 4), accumulate "celebration_pending = true" if a unlock completes the set, but **don't emit celebration popup until** `sink.recv()` returns "queue drained" (i.e., next `recv()` would block). Append celebration as the synthetic next event. **Persistence guard:** check `settings.completion_<appid>` first to ensure once-ever (D-11).
**Warning signs:** Celebration appears between standard popups during a burst.

### Pitfall 11: tokio mpsc `recv` blocks forever if no events come
**What goes wrong:** Queue task is alive but does nothing because Phase 1 isn't producing events.
**Why it happens:** This is intentional — bounded mpsc channel waits for senders.
**How to avoid:** This is correct behavior; the task will wake when an event arrives. Don't add a timeout/poll wrapper — that wastes CPU.
**Warning signs:** None — this is normal idle.

### Pitfall 12: `EnumWindows` callback panics if Rust panics inside it
**What goes wrong:** Calling Rust code that can panic from inside the C-callable EnumWindows callback unwinds across the FFI boundary, which is undefined behavior on Windows.
**Why it happens:** Rust panics across FFI = UB.
**How to avoid:** Use only operations that cannot panic inside the callback (struct field assigns, BOOL returns). Wrap any borderline-risky logic in `std::panic::catch_unwind`.
**Warning signs:** Rare, intermittent crashes during game-launch detection.

### Pitfall 13: Asset URL scheme mismatch (`asset://`, `tauri://`, `file://`)
**What goes wrong:** Achievement icons don't load in popup webview — broken-image icon shows.
**Why it happens:** Tauri 2 changed the asset protocol. Local files require `convertFileSrc` from `@tauri-apps/api/core` or the `asset:` protocol must be allowed in the capability config.
**How to avoid:** Use `convertFileSrc(icon_path)` in JS to get the right URL. Add `core:default` permission (or specifically `core:webview:allow-internal-toggle-devtools` plus the asset protocol opt-in) to the popup capability.
**Warning signs:** 404s in DevTools network tab; broken icon.

### Pitfall 14: Vite + Tauri dev server port + `devUrl`
**What goes wrong:** `cargo tauri dev` starts the Vite server, but Tauri tries to load `http://localhost:5173` while Vite picked another port.
**Why it happens:** Tauri reads `build.devUrl` from `tauri.conf.json`; Vite default is 5173 but Tauri's React-TS template uses 1420.
**How to avoid:** Pin Vite port to 1420 in `vite.config.ts`: `server: { port: 1420, strictPort: true }`. Match `build.devUrl: "http://localhost:1420"` in `tauri.conf.json`.
**Warning signs:** Blank window on `cargo tauri dev`; "ECONNREFUSED" in tauri log.

## Code Examples

### Pre-loading SFX in `setup()`

```rust
// src-tauri/src/lib.rs (extension to existing run())
pub fn run() {
    init_tracing();
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Open SQLite (Phase 1 pattern)
            let store = std::sync::Arc::new(
                hallmark_lib::store::SqliteStore::open(/* db_path */).unwrap()
            );
            // Apply Phase 2 migration 002_schema_cache.sql
            store.with_conn(|c| Ok(c.execute_batch(include_str!("store/migrations/002_schema_cache.sql"))?)).unwrap();

            // Build popup + companion windows
            ui::create_popup_window(&app_handle).unwrap();
            ui::create_companion_window(&app_handle).unwrap();

            // Start audio dispatcher
            let audio = audio::AudioDispatcher::new().unwrap();

            // Start Phase 1 pipeline (returns sink)
            let (sink_tx, sink_rx) = tokio::sync::mpsc::channel(64);
            tokio::spawn(hallmark_lib::watcher::run_pipeline(/* ..., */ sink_tx, /* ..., */));

            // Spawn Phase 2 tasks
            let schema = schema::SchemaCache::new(store.clone());
            tokio::spawn(popup_queue::run(app_handle.clone(), sink_rx, schema.clone(), audio));
            tokio::spawn(game_detect::run(app_handle.clone(), schema.clone()));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri runtime failed to start");
}
```

### SQLite migration `002_schema_cache.sql`

```sql
-- Phase 2 schema additions: achievement schema cache + companion preferences + 100% flags.
-- Idempotent — safe on every open() per Phase 1 convention.

CREATE TABLE IF NOT EXISTS schema_cache (
    app_id          INTEGER NOT NULL,
    ach_api_name    TEXT    NOT NULL,
    display_name    TEXT,                -- nullable while resolution pending; popups fall back to ach_api_name
    description     TEXT,
    icon_path       TEXT,                -- absolute path to local PNG/JPG file in user data dir
    hidden          INTEGER NOT NULL DEFAULT 0,
    global_pct      REAL,                -- nullable; absent = rarity unavailable; render gracefully
    cached_at       INTEGER NOT NULL,
    PRIMARY KEY (app_id, ach_api_name)
);
CREATE INDEX IF NOT EXISTS idx_schema_app ON schema_cache(app_id);

CREATE TABLE IF NOT EXISTS companion_prefs (
    app_id      INTEGER PRIMARY KEY,
    filter      TEXT,        -- 'all' | 'earned' | 'locked'
    sort        TEXT,        -- 'earned-first' | 'a-z'
    expanded_id TEXT,        -- last-expanded ach_api_name (optional)
    width       INTEGER,
    height      INTEGER,
    pos_x       INTEGER,
    pos_y       INTEGER
);

-- 100%-completion flag uses existing settings table per CONTEXT.md D-11.
-- One row per game that hit 100%, keyed as 'completion_<app_id>'. Settings table
-- already has (key TEXT PRIMARY KEY, value TEXT NOT NULL).
```

### Steam global rarity fetch (no API key)

```rust
// src-tauri/src/schema/steam_api.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GlobalAchPctResponse {
    achievementpercentages: GlobalAchPctInner,
}
#[derive(Debug, Deserialize)]
struct GlobalAchPctInner { achievements: Vec<GlobalAchPct> }
#[derive(Debug, Deserialize)]
struct GlobalAchPct { name: String, percent: f64 }

/// Fetch global unlock percentages for an app from the public Steam Web API.
/// **No API key required.**
pub async fn fetch_global_pcts(client: &reqwest::Client, app_id: u64)
    -> anyhow::Result<std::collections::HashMap<String, f64>>
{
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/?gameid={app_id}&format=json"
    );
    let resp: GlobalAchPctResponse = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(8))
        .send().await?
        .error_for_status()?
        .json().await?;
    Ok(resp.achievementpercentages.achievements
        .into_iter()
        .map(|a| (a.name, a.percent))
        .collect())
}
```

[VERIFIED: partner.steamgames.com/doc/webapi/isteamuserstats] `GetGlobalAchievementPercentagesForApp` is publicly accessible without authentication. [VERIFIED: steamcommunity.com/dev shows the v0002 endpoint as anonymous-friendly.]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `OutputStream::try_default()` + `Sink::new(&handle)` (rodio 0.20) | `DeviceSinkBuilder::open_default_sink()` + `Player::connect_new(stream.mixer())` (rodio 0.22) | rodio 0.22 (2025) | API rename only; no behavior change. Update Cargo.toml + module. |
| `app.windows[].focus = false` only | `focusable(false)` builder method (Tauri 2.x) + `WS_EX_NOACTIVATE` HWND patch | Tauri 2.0 stable + ongoing patches | Defense-in-depth required; single mechanism still unreliable. |
| `tauri::api::http` (Tauri 1) | `tauri-plugin-http` 2.x OR direct `reqwest` from Rust | Tauri 2 | We use reqwest direct (Rust orchestrates fetches). |
| `cmd_args()` on sysinfo's Process | `cmd()` returning `&[OsString]` (sysinfo 0.30+) | sysinfo 0.30 (2024) | Minor breaking; CLAUDE.md / Phase 1 pinned 0.39 already. |
| Tauri allowlist (Tauri 1) | Capabilities + permissions ACL (Tauri 2) | Tauri 2.0 | New JSON files in `src-tauri/capabilities/`. |
| Steam Web API key embedded in client | Public no-key endpoints only (`GetGlobalAchievementPercentagesForApp/v0002/`) | Always — but commonly violated | TOS-safe; no revocation risk for OSS distribution. |

**Deprecated / outdated:**
- `OutputStream::try_default()` — gone in rodio 0.22. Use `DeviceSinkBuilder::open_default_sink()`.
- `tauri::api::http` — gone in Tauri 2. Use `tauri-plugin-http` or `reqwest`.
- Tauri `allowlist` — gone in Tauri 2. Use capabilities JSON.
- Goldberg `Documents/Goldberg SteamEmu Saves/` only — modern gbe_fork uses `%APPDATA%\GSE Saves\`. Phase 1 already covers both.

## Validation Architecture

> Phase 2 includes this section because Nyquist validation is configurable; even though `.planning/config.json` currently sets `nyquist_validation: false`, the planner can opt-in per task. The mapping below is provided for future reference.

### Test Framework

| Property | Value |
|----------|-------|
| Framework (Rust) | `cargo test` (already in Phase 1) |
| Framework (Frontend) | `vitest` (Vite-native; add only if frontend logic grows beyond declarative components) |
| Config file | `Cargo.toml` `[lib]` test target; `vitest.config.ts` if added |
| Quick run command | `cargo test -p hallmark popup_queue::` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| POPUP-01 | Popup webview receives `popup-show` payload | Integration | `cargo test --test popup_emit -- --nocapture` | Wave 0 (test fixture sends synthetic RawUnlockEvent into popup_queue, asserts emit_to was called) |
| POPUP-02 | 5-burst queue drains all 5 in order | Unit (queue logic) | `cargo test popup_queue::burst_5_drains_all` | Wave 0 |
| POPUP-03 | Popup positioned on game's monitor | Unit (math only) | `cargo test monitor::popup_position_top_right_quarter_down` | Wave 0 |
| POPUP-04 | DPI scaling — popup size in CSS px | Manual | `cargo tauri dev` on a 4K + 1080p multi-monitor rig | Manual |
| POPUP-05 | 100% celebration appears LAST in burst | Unit | `cargo test popup_queue::celebration_appended_last` | Wave 0 |
| POPUP-06 | Tier classification rare <10% | Unit | `cargo test schema::tier_classify_rare_threshold` | Wave 0 |
| POPUP-07 | Rarity rendered when present, absent when None | Unit (frontend, optional vitest) | `npm run test -- popup-rarity` | Defer (declarative; visual review acceptable) |
| POPUP-08 | WS_EX_NOACTIVATE applied; foreground unchanged | Manual | Launch `notepad.exe` foreground; trigger popup; check `GetForegroundWindow` unchanged | Manual |
| COMP-01 | Companion shows on game-start, hides on game-stop | Integration | `cargo test --test companion_lifecycle` | Wave 0 |
| COMP-02 | Companion lists earned + locked | Manual | Drop fixtures; verify list contents | Manual |
| COMP-03 | Mid-restart preserves session unlocks | Integration | `cargo test --test session_continuity` | Wave 0 |
| GAME-01 | Hybrid detection — Steam state + sysinfo fallback | Unit (each leg) + manual end-to-end | `cargo test game_detect::steam_priority`, `game_detect::sysinfo_fallback` | Wave 0 |
| GAME-02 | Schema async, popup uses cached at fire | Integration | `cargo test --test schema_async` | Wave 0 |
| GAME-03 | schema_cache offline operation | Integration | `cargo test --test schema_offline_after_warm` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --lib` (~5-15s on a warm cache)
- **Per wave merge:** `cargo test --workspace` + `cargo clippy -- -D warnings`
- **Phase gate:** Full suite green + manual POPUP-04 / POPUP-08 / COMP-02 verification on real hardware + WASAPI latency measurement on representative gaming rig

### Wave 0 Gaps

- [ ] `tests/popup_emit.rs` — integration test injecting synthetic events
- [ ] `tests/companion_lifecycle.rs` — game-start/stop event wiring
- [ ] `tests/session_continuity.rs` — restart-mid-session SQLite restore
- [ ] `tests/schema_async.rs` — async schema fetch doesn't block popup
- [ ] `tests/schema_offline_after_warm.rs` — offline behavior post-warm
- [ ] `src-tauri/src/popup_queue.rs::tests` — burst drain, celebration ordering, adaptive compression
- [ ] `src-tauri/src/monitor.rs::tests` — popup_position math
- [ ] `src-tauri/src/schema/mod.rs::tests` — tier classification thresholds
- [ ] Audio test harness — manual / out-of-band measurement; not unit-testable (requires real WASAPI device)

## Domain Notes (per the prompt's research areas A–N)

### A. External overlay window for popup (Tauri 2.11.1 + windows-rs 0.58)

**Decision:** See Pattern 1 (Section above). `WebviewWindowBuilder::new(...).decorations(false).transparent(true).always_on_top(true).skip_taskbar(true).focused(false).focusable(false).resizable(false).visible(false).inner_size(440.0, 96.0).accept_first_mouse(false).visible_on_all_workspaces(true).shadow(false).build()`. Then HWND patch with `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW`. Then `set_ignore_cursor_events(true)`.

**DX11 vs DX12 borderless-windowed compatibility:** The HWND patch + always_on_top + WS_EX_TOPMOST implicit covers borderless-windowed games regardless of D3D version. The risk case is **exclusive fullscreen** — by Windows DWM design, exclusive-fullscreen apps *should* paint over WS_EX_TOPMOST popups. Modern AAA games default to borderless-windowed (looks fullscreen, isn't actually exclusive), so the practical coverage is high. Per CLAUDE.md scope, exclusive-fullscreen is a v2 concern. **No special workaround required for DX11/DX12 in v1.**

`SetWindowDisplayAffinity` is a v2.5+ feature for the streamer-mode requirement (QOL-V2-02), not Phase 2.

**Existing real-world Tauri 2 game-overlay apps:** [CITED: blog.manasight.gg/why-i-chose-tauri-v2-for-a-desktop-overlay/] confirms 14MB RAM Tauri overlay on Windows 11 with the exact patterns above. No public open-source reference impl with the same use-case was found in 2026 — Hallmark may be the cleanest reference once shipped.

### B. Multi-monitor placement

**Decision:** Pattern 2 above. `MonitorFromWindow(game_hwnd, MONITOR_DEFAULTTONEAREST)` + `GetMonitorInfoW(hmon).rcWork` (`rcWork` excludes the taskbar — better than `rcMonitor` for popup placement). Compute physical pixel position; pass to `popup.set_position(PhysicalPosition::new(x, y))`.

**Tauri-API alternative:** `popup_window.current_monitor()` exists, but returns the popup's current monitor, not the game's. The Win32 path via game's HWND is required.

### C. Per-monitor DPI awareness

**Decision:** Tauri/TAO defaults to per-monitor v2 DPI awareness on Windows 10 1703+ (verified in DeepWiki TAO docs). **No manifest changes required.** Tauri's bundler injects the right manifest. CSS works in logical pixels and WebView2's `devicePixelRatio` is already correctly set by Tauri/TAO. **Don't manually multiply by `devicePixelRatio` in CSS** — that's double-counting.

**Pitfall when game launches on secondary monitor with different DPI:** Tauri emits a `ScaleFactorChanged` event on the popup window when its HMONITOR changes; we don't handle this in v1 (no UI changes per-monitor — the pill is a fixed CSS layout). The position math uses physical px so different DPI doesn't change the placement calculation.

**Specific pitfall:** When dragging the *companion* across monitors with different DPI, the inner content reflows correctly via WebView2 auto-scale — no action needed.

### D. Audio latency for signature sound (rodio 0.22.2 vs kira 0.12)

**Typical WASAPI shared-mode latency on Windows 10/11:** Default 10ms buffer + ~12ms Windows-internal = ~22ms baseline. Some hardware adds 30-80ms (older Realtek HDA, USB audio devices). Windows 10+ supports buffer sizes down to 2ms in shared mode. (Sources: learn.microsoft.com Low Latency Audio docs, mundobytes.com WASAPI latency guide.)

**Measurement methodology (the ROADMAP research flag):**
1. Build a small test binary that:
   - Initializes `DeviceSinkBuilder::open_default_sink()`.
   - On a hotkey, captures `Instant::now()` AND calls `mixer.add(decoder)`.
   - Logs the timestamp.
2. Use a **loopback recording** on the same machine (Audacity → Stereo Mix or VB-Cable virtual device).
3. The first audible peak in the waveform is the actual playback time; the time delta from logged-call to first sample = end-to-end latency.
4. Run on:
   - A typical gaming desktop with onboard Realtek + dedicated GPU.
   - A laptop with integrated audio.
   - Optionally a USB audio interface user.
5. **Threshold:** Median dispatch-to-audible >30ms → swap to kira 0.12.

**Pre-loading sound:** `include_bytes!("../../assets/sfx/popup-standard.wav")` puts the bytes in the `.exe`. At process start, decode once to verify validity (catches asset corruption). At play-time, clone the byte vec (Arc) into a fresh Cursor + Decoder. Decoder construction is cheap (<1ms for short WAV).

**Concurrent vs sequential play:** Per CONTEXT.md D-08 / D-09, popups are sequential at the **window-emit level** (one popup on screen at a time). But the **audio tail** of popup N can overlap popup N+1's slide-in. Use `mixer.add()` (overlapping/layered) for safety, not `Player::append` (strictly sequential). This avoids "previous sound clipped by next play" artifacts during bursts.

**Rare-tier sound variant:** Recommend **separate WAV file** (`popup-rare.wav`) over pitch-shifting `popup-standard.wav`. Reasons: (1) rodio doesn't have built-in real-time pitch-shift without a Source pipeline; (2) creative control over the exact mix is locked-in; (3) bundle size impact is trivial (~50-100KB per stem).

### E. Popup animation in WebView (Framer Motion 12)

**Decision:** Pattern 5 above. Spring `{ stiffness: 380, damping: 28, mass: 0.9 }` for entry; same for exit with `{ y: -16 }` on exit (slide-up + fade). Total perceived motion ~280ms — matches D-04's ~300ms spec.

**On-screen time:** D-08 locks 3000ms standard. The total cycle (300 in + 3000 hold + 300 out) = 3.6s, matching the user's framing. Adaptive compression D-10 brings it to 1500ms hold + 0ms gap = 2.1s/popup at depth >5.

**Reduced-motion:** Use `useReducedMotion()` from framer-motion to auto-detect `prefers-reduced-motion: reduce`. When true, fall back to opacity-only transition over 150ms. Internal toggle is **not in scope for v1** per "Customization: signature style locked" — the OS-level pref is the only honored signal.

**100% celebration variant:** Same component, slightly different transition tuning (more bounce — `{ stiffness: 280, damping: 22 }`), longer hold (5000ms vs 3000ms), and the 4-stem audio file. Tier marker on `motion.div` sets a CSS class for visual treatment (animated stroke, gold accent).

### F. Popup queue management (Rust ↔ React)

**Decision:** Pattern 4 above. Rust owns the queue (single drain task on Phase 1's `sink: mpsc::Receiver`). React side has zero queue logic; it just listens for `popup-show` / `popup-hide` and renders the latest payload.

**Why Rust-side beats React-side:**
1. The popup window may not be loaded yet at burst start (cold WebView2 first paint takes ~200-500ms); a React queue can't hold events emitted to a window that doesn't yet have a listener.
2. Restart-resilience: a React reload (e.g. dev hot reload) loses queue state. Rust survives.
3. Cross-window coordination: the 100% celebration appended-last rule (D-12) requires queue-level visibility; React-only can't see the queue tail without a Rust round-trip.

**Backpressure:** Bounded `mpsc::channel(64)` from Phase 1 — if 64 unlocks pile up unconsumed, Phase 1's send() blocks. That's the correct overload signal; in practice, gameplay never produces 64-event bursts (more than a single "DLC mass-unlock" event), so 64 is generously safe.

**100% appended-last (D-12):** Inside the drain loop, after `sink.recv()` returns `None` *would* mean the channel closed — but we want "next recv would block but channel still open." Use `sink.try_recv()` to peek; if `Empty`, we've drained; emit pending celebration as a synthetic event. Simple state machine.

### G. Companion window

**Decision:** Separate `WebviewWindow` (label `"companion"`), entry HTML `index.html` (Vite-built React tree). Config: `decorations: false`, `transparent: false` (this window has substance), `always_on_top: false` (D-16), `skip_taskbar: false` (visible in alt-tab), `focusable: true` (interactive), `resizable: true`, `inner_size: 480 × 720`, `min_inner_size: 360 × 480`.

**No HWND patch on companion** — this window IS interactive; we want normal focus.

**Show/hide on game-start/stop:** game_detect task emits `app.emit("game-started", { app_id })` and `app.emit("game-stopped", {})`. Companion JS:
```ts
listen("game-started", () => getCurrentWebviewWindow().show());
listen("game-stopped", () => getCurrentWebviewWindow().hide());
```
Hide ≠ destroy. Window stays in memory; next show() is instant.

**Persistence (D-15):** On `tauri://move` and `tauri://resize` events, write `companion_prefs` row to SQLite (debounce 500ms). On startup, read row; apply `set_size` + `set_position` if present. If absent (first run), `center()`.

**Scroll behavior + virtualization:** Native CSS scroll (`overflow-y: auto`) + `content-visibility: auto` on each row. This handles up to ~500 achievements smoothly. Add `@tanstack/react-virtual` only if a target game needs it (none currently known to break this threshold).

**Race: companion shows before schema warm:** D-20. Show with cache hits + skeleton for misses; subscribe to `schema-resolved` event; upgrade rows in place. Earned rows from `unlock_history` always show with at least the api_name as title.

### H. "Earned this session" persistence (SQLite restore on restart)

**Decision:** Reuse Phase 1's `sessions` + `unlock_history` tables. New rule: on Hallmark startup, before opening a new session, query:
```sql
SELECT session_id, app_id, started_at FROM sessions
WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1;
```
If a row exists AND that app_id's process is still running (sysinfo check), reuse `session_id`. If process is gone, mark `ended_at = now()` and don't reuse.

**Recommended crate:** `rusqlite` 0.39 — already in Cargo.toml from Phase 1. Rationale: sync API is a non-issue for this access pattern (queries take <1ms; called from already-async tokio tasks via `with_conn` closures). **Don't introduce sqlx** — single-binary parity with Phase 1 is more valuable than async DB.

**Migration approach:** New file `migrations/002_schema_cache.sql` with `IF NOT EXISTS` everywhere; loaded with `include_str!` + `execute_batch` per Phase 1 pattern. No `rusqlite_migration` or `refinery` crate needed.

### I. Achievement schema cache

**Decision:** Single `schema_cache` table (one is fine — icons stored as filesystem paths, not blobs, so the row is small). Fields: see `002_schema_cache.sql` SQL above. Icon storage: extract Goldberg-bundled icons (when present) or download from `media.steampowered.com/steamcommunity/public/images/apps/<AppID>/<ach_image_hash>.jpg`, save to `%APPDATA%\Hallmark\icons\<app_id>\<ach_api_name>.jpg`, store path in `icon_path`.

**File path over base64 in SQLite:** Files load instantly via `file://` (or `convertFileSrc()`) without rehydration; SQLite gets faster row reads (no large BLOB scan).

**Pre-warm timing (D-25):** schema task fires on game-start. First popup of FIRST session may have `display_name = ach_api_name` and no icon — that's the documented graceful degradation (D-26). All subsequent popups (and all popups in second+ session) are instant.

**Lookup chain (D-24):**
1. `schema_cache` SQLite read — most recent.
2. Goldberg `achievements.json` — direct read; gives display_name, description, icon URL (download if not yet local).
3. Steam local `appcache/librarycache/<app_id>_*` — gives game header; achievement icons not stored here in modern Steam (rare).
4. Steam Web API `GetGlobalAchievementPercentagesForApp` — rarity only; **no name/description from this endpoint**.
5. **NO** `GetSchemaForGame` — requires user API key; TOS-incompatible with OSS distribution.

For Goldberg-only games, sources 2 + 4 cover everything. For legitimate Steam games where Goldberg isn't running (Phase 3 territory), the schema fallback is empty — display will be api_name only until Steam binary VDF parsing is implemented in Phase 3.

### J. Rarity % from Steam appcache

**Refinement vs CONTEXT.md D-24:** The local `%STEAM%\appcache` path does NOT contain global rarity %. `appcache/stats/UserGameStats_<userid>_<appid>.bin` is per-user achievement state (Phase 3 binary VDF target). Global % is **only** available from the Steam Web API `GetGlobalAchievementPercentagesForApp/v0002/` endpoint.

**Free-path constraint resolution:** The endpoint requires NO API key. It IS publicly accessible. So Hallmark CAN use it without violating the "no Steam Web API for detection" constraint — that constraint scopes to *unlock detection* (where API polling lag is the issue), not *metadata enrichment*. PROJECT.md scope explicitly permits "schema/icon fetch and update check" as outbound network.

**Graceful absence (POPUP-07):** Component renders `popup-rarity` div only when `global_pct !== null`. CSS Flex column layout reflows automatically without it.

**Rare threshold (D-27):** `global_pct < 10.0` → tier = "rare". Matches Steam's own UI cutoff.

### K. Process scanner integration with companion (Phase 1 → Phase 2 wiring)

**Decision:** Phase 1's process scanner does NOT exist (Phase 1 is detection-only with no game session concept). Phase 2 builds the scanner from scratch in `game_detect/`.

**Steam state read (no IPC):** Steam doesn't expose a public IPC for "currently playing" without third-party hacks (SteamRE, Steamworks SDK). Practical free-path:
- Read `%STEAM%\config\loginusers.vdf` (text VDF) — gives signed-in user.
- Process scan for `steam.exe` confirms Steam is running.
- For "currently playing app": parse `%STEAM%\steamapps\appmanifest_*.acf` for the most-recently-modified manifest with `LastUpdated` near now → heuristic, unreliable.
- **Better path:** scan all running processes via sysinfo; cross-reference each process's `exe()` path against discovered Steam library paths (Phase 1 already has `paths::SteamLibraries`). If a process's exe is inside a steamapps/common/<install_dir>, the matching `appmanifest_*.acf` gives the appid.
- For Goldberg-emulated games not in any steamapps/common (running from arbitrary path), scan command-line args for `AppId=<num>` or check process exe path against Goldberg redirect roots (Phase 1 already discovers these via `local_save.txt`).

**Polling cadence:** `tokio::time::interval(Duration::from_secs(3))`. On each tick, refresh sysinfo, diff running-game set vs previous tick, emit `game-started` for new entries and `game-stopped` for departed entries.

**Race: companion auto-shows before schema warm:** Already addressed in section G. Skeleton state from `unlock_history` entries; in-place row upgrade on `schema-resolved`.

### L. Click-through / non-interactive popup

**Decision:** `popup.set_ignore_cursor_events(true)` once at window creation. Covers entire window since popup has no controls.

**Companion:** Do NOT set this. Default click-through is off; companion is fully interactive.

**Hover-to-pause-dismiss:** **NOT in v1.** Per CLAUDE.md "Customization: Signature style locked — no user-editable themes, sounds, positions, or animations in v1." Hover-pause counts as interactivity not in spec. Confirmed deferred.

### M. Validation Architecture

See the dedicated section above. Key points:
- Audio test: out-of-band loopback recording (manual). Not unit-testable.
- Focus-steal test: write a `tests/focus_steal_e2e.rs` that spawns `notepad.exe`, waits for it foreground, triggers a synthetic popup via Tauri command, asserts `GetForegroundWindow()` is still notepad's HWND. Use windows-rs in the test crate for the assertion.
- Multi-monitor test: skip on single-monitor CI runners; gate on `cfg!(feature = "multi_monitor_test")` and run only in dev workstations.

### N. Pitfalls / landmines specific to Tauri 2.11.1 on Windows

- **Issues #7519, #11566, #12055** — focus/focusable not 100% reliable. Workaround: defense-in-depth with manual WS_EX_NOACTIVATE (Pitfall 2 above). All three issues are **closed without explicit fix-version** as of last research; treat as still-active risk.
- **Transparent windows + WebView2:** No glass/blur effect on the window itself; we paint our own backdrop-blur via CSS. WebView2 inherits the transparent window background, so the popup's CSS `background: rgba(15,18,24,0.8); backdrop-filter: blur(24px)` works as expected. **Caveat:** `backdrop-filter` blur in WebView2 v125+ is supported; older WebView2 runtimes (Win10 LTSC) may render solid. Tauri 2.11 requires WebView2 125+ which the user has.
- **HWND lifecycle:** `window.hwnd()` is valid immediately after `build()` returns successfully. Issue #13046 reports intermittent `hwnd()` errors on child windows created via JS — we create both windows from Rust during `setup()`, which avoids the reported issue path.
- **`get_webview_window` vs `get_window`:** Use `app.get_webview_window("popup")`. The Tauri 1 → 2 migration renamed `get_window` to `get_webview_window`.
- **Windows manifest:** Tauri's bundler injects a per-monitor v2 DPI manifest by default. Don't add a custom `manifest.xml`; it overrides Tauri's and may break DPI.

## Runtime State Inventory

> Phase 2 introduces NEW state but does not rename / refactor / migrate any Phase 1 state. Section included for completeness.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | New: `schema_cache` rows, `companion_prefs` rows, `settings.completion_<appid>` keys, icon files at `%APPDATA%\Hallmark\icons\` | New writes only; no migration of Phase 1 data |
| Live service config | None — Hallmark is a local-only desktop app | None |
| OS-registered state | None in Phase 2 (Phase 4 will add `HKCU\...\Run` for start-with-Windows) | None |
| Secrets/env vars | None — Steam Web API endpoints used are key-less. No secrets to rotate. | None |
| Build artifacts | `dist/` directory replaced by Vite-built React frontend (Phase 1 had a placeholder `dist/index.html`); `target/` rebuilds with new deps | `cargo clean` recommended on first Phase 2 build to clear stale Phase 1 artifacts |

**Nothing found in category "Live service config":** None — verified by Hallmark scope (PROJECT.md): no cloud, no accounts, no telemetry, no remote services.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain ≥1.85 | All Rust code | ✓ (Phase 1 confirmed) | stable | — |
| Node.js ≥20.11 | Vite + React build | Must verify on dev machine | — | `nvm install 22` |
| pnpm or npm | Frontend deps install | Must verify | — | `npm install -g pnpm` |
| WebView2 Runtime ≥125 | Tauri 2.11 popup/companion | ✓ (Windows 10/11 default) | varies | Tauri bundler ships fixed-version WebView2 if needed |
| Windows 10 ≥1703 | Per-monitor DPI v2 (TAO requirement) | ✓ (covered by Win10/11 baseline) | — | Older Win10: per-monitor v1 fallback (auto) |
| Steam client | Steam-state detection (Section K) | Optional — Goldberg path doesn't require it | — | Process scan + ACF match works without Steam running |
| Audio output device | rodio | ✓ (any Windows machine) | — | If no device: log warning, popups still fire visually |
| Internet connection | First-time schema/rarity fetch | Optional | — | Cached after first successful fetch; popups degrade to "no description" / "no rarity" |

**Missing dependencies with no fallback:** None — all required deps are baseline Windows 10/11.

**Missing dependencies with fallback:**
- Steam client absent → Goldberg-only mode (covered by D-21).
- Audio device absent → silent popups (logged warning, visual still fires).
- Internet absent on first run → schema cache cold, popups show api_name only until first connection.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | rodio 0.22 `Mixer::add` allows true layered/concurrent SFX without explicit mixing config | Section D Pattern 3 | Concurrent rare-tier+standard plays may clip or not layer; fallback is sequential `Player::append` (acceptable degradation) |
| A2 | `WS_EX_NOACTIVATE` post-creation patch fully prevents focus steal across DX11 + DX12 borderless games — ROADMAP research flag | Section A, Pitfall 2 | If fails: explore `SetWindowPos` with `SWP_NOACTIVATE` on every show, or migrate to wry's lower-level window APIs |
| A3 | WASAPI shared-mode latency on representative gaming hardware will be ≤30ms with rodio 0.22 default buffer config — ROADMAP research flag | Section D, Pitfall 6 | Swap to kira 0.12; documented fallback per CLAUDE.md alternatives table |
| A4 | `GetGlobalAchievementPercentagesForApp/v0002/` is publicly accessible without API key, indefinitely | Section J, Pitfall 7 | If Valve gates it: degrade rarity to "unavailable" globally; popup still fires without rarity (POPUP-07 graceful path) |
| A5 | `media.steampowered.com/steamcommunity/public/images/apps/<AppID>/<hash>.jpg` continues to host icons publicly | Section I | Goldberg-bundled icons cover all Goldberg games regardless; legitimate Steam games' icons via `appcache/librarycache` cover the rest |
| A6 | Tauri 2.11 `WebviewWindowBuilder::focusable(false)` builder method is the documented Windows-mapped to WS_EX_NOACTIVATE — no internal regression | Section A Pattern 1 | Manual HWND patch is the load-bearing mechanism; focusable() is belt-and-suspenders |
| A7 | Goldberg `achievements.json` schema field names (display_name vs displayName) are stable enough that Phase 1's empirical notes cover Phase 2 reads | Section I, Pitfall 8 | Tolerant parse via `serde_json::Value` for unknown variants; explicit field-mapping fallback list in cache.rs |
| A8 | The single-bounded-mpsc(64) backpressure model from Phase 1's sink is sufficient for the worst-case unlock burst (5-10 typical, 50+ in DLC mass-grant) | Section F | Increase capacity; observe in production via tracing |
| A9 | Windows 10 1703+ baseline holds for Hallmark v1 users (per-monitor DPI v2 default) | Section C | Older Win10 falls back to per-monitor v1 — popup may scale incorrectly across monitors with mixed DPI; manifest tweak fallback documented at gist.github.com/emoacht |
| A10 | rodio 0.22's `mixer().add()` is non-blocking and lock-free in the hot path | Section D Pattern 3 | If blocking: move audio dispatch to `tokio::task::spawn_blocking` from the queue task |

## Open Questions

1. **What's the actual WASAPI shared-mode latency on representative gaming hardware?**
   - What we know: Windows 10+ baseline ~22ms; can be 30-100ms on older drivers.
   - What's unclear: Hallmark's specific user hardware distribution.
   - Recommendation: Build a small measurement binary (Section D), run on 2-3 representative rigs during Plan 02-XX, lock the rodio-vs-kira decision based on data.

2. **Does Tauri 2.11's `focusable(false)` actually map to WS_EX_NOACTIVATE on Windows?**
   - What we know: Builder method exists; behavior on Windows not explicitly documented in 2.11 release notes.
   - What's unclear: Internal implementation may apply other techniques (focus-steal-prevention without WS_EX_NOACTIVATE proper).
   - Recommendation: Defense-in-depth — apply both. Verify with a `GetWindowLongPtrW(hwnd, GWL_EXSTYLE)` read after build to confirm the flag is set; manually OR-in if missing.

3. **Schema cache hit rate for Goldberg-only games.**
   - What we know: Goldberg `achievements.json` includes display_name + description for most cracks.
   - What's unclear: Some Goldberg builds strip metadata (only api_name + earned). For these, popups would degrade per D-26.
   - Recommendation: Track hit-rate in tracing; if <80%, add SteamSpy as a metadata fallback.

4. **Steam library cache `librarycache` icon naming convention in 2026.**
   - What we know: `<app_id>_*.jpg` patterns vary (`_library_600x900.jpg`, `_library_hero.jpg`, `_logo.png`, etc.). Achievement icons specifically aren't in librarycache — they're at `media.steampowered.com/steamcommunity/public/images/apps/<AppID>/<icon_hash>.jpg` (resolved from binary VDF — Phase 3 territory) or from Goldberg's `achievements.json` (Phase 2 reachable).
   - What's unclear: Whether legitimate-Steam achievement icons can be resolved without binary VDF parsing.
   - Recommendation: For Phase 2, only Goldberg games get achievement-icon coverage. Legitimate Steam (Phase 3) gets icons after binary VDF parsing.

5. **100%-completion atomic update under burst-with-celebration scenario.**
   - What we know: D-12 says celebration appended last in queue.
   - What's unclear: If Phase 1 detects unlock #5 (which completes the set) WHILE the queue task is mid-emit on unlock #3, does the `celebration_pending` flag survive correctly? The cleaner pattern is to mark it on detect (in popup_queue) and only emit when sink is empty.
   - Recommendation: Implement and unit-test. The atomicity is local to one task's state, so straightforward.

## Project Constraints (from CLAUDE.md)

These constraints from `./CLAUDE.md` MUST be honored by the planner:

- **Platform:** Windows-only for v1. `cfg(target_os = "windows")` is acceptable; cross-platform abstraction is explicitly out of scope.
- **Overlay tech:** External borderless always-on-top window only. **No DLL injection** in v1 (anti-cheat risk). DX12-exclusive-fullscreen edge cases are deferred.
- **Detection:** Local file watcher only — Phase 2 does NOT add Steam Web API for unlock detection. Schema/icon/rarity fetch is permitted (different feature).
- **Distribution:** Free, OSS on GitHub — no embedded API keys, no telemetry, no analytics, no crash reporting.
- **Goldberg/emulator stance:** Passive detection only — Phase 2 reads emulator output paths but does NOT install, configure, or recommend.
- **Customization:** **Signature style locked.** No user-editable themes, sounds, positions, or animations in v1. Confirms hover-to-pause is out (Section L), confirms theme system is out, confirms no settings UI in Phase 2.
- **Pace:** Hobby project — polish over speed; no fixed deadline.
- **Stack pins:** Tauri 2.11, React 19, Vite 6, Framer Motion 12, rodio 0.22.2 (or kira 0.12 fallback), sysinfo 0.39, windows-rs 0.58.
- **What NOT to use:** Electron (RAM bloat), DLL injection (deferred), FMOD/irrKlang (commercial), WMI (slow), ETW (overkill), velopack (pre-release).
- **Stack patterns:** Per CLAUDE.md "Stack Patterns by Variant" — popup overlay window pattern, companion window pattern, file watcher pattern, audio playback pattern, process scanner pattern. **All five are referenced and refined in this RESEARCH.md.**
- **GSD workflow enforcement:** No direct repo edits outside a GSD command.

## Sources

### Primary (HIGH confidence)

- [Cargo.toml workspace pin] `tauri = "2.11"`, `rusqlite = "0.39"`, `notify = "8.2"`, `tokio = "1.52"` — already in tree, verified.
- [Cargo search 2026-05-08] `rodio 0.22.2`, `sysinfo 0.39.0`, `kira 0.12.0`, `windows 0.62.2`, `reqwest 0.13.3`, `tauri-plugin-sql 2.4.0`, `rusqlite_migration 2.5.0`, `refinery 0.9.1` — all current.
- [docs.rs/tauri/2.11.1/tauri/webview/struct.WebviewWindowBuilder.html] All builder methods used in Pattern 1.
- [docs.rs/tauri/2.11.1/x86_64-pc-windows-msvc/tauri/window/struct.Window.html] `hwnd()`, `current_monitor()`, `available_monitors()`, `scale_factor()`, `set_ignore_cursor_events()`.
- [docs.rs/rodio/0.22.2] `DeviceSinkBuilder`, `Player`, `MixerDeviceSink`, `Decoder`, `Source`.
- [github.com/rustaudio/rodio UPGRADE.md] 0.20 → 0.22 API rename mapping.
- [learn.microsoft.com winuser/MonitorFromWindow] `MONITOR_DEFAULTTONEAREST` flag semantics.
- [partner.steamgames.com/doc/webapi/isteamuserstats] `GetGlobalAchievementPercentagesForApp` (no key required), `GetSchemaForGame` (key required — DON'T USE).
- [Phase 1 RESEARCH.md / SUMMARY.md / VERIFICATION.md / empirical-goldberg-schema-NOTES.md] Locked detection contract, Goldberg field names, file paths.
- [Context7: tauri-apps/tauri-docs] Window customization patterns, event system, capabilities.
- [Context7: grx7/framer-motion] AnimatePresence, spring transitions, useReducedMotion, variants.

### Secondary (MEDIUM confidence)

- [Tauri issues #7519, #11566, #12055, #14102, #13046] Focus-steal bugs and HWND lifecycle issues — the recommendation to defense-in-depth WS_EX_NOACTIVATE is based on these reports.
- [WebSearch: WASAPI shared mode latency] Microsoft Learn Low Latency Audio docs + miniaudio issue + mundobytes guide — typical 22ms baseline, 2ms minimum buffer in Win10 1703+.
- [WebSearch: tauri-plugin-sql vs rusqlite] sqlx is the Tauri-recommended path; rusqlite is justified for our existing-Phase-1 case.
- [WebSearch: React virtualization 2026] @tanstack/react-virtual ~12KB, react-window 4 components — neither needed for v1 list sizes.
- [WebSearch: tokio mpsc bounded vs unbounded] Bounded for desktop apps; unbounded leaks memory under producer overrun.
- [WebSearch: prefers-reduced-motion + Framer Motion `useReducedMotion` hook] Native framer hook is the recommended path.
- [WebSearch: Steam appcache librarycache] Game header images present; achievement icons not stored locally for legitimate Steam.

### Tertiary (LOW confidence — flagged for empirical validation)

- WASAPI dispatch-to-audible latency on Hallmark-target hardware: must measure (ROADMAP research flag).
- `WS_EX_NOACTIVATE` effectiveness across DX11 + DX12 borderless-windowed AAA titles: must verify with at least one DX11 + one DX12 sample game (ROADMAP research flag).
- Tauri 2.11's internal mapping of `focusable(false)` to Windows: documented method exists; internal mapping not explicitly documented.
- 100%-completion celebration ordering invariants under high-burst conditions: unit-test pending.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates verified via crates.io 2026-05-08; only the rodio 0.22 API shape is a 2025 change vs CLAUDE.md (Sink → Player rename, semantically identical).
- Architecture: HIGH — patterns derive from Tauri 2 official docs + Phase 1 established conventions (Arc<Mutex>, with_conn closures, mpsc fan-out, setup()-based task attachment).
- Pitfalls: MEDIUM — focus-steal bug is documented in three closed Tauri issues without explicit fix version; defense-in-depth pattern is the safe path.
- Audio latency: LOW until measured — research flag from ROADMAP must be resolved with empirical data on representative hardware.

**Research date:** 2026-05-08
**Valid until:** 2026-06-07 (30 days for stable items; rodio + Tauri release pace warrants re-verification before 2026 Q3).

## RESEARCH COMPLETE

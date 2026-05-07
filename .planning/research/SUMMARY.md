# Project Research Summary

**Project:** Hallmark
**Domain:** Windows desktop achievement-notifier + session companion overlay (Steam legit + Goldberg/CreamAPI/SmartSteamEmu emulators)
**Researched:** 2026-05-07
**Confidence:** HIGH

## Executive Summary

Hallmark is a Windows desktop overlay app whose hero feature is a premium console-grade in-game popup + signature sound on every Steam achievement unlock — covering both legitimate Steam and Goldberg/CreamAPI/SmartSteamEmu emulators through a single local file watcher. Secondary feature is a session-focused companion window auto-shown while a game is running. No cloud, no themes, no DLL injection, signature style locked.

Recommended stack: **Tauri v2 (Rust + WebView)** for tiny RAM footprint (~14MB target) and native Win32 access; **notify 8.x + notify-debouncer-full** for the file watcher; **rodio 0.22.x** for low-latency audio (kira as fallback if WASAPI shared-mode latency is unacceptable); **sysinfo** for process-based game detection; **Steam Web API GetSchemaForGame** with local SQLite cache for achievement metadata; **NSIS installer + tauri-plugin-updater** for distribution.

Critical risks are concentrated in three areas: (1) **first-launch state seeding** — every notifier ships this bug at least once; the watcher must seed baseline state from existing files BEFORE attaching change handlers; (2) **focus-stealing** — Tauri `focus: false` is a known unfixed bug on Windows, requires post-creation `WS_EX_NOACTIVATE` HWND patch; (3) **Goldberg path discovery** — many real-world setups redirect saves via `local_save.txt` next to the steam_api.dll, so watching only `%APPDATA%\Goldberg SteamEmu Saves\` produces silent zero-popup failure. Mitigations are well-known and addressable in Phase 1.

## Key Findings

### Recommended Stack

Tauri v2 wins decisively over Electron (~14MB vs 200–300MB RAM running alongside a game) and over WPF/WinUI (no animation/web-tooling ecosystem, no future cross-platform path). All file/process/Win32 access happens in the Rust backend at zero marshalling cost. Modern web animation libraries deliver the popup polish.

**Core technologies:**
- **Tauri v2.11.x** — app shell, multi-window (popup + companion), Win32 HWND access for overlay flags
- **notify 8.2.x + notify-debouncer-full** — Windows `ReadDirectoryChangesW` file watcher with 500ms debounce
- **rodio 0.22.2** — low-latency one-shot audio playback (Sink pattern, persistent OutputStream)
- **sysinfo 0.39.x** — process enumeration + command-line capture (2–3s polling)
- **rusqlite (bundled)** — single `hallmark.db` for schema cache, icon cache (BLOB), unlock history, sessions
- **windows-rs** — `SetWindowLongW` to apply `WS_EX_NOACTIVATE` post-window-creation
- **Steam Web API GetSchemaForGame/v2** — keyless 100k/day quota; lazy-fetched, locally cached
- **Goldberg `steam_settings\achievements.json`** — offline schema fallback for emulator-only users
- **NSIS bundler + tauri-plugin-updater 2.10.x** — installer + auto-update via GitHub Releases `latest.json`
- **Frontend** — React + Tailwind + Framer Motion (or equivalent) for popup/companion UI

**Anti-recommendations:**
- Electron — 20× RAM cost while a game runs
- DLL injection / RTSS hooks — out of scope; anti-cheat / AV risk
- Steam Web API as primary detection — 1–5 min lag breaks the popup feel
- Velopack — still pre-release as of 2026-05; revisit for v2
- Theme presets / sound packs — locked-style is a brand decision, not a feature

### Expected Features

**Must have (table stakes — 13):**
- Real-time file-watcher detection of Steam-legit + Goldberg unlocks (single mechanism, transparent)
- Per-emulator duplicate suppression (one logical unlock can fire from multiple sources)
- First-launch state seeding (avoid "everything just unlocked" spam)
- Schema-and-icon resolution at game-launch time, not at unlock time (avoid popup latency)
- Achievement icon + title + description in popup
- Popup animation in/out + signature sound
- Popup queue handling for close-succession unlocks (with explicit 100% celebration placement)
- Popup duration / dismiss behavior
- Game-running detection (Steam state when available + process scanner fallback)
- Achievement icon caching (offline operation)
- Unlock-history persistence (mid-game restart preserves "earned this session")
- Session-focused companion: auto-shows on game launch, auto-hides on close
- Start-with-Windows option

**Should have (differentiators — 8):**
- Tier-based popup styling (rare achievements get richer treatment) — graceful degrade if rarity data absent
- Rarity % displayed in popup (sourced from Steam `appcache` global stats; not always present)
- 100% / completion celebration popup
- Multi-monitor: popup on the game's monitor (HWND lookup)
- DPI-aware popup sizing
- Pinned "next unlock" surface in companion view
- DND mode (silence during cutscenes / streams)
- Streamer / privacy mode (popup hidden from capture)

**Defer (v2+):**
- DLL injection overlay (exclusive-fullscreen reliability)
- Cloud sync, accounts, leaderboards, social
- Theme presets / sound customization
- Stores beyond Steam (Epic, GOG, Xbox, Ubisoft, EA, Battle.net) — community-extensible adapter pattern designed in, but not shipped
- Screenshot-on-unlock
- Lifetime stats dashboard

### Architecture Approach

Event-driven pipeline: filesystem change → source adapter (per emulator family) → debouncer → diff vs last-known state → canonical Unlock event → schema resolver enrichment → notification queue → popup window + companion window update + SQLite history write. Single-process Tauri app with a background-tray model and two windows (popup, companion). Schema/icon resolution is lazy + async + non-blocking — emit a partial popup immediately if metadata is missing, re-render when fetched.

**Major components:**
1. **Source adapters** — `SourceAdapter` trait with implementations for Goldberg (JSON), CreamAPI (JSON/INI), SmartSteamEmu (per-persona folder), Steam-legit (binary VDF). Hides format differences.
2. **Watcher core** — notify-based monitor + debouncer + deduplicator across adapters
3. **Schema resolver** — Steam Web API `GetSchemaForGame` with SQLite cache; Goldberg `steam_settings/achievements.json` as offline fallback
4. **Game-session detector** — sysinfo process scan + Steam appmanifest/libraryfolders.vdf parsing → "what game is running now?"
5. **Notification queue / orchestrator** — bursts, dismiss timing, 100% popup placement
6. **Popup renderer** — Tauri overlay window (`always_on_top: true`, `decorations: false`, `transparent: true`) + post-creation `WS_EX_NOACTIVATE` patch
7. **Companion window** — interactive, no `WS_EX_NOACTIVATE`, session-focused list view
8. **Persistent store** — single SQLite `hallmark.db` (schema_cache, icon_cache BLOB, unlock_history, sessions)
9. **Settings / preferences** — minimal: start-with-Windows, monitor target, DND, mute

### Critical Pitfalls

1. **First-launch "200 achievement spam"** — seed baseline state from existing files BEFORE attaching change handlers. Same phase as the watcher itself.
2. **2–4 file events per logical write** — debounce 400–600ms (use 500ms) + content-hash equality check.
3. **Goldberg path discovery is non-trivial** — many real cracks redirect via `local_save.txt` adjacent to `steam_api.dll`; default `%APPDATA%` watch alone produces zero popups. Discovery must scan `local_save.txt` redirects + Steam library folders for Goldberg-stamped game directories.
4. **Focus-stealing** — Tauri `focus: false` is a confirmed open bug; apply `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT` via `windows-rs` `SetWindowLongW` immediately after window creation. Companion window must NOT get this flag (it's interactive).
5. **Steam library path moved in 2022** — parse both `config\libraryfolders.vdf` and legacy `SteamApps\libraryfolders.vdf`; both old and new VDF formats.
6. **No visible test trigger** — ship a "fire test popup" button on day one; users with silently-misconfigured installs need feedback.
7. **SmartScreen / AV false positives** — file-reading + topmost-window pattern triggers heuristics. Mitigation: README privacy framing, VirusTotal scan link in releases, eventual code-signing.
8. **Hidden / spoiler achievements** — Steam returns placeholder; do NOT expose the real text on unlock without an explicit decision.
9. **DPI scaling** — popup looks tiny on 4K if not declared DPI-aware in manifest.

## Implications for Roadmap

Coarse granularity (3–5 phases) is the right call given hobby pace + tightly-scoped v1.

### Phase 1: Detection Pipeline Foundation
**Rationale:** Watcher correctness + first-launch seeding + path discovery are the load-bearing risks. If any one of them ships broken, every later phase tests against poisoned data. Build this end-to-end with fixtures before any UI work.
**Delivers:** Tauri scaffold (no UI), `SourceAdapter` trait, Goldberg adapter, watcher core with 500ms debounce, first-launch state seeding, Steam library/Goldberg path discovery (incl. `local_save.txt` redirects), SQLite store (schema_cache + unlock_history), CLI test harness that prints unlock events to stdout.
**Addresses:** All "must have" detection-side features for Goldberg.
**Avoids:** First-launch spam, debounce pitfalls, path discovery silent failures.
**Research flag:** None — research is sufficient.

### Phase 2: Premium Popup + Sound + Companion Window (UI)
**Rationale:** Can run in parallel structure-wise but is sequenced after Phase 1 because premium-feel verification requires real unlock events through the pipeline. Popup window and companion window can be developed concurrently against mock events; integration with Phase 1 happens at end of phase.
**Delivers:** Tauri popup window with signature animation + sound (rodio), `WS_EX_NOACTIVATE` HWND patch, popup queue with 100% placement, Tauri companion window with session-focused list, schema-at-launch-time resolution + icon cache, multi-monitor + DPI-aware positioning, "fire test popup" button, game-launch detection via sysinfo + Steam state.
**Uses:** Tauri v2 multi-window, rodio, sysinfo, frontend animation library.
**Implements:** Popup renderer, companion window, notification queue, schema resolver, game-session detector.
**Avoids:** Focus stealing, latency-on-unlock, multi-monitor wrong-screen, DPI tiny popup, no-test-trigger UX dead end.
**Research flag:** Empirical validation of WASAPI shared-mode latency on real gaming hardware — kira fallback may be needed if rodio cannot hit ~30ms.

### Phase 3: Steam-Legit Adapter + Remaining Emulators
**Rationale:** Steam-legit binary VDF parsing is the highest-risk parser; CreamAPI and SmartSteamEmu follow Goldberg's pattern but each has format quirks. Sequenced after Phase 2 so the popup is already validated end-to-end via Goldberg before adding adapter complexity.
**Delivers:** Steam-legit adapter (`appcache/stats` mtime trigger + binary VDF parser of `userdata/<steamid>/<appid>/remote/`), CreamAPI adapter, SmartSteamEmu adapter, per-source duplicate suppression across adapters watching the same appID.
**Uses:** Existing Phase 1 trait + watcher core; no new dependencies.
**Implements:** Three additional `SourceAdapter` implementations.
**Avoids:** Multi-source duplicate popups, Goldberg-only blind spots.
**Research flag:** **HIGH** — Steam binary VDF schema fields and CreamAPI per-appid file format need empirical validation against live installations during planning.

### Phase 4: Polish, Distribution, Public Release
**Rationale:** Polish features and distribution wrap-up; nothing here gates correctness.
**Delivers:** DND mode, streamer/privacy mode, start-with-Windows option, rarity % popup (where data available — graceful degrade), tier-based popup styling for rare unlocks, NSIS installer build via tauri-action GitHub workflow, tauri-plugin-updater wired to `latest.json`, README privacy framing, VirusTotal scan workflow, public GitHub release.
**Uses:** Existing stack; tauri-action, tauri-plugin-updater.
**Implements:** Polish features + release pipeline.
**Avoids:** SmartScreen/AV trust pitfalls, missing rarity gracefully, distribution surprises.
**Research flag:** None — distribution patterns are standard.

### Phase Ordering Rationale

- Detection pipeline first because every visible feature depends on correct events flowing through it; building UI on a leaky pipeline produces unreproducible popup bugs.
- UI second because premium-feel can only be verified against real events from a stable pipeline.
- Steam-legit and remaining emulators third because they share the validated trait + watcher infrastructure but each carries parse-format risk; isolating them prevents Phase 1/2 timeline slip.
- Polish + distribution last because correctness must precede release.

### Research Flags

Phases needing deeper research at plan-time:
- **Phase 2:** WASAPI shared-mode latency measurement on representative gaming hardware. If >30ms, switch to kira before signature sound is finalized.
- **Phase 3:** Steam binary VDF achievement-state schema; CreamAPI per-appid file format; SmartSteamEmu per-persona folder layout. All need live-installation validation.

Phases with standard patterns (skip research-phase):
- **Phase 1:** notify, sysinfo, rusqlite, Tauri Rust backend — well-documented.
- **Phase 4:** NSIS installer, tauri-plugin-updater, GitHub Actions release flow — standard recipes.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against current crate registries / official docs as of May 2026 |
| Features | HIGH | Cross-checked against 4 prior-art tools (Achievement Watcher, SteamAchievementNotifier, RetroAchievements, Playnite SuccessStory) and console UX patterns |
| Architecture | HIGH | Standard event-pipeline + hexagonal adapter pattern; Tauri multi-window confirmed via official docs |
| Pitfalls | HIGH | Domain-specific, sourced from Achievement Watcher source/wiki, Microsoft Win32 docs, Avalonia/Tauri issue trackers |

**Overall confidence:** HIGH

### Gaps to Address

- **Goldberg JSON field name** (`earned` vs `earned: 1` vs `achieved`) — confirm against achievement-watchdog source or live Goldberg install during Phase 1.
- **CreamAPI per-appid file schema** — empirical inspection during Phase 3 plan-phase research.
- **SmartSteamEmu per-persona folder layout** — empirical inspection during Phase 3 plan-phase research.
- **WASAPI shared-mode latency** — measure on target hardware during Phase 2; fallback to kira if rodio cannot hit ~30ms.
- **Rarity data presence in `appcache/stats`** — small spike against 10–20 games before committing to tier popup feature; degrade gracefully if absent.
- **`WS_EX_TRANSPARENT` + popup event behavior** — confirm popup needs no OS events before enabling click-through.
- **Windows 11 Fullscreen Optimizations coverage** — test across DX9/DX11/DX12 titles to validate exclusive-fullscreen scope assumption.

## Sources

### Primary (HIGH confidence)
- Tauri v2 official docs + GitHub releases — current version + multi-window + plugin-updater
- crates.io / docs.rs — notify 8.2, rodio 0.22.2, sysinfo 0.39, rusqlite, windows-rs
- Steam Web API documentation — GetSchemaForGame, GetGlobalAchievementPercentagesForApp
- Goldberg SteamEmu official README — default save paths, `local_save.txt` redirect
- Microsoft Win32 docs — `WS_EX_NOACTIVATE`, `WS_EX_TRANSPARENT`, `WS_EX_TOPMOST`, DPI awareness, `ReadDirectoryChangesW`
- Tauri GitHub issues #7519, #11566 — focus-stealing bug confirmation

### Secondary (MEDIUM confidence)
- Achievement Watcher (xan105) source/wiki — debounce constant (600ms), supported emulator paths, duplicate-suppression logic
- SteamAchievementNotifier issues — popup queue + 100% celebration ordering
- RetroAchievements overlay — companion HUD pattern
- Avalonia issue #17097 — `WM_MOUSEACTIVATE` / `MA_NOACTIVATE` requirement
- Achievement-watchdog source — Goldberg JSON schema corroboration
- Real-world Tauri overlay case study (MTG Arena, 2026) — 14MB RAM measurement

### Tertiary (LOW confidence)
- Community forum reports of WASAPI shared-mode latency 83–93ms on Realtek-equipped gaming laptops — needs in-house measurement
- CreamAPI / SmartSteamEmu file format details — sparse public documentation; validate empirically

---
*Research completed: 2026-05-07*
*Ready for roadmap: yes*

# Roadmap: Hallmark

## Overview

Hallmark ships in four coarse phases. Phase 1 lays the load-bearing detection pipeline: the Goldberg adapter, watcher core, first-launch state seeding, path discovery, and SQLite store — the foundation every other feature depends on. Phase 2 builds the entire premium UI layer on top of a stable event stream: popup overlay, companion window, game-session detection, and schema/icon resolution, making the hero feature verifiable end-to-end via Goldberg events. Phase 3 completes adapter coverage by adding the Steam-legit binary VDF parser plus the CreamAPI and SmartSteamEmu adapters, sealing the cross-source dedup contract. Phase 4 wraps with polish triggers and the full distribution pipeline, producing a public GitHub release.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Detection Pipeline Foundation** - Watcher core, Goldberg adapter, first-launch seeding, path discovery, SQLite store (completed 2026-05-08)
- [x] **Phase 2: Premium UI — Popup, Companion & Game Session** - Popup overlay with signature sound, companion window, game-session detection, schema/icon resolution
 (completed 2026-05-08)
- [x] **Phase 3: Remaining Source Adapters** - Steam-legit binary VDF adapter, CreamAPI adapter, SmartSteamEmu adapter, cross-source dedup
 (completed 2026-05-09)
- [x] **Phase 4: Polish & Distribution** - Test popup trigger, start-with-Windows, NSIS installer, auto-updater, GitHub Actions release pipeline, first-run wizard
 (completed 2026-05-09)

## Phase Details

### Phase 1: Detection Pipeline Foundation
**Goal**: A reliable, spam-free unlock event stream is flowing end-to-end for Goldberg-emulated games, with correct first-launch baseline seeding, 500ms debounce, and cross-source dedup — ready for a UI layer to consume.
**Depends on**: Nothing (first phase)
**Requirements**: DETECT-01, DETECT-05, DETECT-06, DETECT-07, DETECT-08
**Success Criteria** (what must be TRUE):
  1. Dropping a pre-populated Goldberg `achievements.json` for a known appID into the watched directory — then marking one entry as earned — produces exactly one unlock event in the CLI test harness within one second, with no duplicate events for 5 seconds afterward.
  2. When Hallmark starts with an already-populated Goldberg save directory (achievements earned before install), zero historical unlock events are emitted; only net-new changes after startup trigger events.
  3. A game whose Goldberg installation uses a `local_save.txt` redirect (non-default AppData path) is discovered automatically and its achievements are watched without any manual configuration.
  4. Unlocking the same achievement simultaneously via two simulated adapter sources (cross-source dedup test) produces exactly one event, not two.
  5. All discovered paths (Steam library folders, Goldberg default + redirect paths) are logged at startup so silent zero-popup failures can be diagnosed.
**Plans**: 5 plans
  - [x] 01-01-PLAN.md — Tauri/Rust scaffold + dep pinning + A4 Goldberg schema empirical check
  - [x] 01-02-PLAN.md — SourceAdapter trait + RawUnlockEvent types + SqliteStore + 001 migration
  - [x] 01-03-PLAN.md — Path discovery (registry + libraryfolders.vdf both locations + local_save.txt resolution)
  - [x] 01-04-PLAN.md — GoldbergAdapter (baseline seed + content-hash dedup) + WatcherCore (notify-debouncer-full 500ms)
  - [x] 01-05-PLAN.md — CrossSourceDedup TTL stage + hallmark-cli binary + integration tests for all 5 success criteria

### Phase 2: Premium UI — Popup, Companion & Game Session
**Goal**: A real achievement unlock from the Phase 1 pipeline fires a premium PS5-style popup overlay with signature sound on the correct monitor, the companion window auto-shows and lists earned achievements when a game is running, and the system handles queue bursts, DPI, rarity display, and 100% completion without dropping events or stealing focus.
**Depends on**: Phase 1
**Requirements**: POPUP-01, POPUP-02, POPUP-03, POPUP-04, POPUP-05, POPUP-06, POPUP-07, POPUP-08, COMP-01, COMP-02, COMP-03, GAME-01, GAME-02, GAME-03
**Research flag**: WASAPI shared-mode latency — measure rodio audio latency on representative gaming hardware before finalizing the signature sound implementation; fall back to kira if latency exceeds ~30ms. Also validate that `WS_EX_NOACTIVATE` post-creation HWND patch fully prevents focus-steal across DX11/DX12 borderless-windowed titles.
**Success Criteria** (what must be TRUE):
  1. When a Goldberg achievement unlocks, a popup appears within one second on the monitor where the game window is displayed, shows the achievement icon, title, and description, plays the signature sound, animates in and out, and the game window never loses focus.
  2. Unlocking five achievements in rapid succession queues all five popups; each displays sequentially with no dropped entries, and if the game hits 100% the celebration popup appears last in the queue.
  3. When a game launches, the companion window appears automatically, lists the full achievement set (earned entries marked), and when the game closes the companion window hides; restarting Hallmark mid-session restores the "earned this session" list from SQLite.
  4. On a 4K / high-DPI display the popup renders at the correct physical size with no pixelation or truncation; rarity percentage is shown when sourced from Steam `appcache` global stats and gracefully absent when unavailable; rare achievements display the richer animation/sound treatment.
  5. Schema (display name, description, icon) for the running game is resolved and cached in SQLite before the first popup fires, so popups appear without a loading delay after the first session with that game.
**Plans**: 7 plans
  - [x] 02-01-PLAN.md — Foundation: Cargo deps + frontend scaffold (Vite + React 19 + Framer Motion) + SQLite migration 002 + Tauri config (CSP + capabilities) + lib.rs Phase 2 module stubs
  - [x] 02-02-PLAN.md — Schema resolution (D-24 lookup chain): cache.rs query helpers + steam_api.rs (no-key Web API) + appcache.rs + goldberg_meta.rs + SchemaCache orchestrator + classify_tier
  - [x] 02-03-PLAN.md — Game detection (D-21 hybrid) + Win32 monitor placement (POPUP-03): game_detect/{mod,process_scan,steam_state}.rs + monitor.rs HWND/MonitorFromWindow + paths.rs visibility tweak
  - [x] 02-04-PLAN.md — Audio dispatcher (POPUP-06): rodio 0.22 AudioDispatcher + Tier enum + 3 bundled SFX assets (placeholder synthesis script for Phase 4 polish)
  - [x] 02-05-PLAN.md — Popup overlay hero feature (POPUP-01..08): ui.rs window builders + WS_EX_NOACTIVATE HWND patch + popup_queue.rs (adaptive compression + 100% appended-last) + React PopupCard + PS5 Pure CSS
  - [x] 02-06-PLAN.md — Companion window (COMP-01..03): 3 Tauri commands (get_companion_state + prefs CRUD) + AppState + 7 React components (header/list/filter/sort/skeleton/empty) + companion CSS
  - [x] 02-07-PLAN.md — Final integration: lib.rs setup() wires all 4 tokio tasks + windows + AppState + invoke_handler + game-started listener spawning schema::resolve; 5 integration tests
**UI hint**: yes

### Phase 3: Remaining Source Adapters
**Goal**: Achievement unlocks from legitimate Steam installations (binary VDF), CreamAPI, and SmartSteamEmu all flow through the same pipeline and fire the same premium popup as Goldberg, with no duplicate popups when multiple adapters observe the same logical unlock.
**Depends on**: Phase 2
**Requirements**: DETECT-02, DETECT-03, DETECT-04
**Research flag**: HIGH — Steam binary VDF achievement-state schema (field names, encoding, timestamp fields) and CreamAPI per-appid file format and SmartSteamEmu per-persona folder layout all require empirical validation against live installations during planning. Do not assume Goldberg JSON field names apply.
**Success Criteria** (what must be TRUE):
  1. Unlocking an achievement in a legitimate Steam game (with Steam client running) fires a Hallmark popup within one second, identical in quality to a Goldberg unlock, with no manual path configuration required.
  2. CreamAPI and SmartSteamEmu installs are detected automatically by path discovery and their unlocks fire the same premium popup.
  3. When a legitimate Steam game is also running a Goldberg or CreamAPI emulator alongside it (unusual but real-world), exactly one popup fires per logical unlock — not two or three.
**Plans**: 5 plans
  - [x] 03-00-PLAN.md — Spike: confirm appcache/stats path empirically, fix REQUIREMENTS.md DETECT-02, write empirical-vdf-NOTES.md, add Cargo deps (byteorder + crc32fast), extend SourceKind enum + DiscoveredPaths struct, declare 4 stub source modules
  - [x] 03-01-PLAN.md — SteamLegit adapter + hand-rolled binary VDF reader (vdf_binary.rs) + path-discovery extension reading HKCU registry user IDs (DETECT-02)
  - [x] 03-02-PLAN.md — CreamAPI adapter + 12-LoC INI parser + path-discovery extension enumerating %APPDATA%/CreamAPI/<appid>/ (DETECT-03)
  - [x] 03-03-PLAN.md — SmartSteamEmu adapter (stats.bin variant) + 24-byte record parser + lazy CRC32 to API-name reverse lookup + path-discovery extension (DETECT-04)
  - [x] 03-04-PLAN.md — Wire 4 adapters in lib.rs::run(), integration tests for all 3 ROADMAP success criteria including 3-source dedup verification

### Phase 4: Polish & Distribution
**Goal**: Any user can install Hallmark from a GitHub Release via a double-click NSIS installer or a portable zip, verify their installation fires a popup immediately via the test trigger, opt into start-with-Windows, receive in-app update prompts, and be guided through path discovery on first run — making the public release genuinely usable without a README deep-dive.
**Depends on**: Phase 3
**Requirements**: POL-01, POL-02, DIST-01, DIST-02, DIST-03, DIST-04
**Success Criteria** (what must be TRUE):
  1. A user can click "Fire test popup" from the tray menu and see a sample unlock popup fire through the full pipeline, confirming their install is working without needing to trigger a real game unlock.
  2. The start-with-Windows toggle in settings causes Hallmark to start automatically on login (registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry written/removed); toggling it off removes the entry cleanly.
  3. A GitHub Release tag push triggers a GitHub Actions workflow that produces both an NSIS installer `.exe` and a portable `.zip`, attaches them to the release, and publishes `latest.json` for the auto-updater.
  4. When a newer version is available on GitHub Releases, Hallmark prompts the user to install the update in-app via `tauri-plugin-updater`; the update installs and restarts without requiring manual download.
  5. On first launch, the path-discovery wizard scans for Steam library folders and Goldberg/CreamAPI/SmartSteamEmu installations and presents what was found (or not found), so users with zero detected paths see an immediate actionable message rather than silent failure.
**Plans**: 15 plans (8 original + 7 gap-closure)
  - [x] 04-01a-PLAN.md — Foundation A: deps + Vite multi-entry + capabilities + 7 module stubs + queries.rs first_run helpers + types.ts extension
  - [x] 04-01b-PLAN.md — Foundation B: tauri.conf.json bundle + CSP + lib.rs setup() spine + AppState extension + 4 commands + plugin registration
  - [x] 04-02-PLAN.md — Tray icon + autostart (HKCU\Run via winreg) + Quit-with-drain (POL-02 surface)
  - [x] 04-03-PLAN.md — Test-popup synthetic injector + portable mode detector (POL-01)
  - [x] 04-04-PLAN.md — Settings window + Updater background-check + Update modal in companion (DIST-02)
  - [x] 04-05-PLAN.md — First-run wizard window (DIST-04)
  - [x] 04-06-PLAN.md — GitHub Actions release pipeline + portable .zip + Ed25519 keypair generation + README polish (DIST-01, DIST-03)
  - [x] 04-07-PLAN.md — Final SFX assets (procedural via gen_placeholder_sfx.rs retune)
  - [x] 04-08-PLAN.md — Gap closure: test_trigger timestamp-suffixed api_name + popup_queue display fallback (UAT test 4 root cause #1)
  - [x] 04-09-PLAN.md — Gap closure: Vite optimizeDeps + WebView ready handshake (popup_ready / wizard_ready / settings_ready Notify gates) (UAT test 4 root cause #2 + test 14 #1)
  - [x] 04-10-PLAN.md — Gap closure: settings.css surface regression patch (reset, scroll containment, sticky header, skeleton mirror, scrollbar styling) + 3 drag-region attribute additions (UAT tests 3, 6, 7, 14)
  - [x] 04-11-PLAN.md — Gap closure: tauri-plugin-shell wiring (Cargo + npm + Builder + capability least-privilege allowlist + Settings/UpdateModal openExternal) (UAT test 6 dead links)
  - [ ] 04-12-PLAN.md — Gap closure: CheckOutcome tagged enum (no_release / offline / platform_missing / other_error) — Settings Updates panel error wording (UAT test 9)
  - [x] 04-13a-PLAN.md — Gap closure: tray.rs Hallmark header drop + 04-CONTEXT D-01 amendment (UAT test 2 root cause #1; autonomous, wave 1)
  - [x] 04-13b-PLAN.md — Gap closure: tray.ico/icon.ico real artwork (UAT test 2 root cause #2; human-action checkpoint, wave 1)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Detection Pipeline Foundation | 5/5 | Complete | 2026-05-08 |
| 2. Premium UI — Popup, Companion & Game Session | 7/7 | Complete   | 2026-05-08 |
| 3. Remaining Source Adapters | 5/5 | Complete   | 2026-05-09 |
| 4. Polish & Distribution | 8/15 | UAT gaps pending | 2026-05-09 (gap closure in progress) |

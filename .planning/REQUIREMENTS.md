# Requirements: Hallmark

**Defined:** 2026-05-07
**Core Value:** Make PC achievement unlocks feel as satisfying as a PS5 trophy ding — every time, in every supported game.

## v1 Requirements

### Detection

- [x] **DETECT-01**: Real-time watcher detects unlocks from Goldberg SteamEmu output, including default `%APPDATA%\Goldberg SteamEmu Saves\` and `local_save.txt` redirected paths
- [ ] **DETECT-02**: Real-time watcher detects unlocks from legitimate Steam (binary VDF parser of `userdata/<steamid>/<appid>/remote/`, mtime trigger via `appcache/stats`)
- [ ] **DETECT-03**: Real-time watcher detects unlocks from CreamAPI per-appid output
- [ ] **DETECT-04**: Real-time watcher detects unlocks from SmartSteamEmu per-persona output
- [x] **DETECT-05**: First-launch state seeding — baseline existing achievement state from disk before attaching change handlers (no install-time spam of historic unlocks)
- [x] **DETECT-06**: 500ms debounce + content-hash equality check on file events (no double-popups for a single logical write)
- [x] **DETECT-07**: Cross-source duplicate suppression — one logical unlock observed by multiple adapters produces exactly one popup
- [x] **DETECT-08**: Path discovery — parse Steam `libraryfolders.vdf` (post-2022 location and legacy location) and discover Goldberg redirects via `local_save.txt` adjacent to `steam_api.dll`

### Popup

- [ ] **POPUP-01**: Premium signature-style popup renders achievement icon, title, description, in/out animation, and signature sound effect on every unlock
- [ ] **POPUP-02**: Popup queue handles close-succession unlocks without dropping entries; queued popups display sequentially with consistent timing
- [ ] **POPUP-03**: Popup appears on the monitor where the running game is displayed (multi-monitor aware via game window HWND lookup)
- [x] **POPUP-04**: Popup is DPI-aware and renders correctly on 4K and scaled displays
- [ ] **POPUP-05**: 100% completion celebration popup fires when a game's achievement set hits 100%; placed last in the queue if other unlocks are pending
- [ ] **POPUP-06**: Tier-based popup styling — rare unlocks receive richer animation/sound treatment; degrades gracefully to standard popup when rarity data is unavailable
- [ ] **POPUP-07**: Rarity percentage rendered in popup when sourced from Steam `appcache` global stats; rendered without rarity when data is unavailable
- [ ] **POPUP-08**: Popup uses external borderless always-on-top window (`WS_EX_TOPMOST`) with `WS_EX_NOACTIVATE` applied via `SetWindowLongW` post-creation to prevent focus-stealing from the running game

### Companion

- [ ] **COMP-01**: Companion window auto-shows when a game launches and auto-hides when the game closes
- [ ] **COMP-02**: Companion displays the current game's achievement list — earned and locked entries with icon, title, description
- [ ] **COMP-03**: Session unlock history persists to local SQLite, so a mid-game restart of Hallmark preserves the "earned this session" list

### Game Session & Schema

- [ ] **GAME-01**: Hybrid game-launch detection — read Steam currently-playing state when available; fall back to `sysinfo` process scanner with `appmanifest_*.acf` matching for Goldberg / non-Steam launches
- [ ] **GAME-02**: Achievement schema + icon resolution at game-launch time, async + non-blocking; popups using cached schema appear without latency
- [x] **GAME-03**: Schema metadata and achievement icons cached in local SQLite (`hallmark.db`); subsequent runs operate fully offline once cache is warm

### Polish

- [ ] **POL-01**: "Fire test popup" button in tray menu / settings — emits a sample unlock through the full pipeline so users can verify the install works without waiting for a real unlock
- [ ] **POL-02**: Start-with-Windows option (registry `HKCU\...\Run` entry, user-toggleable)

### Distribution

- [ ] **DIST-01**: NSIS installer + portable `.zip` build via Tauri bundler — both artifacts produced per release
- [ ] **DIST-02**: Auto-updater wired to GitHub Releases `latest.json` via `tauri-plugin-updater`; user is prompted to install available updates
- [ ] **DIST-03**: GitHub Actions release workflow (`tauri-action`) builds and attaches installer + portable artifacts on tag push
- [ ] **DIST-04**: First-run path-discovery wizard scans for Steam library folders and Goldberg installations and surfaces what was detected (reduces silent zero-popup failure on misconfigured installs)

## v2 Requirements

### Differentiators (deferred)

- **POPUP-V2-01**: Screenshot-on-unlock (capture game frame at unlock moment)
- **POPUP-V2-02**: Pinned "next unlock" surface in companion view (suggest easiest-next or rarest-next)
- **QOL-V2-01**: DND mode (silence popups during cutscenes / focused play / streaming)
- **QOL-V2-02**: Streamer / privacy mode (popup hidden from screen capture; sound retained)
- **QOL-V2-03**: Hidden-achievement spoiler protection (display `???` for hidden until unlock)

### Stores (deferred — community-extensible adapter pattern designed in v1)

- **STORE-V2-01**: Epic Games Store achievements adapter
- **STORE-V2-02**: GOG Galaxy achievements adapter
- **STORE-V2-03**: Xbox / Microsoft Store achievements adapter
- **STORE-V2-04**: Ubisoft Connect achievements adapter
- **STORE-V2-05**: EA App achievements adapter
- **STORE-V2-06**: Battle.net achievements adapter

### Reach (deferred)

- **REACH-V2-01**: DLL injection overlay path for exclusive-fullscreen reliability
- **REACH-V2-02**: Linux / Steam Deck support
- **REACH-V2-03**: macOS support

## Out of Scope

| Feature | Reason |
|---------|--------|
| Cloud sync, accounts, profiles | Local-only by design; user usage is moment-to-moment, not lifetime |
| Leaderboards, friends, social, comments | Not the driver; aggregator dashboards (Exophase, MetaGamerScore) already serve this |
| Theme presets / sound customization | Signature-style locked deliberately for brand identity; "premium feel" requires designer control |
| Lifetime stats dashboard | Out of user's actual usage pattern (session-focused) |
| Goldberg / CreamAPI / SmartSteamEmu setup assistance | Passive detection only; app does not install or configure emulators |
| Steam Web API as primary detection | 1–5 min polling lag breaks the popup feel; file watcher gives sub-second latency |
| DRM-protected achievement bypass tooling | Not a goal |
| Telemetry, usage analytics, crash reporting | Local-only stance — no outbound network beyond schema/icon fetch and update check |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DETECT-01 | Phase 1 | Complete |
| DETECT-02 | Phase 3 | Pending |
| DETECT-03 | Phase 3 | Pending |
| DETECT-04 | Phase 3 | Pending |
| DETECT-05 | Phase 1 | Complete |
| DETECT-06 | Phase 1 | Complete |
| DETECT-07 | Phase 1 | Complete |
| DETECT-08 | Phase 1 | Complete |
| POPUP-01 | Phase 2 | Pending |
| POPUP-02 | Phase 2 | Pending |
| POPUP-03 | Phase 2 | Pending |
| POPUP-04 | Phase 2 | Complete |
| POPUP-05 | Phase 2 | Pending |
| POPUP-06 | Phase 2 | Pending |
| POPUP-07 | Phase 2 | Pending |
| POPUP-08 | Phase 2 | Pending |
| COMP-01 | Phase 2 | Pending |
| COMP-02 | Phase 2 | Pending |
| COMP-03 | Phase 2 | Pending |
| GAME-01 | Phase 2 | Pending |
| GAME-02 | Phase 2 | Pending |
| GAME-03 | Phase 2 | Complete |
| POL-01 | Phase 4 | Pending |
| POL-02 | Phase 4 | Pending |
| DIST-01 | Phase 4 | Pending |
| DIST-02 | Phase 4 | Pending |
| DIST-03 | Phase 4 | Pending |
| DIST-04 | Phase 4 | Pending |

**Coverage:**
- v1 requirements: 28 total
- Mapped to phases: 28
- Unmapped: 0

---
*Requirements defined: 2026-05-07*
*Last updated: 2026-05-07 after roadmap creation (traceability filled)*

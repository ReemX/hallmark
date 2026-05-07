# Feature Research

**Domain:** Windows desktop achievement-notification + session companion app (PC gaming)
**Researched:** 2026-05-07
**Confidence:** HIGH — based on direct analysis of Achievement Watcher (xan105), SteamAchievementNotifier, PSerban93/Achievements, RetroAchievements overlay, PS5/Xbox UX patterns, and aggregator dashboards (Exophase, TrueSteamAchievements, AStats)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete or broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Icon + title + description in popup | Every console and every prior art ships this; missing = popup feels like a system toast | LOW | Achievement schema fetch required first. Icon is the most visually load-bearing element — blank icon breaks trust. |
| Popup slide-in / slide-out animation | Console popups (PS5, Xbox) train users to expect smooth motion; static popups feel like errors | LOW | Ease-in-out curve, ~300 ms in, ~200 ms out is the established console baseline. |
| Sound on unlock | PS5 trophy ding is the core emotional hook of the product; silent popup defeats the entire value proposition | LOW | Single curated sound, no user swap in v1 (signature lock). Volume respects Windows system mixer. |
| Popup duration and auto-dismiss | Users expect popup to leave on its own; stuck popup = perceived crash | LOW | 5–8 s is the console convention. No manual dismiss needed if duration is tuned correctly. |
| Popup queue — sequential display for rapid unlocks | Unlocking 3 achievements in 2 seconds (common in story games) must show all 3 in order, not drop 2 | MEDIUM | Queue serializes onto a single display slot. 100% popup must be placed last in queue. Achievement Watcher and SteamAchievementNotifier both had bugs here before explicit queue logic was added. |
| Achievement icon, title, description fetched from schema per game | Users expect named achievements, not IDs; "Achievement 42" is hobby-grade | MEDIUM | Schema comes from Steam appcache (`AppInfo.vdf`) or a local cache built at first run. Goldberg/CreamAPI `.json` files include names inline so emulator path is easy; legit Steam path requires schema parse. |
| Achievement icon local cache (offline capable) | Popup must fire during gameplay, often on metered connections or offline; network fetch on unlock = delay or blank | MEDIUM | Download icons once at schema-fetch time, store on disk. Cache invalidation can be lazy (never, or on app update). |
| Session companion view — list of unlocks earned this session | Users who session-hunt want a single-glance "what did I get tonight" list; no list = "was that real?" after each popup | MEDIUM | Session starts on game-launch detection; entries appended in real time. Must survive app-restart mid-game (see unlock-history persistence). |
| Unlock-history persistence | If app restarts mid-game, session list must still know which achievements were earned before the restart, not re-notify them | MEDIUM | Persist unlock timestamps per appID to local SQLite or flat JSON. Compare file-watcher state to persisted state on startup to suppress duplicate notifications. Achievement Watcher explicitly calls this out as a requirement for Steam emulators that overwrite timestamps. |
| "Currently playing" detection — auto-show companion on game launch | Without this, companion must be opened manually; users won't bother; companion becomes vestigial | MEDIUM | Hybrid: Steam IPC / registry state for legit games (gives appID), OS process scanner for emulated/non-Steam. Already committed to in PROJECT.md. |
| Companion view auto-hide on game close | Companion cluttering the desktop after the game exits is friction | LOW | Debounce: 10–30 s after process gone before hiding, in case of crash-restart. |
| Start-with-Windows option | Achievement notifier is a background utility; expecting users to manually launch it before every session = high drop-off | LOW | Windows Task Scheduler or HKCU Run key. System-tray icon to indicate running state. |
| Achievement completion percentage per game in companion | "How far am I in this game?" is the first question users ask while looking at their unlock list | LOW | Computed locally from schema count vs unlock count. No external call. Requires schema fetch to know total count. |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valued. Hallmark's signature identity lives here.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Tier-based popup — rare achievements get visually distinct treatment | Rare unlocks (< 10% global unlock rate) feel more earned; a uniform popup erases that distinction; SteamAchievementNotifier's "Rare" type is its most praised feature | MEDIUM | Requires rarity data. Source: Steam `GetSchemaForGame` or local `stats/achievements.bin` file — but PROJECT.md bans Steam Web API. Rarity must come from `appcache/stats` local file (available for installed games) or be omitted gracefully when unavailable. See dependency note below. |
| Rarity percentage displayed in popup | PS5 shows "X% of players earned this trophy" — users actively cite this as satisfying; aggregators (Exophase, TrueSteamAchievements) built entire products around it | LOW (once data is available) | Data source constraint: same as tier-based. Display is trivial once the number is known. If rarity unavailable, omit field gracefully — do not show "0%" or "--". |
| 100% / all-achievements celebration popup | Completing a game is a major milestone; a standard popup for the final achievement is anticlimactic; SteamAchievementNotifier ships a dedicated "100%" notification type | MEDIUM | Requires knowing total achievement count (schema). Must always be last in queue. Distinct animation and potentially longer duration (10–12 s vs 6 s). |
| Companion view — "pinned next unlock" (upcoming achievements) | Power users want to know what they're working toward; Achievement Watcher does not have this; RetroAchievements HUD shows locked achievements with rarity | MEDIUM | Show top 3–5 locked achievements, sorted by unlock rate descending (easiest first) or user-pinned. Requires schema + current unlock state. Read-only view — no guide integration in v1. |
| Multi-monitor positioning — popup follows the game's display | On dual-monitor setups the game is rarely on the primary monitor; popup on the wrong monitor is invisible | MEDIUM | Enumerate displays via Windows API, detect which monitor has the focused game process, position overlay there. SteamAchievementNotifier had explicit bugs around non-primary monitors before fixing this. |
| Popup positioned at a fixed corner (not OS notification area) | OS notification area (bottom-right Action Center) can be obscured by taskbar, games, or other toasts; a dedicated overlay window at a predictable corner builds spatial muscle memory | LOW | Already committed via external borderless always-on-top window in PROJECT.md. The design decision also bypasses the Windows toast system entirely, giving full rendering control. |
| Session companion auto-appearance is animated / polished | Companion sliding in from the edge on game launch reinforces the premium feel; sudden appearance feels like a system dialog | LOW | Slide-in / fade-in on show, slide-out / fade-out on hide. ~250 ms. Low complexity once window management is in place. |
| System-tray presence with right-click controls | Power users expect pause/resume notification, open companion, quit — without navigating to a main window | LOW | Standard Windows system-tray API. Minimal menu. |
| DND mode — silence popups during user-defined windows or on hotkey | Streamers and users in cutscenes need to suppress notifications without closing the app; Re-enabling is one click | MEDIUM | Global hotkey toggle (e.g. Ctrl+Shift+H). State shown in tray icon. Unlocks during DND are still queued (so companion reflects them) but no popup fires. Alternatively: never queue during DND, simpler but loses "missed" unlocks. The queue approach is more honest. |
| Streamer mode — suppress achievement name/description in popup | Achievement names can spoil game content on stream (e.g. "You killed the secret boss"); streamers want the ding without the spoiler | LOW | Show only game name + icon, replace title/description with "Achievement Unlocked" or similar. Toggle per-session. Does not require detecting streaming software — user-controlled. |

### Anti-Features (Deliberately Excluded from v1)

Features that are commonly requested but wrong for this product at this stage.

| Feature | Why Requested | Why Problematic for v1 | What to Do Instead |
|---------|---------------|------------------------|-------------------|
| User-editable themes, sound swaps, animation customization | Every existing tool (SteamAchievementNotifier, Achievement Watcher) offers this; users assume it | Destroys the "designer-controlled signature style" that is Hallmark's primary differentiator; making the popup configurable makes it a canvas, not a premium product | Lock the style. Accept that this is a deliberate brand choice. Revisit in v2 if the locked style is itself a complaint. |
| Screenshot / video capture on unlock | Achievement Watcher and SteamAchievementNotifier both ship this; power users want a "souvenir" | Explicitly Out of Scope in PROJECT.md. Adds GPU/encoding complexity (NVENC/AMF), privacy surface, and storage management that are out of proportion with v1 scope | Users can use GeForce Experience / Xbox Game Bar / Shadowplay independently. |
| Social features — friends, leaderboards, community comparison | Aggregator dashboards (Exophase, TrueSteamAchievements) build on this; users familiar with those sites expect it | Requires cloud, accounts, sync — all explicitly Out of Scope in PROJECT.md. Fundamentally changes the app from a local notification tool into a social platform | Out of scope by design. Local-only is the promise. |
| Lifetime stats dashboard — total achievements, completion rate across all games | Users who come from aggregator sites expect this | Not the usage pattern Hallmark is designed for; session-focused is the identity. Lifetime stats pull users away from playing into browsing | Companion view covers the in-session view. Users who want lifetime stats already have Exophase, AStats, TrueSteamAchievements. |
| Achievement guides integration — link to external walkthroughs | Power users want context for "pinned next unlock" | Requires web scraping or API agreements with guide sites (PowerPyx, TrueAchievements, etc.), introduces external dependencies, and crosses the line from notification tool into browser replacement | Show the achievement description (already in schema). A separate browser tab for guides is fine. |
| Multi-store support — Epic, GOG, Xbox/MS Store, EA App, Ubisoft | Users with multi-store libraries want unified coverage | Out of Scope in PROJECT.md. Each store has a different file path convention, API, and schema format. Each adds a non-trivial detection module. | Architecture is source-adapter extensible. Community can contribute later. |
| Fake achievements / "create your own achievement" | Hobbyist communities do this | Outside the product's identity and creates abuse/confusion vectors | Out of scope entirely. |
| DRM bypass assistance or emulator setup guidance | Some users conflate "supports Goldberg" with "helps set up Goldberg" | Explicitly Out of Scope in PROJECT.md. App is passive — reads existing emulator output paths if they exist. No setup, no documentation of how to use emulators. | Passive detection only: if the files are there, they work. If not, app ignores them silently. |
| Progress achievements (partial unlock bars) | Steam shows "you've earned X of Y" progress notifications | Requires Steam's internal progress tracking API, not surfaced in local achievement files. Not in the real-time file-watcher model. | Out of scope for v1. Full achievement unlock is the atomic event Hallmark handles. |
| Cloud sync of unlock history | Users who play on multiple PCs want persistent history | Explicitly Out of Scope in PROJECT.md. Adds auth, sync conflict resolution, server costs | Local-only SQLite/JSON is sufficient for the session model. |

---

## Feature Dependencies

```
[Achievement schema fetch per game]
    └──required by──> [Icon + title + description in popup]
    └──required by──> [Achievement icon local cache]
    └──required by──> [Completion percentage in companion]
    └──required by──> [Pinned next unlock in companion]
    └──required by──> [100% celebration popup]
    └──enables──>     [Tier-based / rarity popup] (only if rarity field present in local schema)

[Popup duration decision]
    └──required by──> [Popup queue] (queue interval = popup duration + gap)
    └──required by──> [100% celebration popup] (longer duration variant)

[Unlock-history persistence]
    └──required by──> [Session companion list] (survive mid-session app restart)
    └──required by──> [Duplicate-suppress logic] (don't re-notify already-seen unlocks)

["Currently playing" detection]
    └──required by──> [Session companion auto-show]
    └──required by──> [Session companion auto-hide]
    └──required by──> [Schema fetch trigger] (fetch on game launch, not on every file event)

[DND mode]
    └──enhances──> [Popup queue] (queue held during DND, flushed or discarded on DND exit)
    └──enhances──> [System tray] (tray icon reflects DND state)

[Rarity data available in local schema]
    └──required by──> [Rarity % in popup]
    └──required by──> [Tier-based popup treatment]
    └──note──>        Steam appcache contains rarity for installed games;
                      Goldberg .json files do not contain global rarity data.
                      Feature must degrade gracefully when data absent.

[Multi-monitor detection]
    └──required by──> [Popup positioned on game's monitor]
    └──enhances──>    [Screenshot with overlay] (out of scope v1, noted for v2)
```

### Dependency Notes

- **Schema fetch required before popup can show content:** The file watcher fires instantly, but popups need the achievement's display name, description, and icon URL. Schema should be fetched (and cached) at game-launch detection time, not at unlock time, to avoid popup latency.
- **Popup queue requires a settled popup duration:** Queue spacing is `popupDuration + dismissGap`. If duration is variable, queue logic becomes complex. A fixed duration (6 s standard, 10 s for 100%) simplifies this substantially.
- **Rarity data is opportunistic:** Available in Steam's local appcache for installed games, absent for emulator-only installs. All rarity-dependent features must have a graceful no-rarity fallback. Do not call the Steam Web API for this (banned in PROJECT.md constraints).
- **100% popup must always be last in queue:** When the final achievement triggers 100%, multiple achievements may unlock simultaneously. The 100% popup must be queued after all individual achievement popups. This is a known bug in SteamAchievementNotifier's early versions.
- **Duplicate suppress is a correctness requirement, not a nice-to-have:** Steam emulators (Goldberg, CreamAPI) overwrite achievement timestamps on every write, which means a file watcher event fires on every save. Without deduplication keyed on (appID, achievementID), users see repeated popups for already-earned achievements when a game saves. Achievement Watcher documents this explicitly.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept.

- [ ] File watcher detects unlock events (legit Steam + Goldberg/CreamAPI paths)
- [ ] Achievement schema fetched and cached locally at game-launch time
- [ ] Achievement icon cached locally
- [ ] Popup fires with icon, title, description, and sound
- [ ] Popup has enter/exit animation and auto-dismisses after fixed duration
- [ ] Popup queue — sequential display, no drops, 100% last
- [ ] Unlock deduplication (suppress already-seen achievements on watcher restart)
- [ ] Unlock-history persistence (session survives app restart mid-game)
- [ ] "Currently playing" detection (hybrid: Steam state + process scanner)
- [ ] Session companion view — auto-shows on launch, auto-hides on close
- [ ] Session companion lists unlocks earned this session with icon + name
- [ ] Completion percentage per game in companion
- [ ] Start-with-Windows option
- [ ] System-tray icon with pause/resume/quit

### Add After Validation (v1.x)

Features to add once core popup loop is proven satisfying.

- [ ] Tier-based popup treatment for rare achievements — add once rarity data sourcing is confirmed reliable from local appcache
- [ ] Rarity percentage displayed in popup — depends on above
- [ ] 100% celebration popup — distinct animation, longer duration, triggering on game completion
- [ ] DND mode — hotkey toggle, tray indicator, queue held during DND
- [ ] Multi-monitor positioning — popup follows the game's display
- [ ] Pinned next unlock in companion (top 3–5 locked achievements, easiest first)
- [ ] Streamer mode — suppress achievement name/description, show only icon

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] DLL injection overlay — covers exclusive-fullscreen games; significant anti-cheat and per-renderer complexity
- [ ] Additional store source-adapters (GOG, Epic, Xbox) — community-contributed per architecture
- [ ] Screenshot / video capture on unlock — GPU encoder integration, storage management
- [ ] macOS / Linux ports — reduce v1 surface area

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Popup: icon + title + description | HIGH | LOW | P1 |
| Popup: sound | HIGH | LOW | P1 |
| Popup: animation in/out | HIGH | LOW | P1 |
| Popup: auto-dismiss / duration | HIGH | LOW | P1 |
| Popup queue (rapid unlocks) | HIGH | MEDIUM | P1 |
| Unlock deduplication | HIGH | MEDIUM | P1 |
| Achievement schema fetch + cache | HIGH | MEDIUM | P1 |
| Achievement icon local cache | HIGH | MEDIUM | P1 |
| Unlock-history persistence | HIGH | MEDIUM | P1 |
| Game launch detection | HIGH | MEDIUM | P1 |
| Session companion auto-show/hide | HIGH | MEDIUM | P1 |
| Session companion unlock list | HIGH | MEDIUM | P1 |
| Completion percentage in companion | MEDIUM | LOW | P1 |
| Start-with-Windows | HIGH | LOW | P1 |
| System-tray presence | MEDIUM | LOW | P1 |
| Tier-based / rare popup treatment | HIGH | MEDIUM | P2 |
| Rarity % in popup | MEDIUM | LOW (once data available) | P2 |
| 100% celebration popup | HIGH | MEDIUM | P2 |
| DND mode | MEDIUM | MEDIUM | P2 |
| Multi-monitor positioning | MEDIUM | MEDIUM | P2 |
| Pinned next unlock (companion) | MEDIUM | MEDIUM | P2 |
| Streamer mode | LOW | LOW | P2 |
| Session companion animated entrance | LOW | LOW | P2 |

---

## Competitor Feature Analysis

| Feature | Achievement Watcher (xan105) | SteamAchievementNotifier | RetroAchievements overlay | Hallmark approach |
|---------|------------------------------|--------------------------|--------------------------|-------------------|
| Popup style | Windows toast (OS-rendered) | Custom Electron window, fully themeable | Embedded RetroArch OSD | External borderless always-on-top window, signature style locked |
| Sound | Configurable (any file) | Configurable (any file, randomized mode) | Emulator audio system | Single curated sound, locked for brand identity |
| Schema source | Steam API + local cache | Steamworks API (no key) | RetroAchievements server | Local file only (appcache + emulator JSON) — no API polling |
| Rarity display | Not prominent | Rare type (< 10%) gets distinct notification | Shown in companion HUD | Tier-based + % display, data from local appcache |
| 100% completion | Not distinct | Dedicated 100% notification type | "Mastery" alert | Dedicated celebration popup, last in queue |
| Popup queue | Present (had early bugs) | Present (had early bugs, now fixed) | Emulator-handled | Explicit queue required; 100% always last |
| Session companion | Not present | Achievement Stats Overlay (progress bar only) | Locked/unlocked list in HUD | Full session list, pinned next unlocks, completion % |
| Game detection | File path heuristics | Steamworks API (legit Steam only) | Emulator-native | Hybrid: Steam state + process scanner |
| Multi-monitor | Not documented | Fixed in a patch release | N/A (in-emulator) | Popup follows game's display |
| Streamer mode | Not present | Stream Notifications window for OBS | Not present | Toggle: suppress title/desc, show icon only |
| DND | Not present | Not present | Not present | Hotkey toggle, queue held |
| Start with Windows | Background watchdog process | Electron auto-launch | N/A | Task Scheduler / Run key |
| Screenshot on unlock | Yes (screenshot + GPU video) | Yes (screenshot with overlay) | Not present | Out of scope v1 |
| Customization | Extensive (hobby-grade) | Extensive (hobby-grade) | Limited | None — signature style |
| Emulator support | Core feature (Goldberg, CreamAPI, etc.) | Not supported | RetroAchievements only | Goldberg + CreamAPI + SmartSteamEmu (passive detection) |

---

## Sources

- [Achievement Watcher by xan105 — GitHub](https://github.com/xan105/Achievement-Watcher)
- [Achievement Watcher Wiki: Options](https://github.com/xan105/Achievement-Watcher/wiki/Options)
- [Achievement Watcher Wiki: Toast Notification](https://github.com/xan105/Achievement-Watcher/wiki/Toast-notification)
- [SteamAchievementNotifier — GitHub](https://github.com/SteamAchievementNotifier/SteamAchievementNotifier)
- [SteamAchievementNotifier README](https://github.com/SteamAchievementNotifier/SteamAchievementNotifier/blob/master/README.md)
- [PSerban93/Achievements — GitHub](https://github.com/PSerban93/Achievements)
- [RetroAchievements overlay discussion — RAIntegration](https://github.com/RetroAchievements/RAIntegration/discussions/756)
- [RetroAchievements OBS configuration guide](https://docs.retroachievements.org/general/tutorials/how-to-configure-obs-studio.html)
- [Playnite SuccessStory plugin — GitHub](https://github.com/Lacro59/playnite-successstory-plugin)
- [PS5 trophy notifications — Stevivor](https://stevivor.com/news/ps5-trophy-notifications-pop-top-right-screen-record-video/)
- [PS5 system update trophy hunter reengagement — Tom's Guide](https://www.tomsguide.com/news/the-ps5-system-update-has-made-me-a-trophy-hunter-again)
- [Steam "Rare Achievement" threshold discussion — PSNProfiles forum](https://forum.psnprofiles.com/topic/143090-sony-changed-the-maximum-%E2%80%9Ctrophies-in-a-row%E2%80%9D-pop-up-notification/)
- [Steam achievement rarity display — ResetEra](https://www.resetera.com/threads/does-game-pass-reduce-the-novelty-of-%E2%80%9Crare-achievement%E2%80%9D-popups.554329/)
- [Steamworks Stats & Achievements documentation](https://partner.steamgames.com/doc/features/achievements)

---

*Feature research for: Windows desktop achievement-notification + session companion app (Hallmark)*
*Researched: 2026-05-07*

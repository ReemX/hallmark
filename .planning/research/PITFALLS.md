# Pitfalls Research

**Domain:** Windows desktop achievement notifier / overlay app (Steam + Steam emulators)
**Researched:** 2026-05-07
**Confidence:** HIGH (domain-specific; cross-verified against Achievement Watcher source/wiki, Goldberg readme, .NET docs, Windows overlay documentation)

---

## Critical Pitfalls

### Pitfall 1: First-Launch State Seeding — "Everything Just Unlocked"

**What goes wrong:**
On first run, the file watcher sees no prior known-state for achievement files. If the watcher reads current file contents and treats every already-earned achievement as a new event, the user gets spammed with dozens of popups for achievements they earned months or years ago. This is catastrophic for first impressions.

**Why it happens:**
Developers write the "watch for change → fire popup" path first, then discover the seeding problem only when they test on an account that already has achievements. It's invisible in unit tests because tests start with empty state.

**How to avoid:**
Before starting any file watcher, read all existing achievement files and build an in-memory baseline state (a set of already-unlocked achievement IDs per appID). Only fire popups for IDs that transition from `earned: false` to `earned: true` relative to that baseline. Serialize this baseline to disk so it survives app restarts (a simple JSON file under `%APPDATA%\Hallmark\state\` keyed by appID + source). On restart, load the baseline before attaching watchers.

**Warning signs:**
- Spam of popups on app launch in testing
- User reports "got 200 achievement notifications immediately after installing"
- Baseline file is missing or zero-byte on first run

**Phase to address:**
File watcher / detection foundation phase. This must be solved before any popup logic is wired up — solve it day one of implementing the watcher or every subsequent test will be polluted.

---

### Pitfall 2: FileSystemWatcher Multi-Fire — Debounce Without Losing Real Unlocks

**What goes wrong:**
Windows `FileSystemWatcher` fires multiple `Changed` events for a single logical write. Steam and emulators often write achievement files in 2–4 steps: open → partial write → close → rename from temp. This fires Created + Changed + Renamed events. Without debouncing, the app processes the same unlock 2–4 times, firing duplicate popups.

**Why it happens:**
`FileSystemWatcher` reflects the raw OS filesystem events, not logical file operations. The .NET documentation explicitly warns: "Common file system operations might raise more than one event." Developers who don't know this ship duplicate-popup bugs immediately.

**How to avoid:**
Implement a per-file debounce timer (400–600ms window is safe; Achievement Watcher uses 600ms). When a file event fires, cancel any pending timer for that file path and restart it. Only process the file when the timer fires without a new event. Track a `lastProcessedHash` (MD5 or SHA1 of file contents) per file to skip re-processing identical contents. Both together (timer + hash) eliminate both duplicates and idempotent re-reads.

**Warning signs:**
- Two popups appearing a fraction of a second apart for one unlock
- Logs showing 3–5 file events per single achievement write
- InternalBufferSize overflow warnings (default 8KB; increase to 64KB for libraries with many games)

**Phase to address:**
File watcher foundation phase. Implement debounce in the same pass as the watcher, not as a later fix.

---

### Pitfall 3: File Locked / Partial Read During Write

**What goes wrong:**
If the app reads an achievement file at the exact moment the game is writing it, the read either throws `IOException` (file locked) or returns partial/corrupt JSON that crashes the parser. This causes silent failures or crashes that are hard to reproduce.

**Why it happens:**
Games and emulators hold the file open for the full write cycle. The file watcher fires the event at the start of the write, not after it completes. A naive immediate read hits the open file handle.

**How to avoid:**
Wrap all file reads in a retry loop with a short delay (e.g., 3 attempts × 50ms sleep). Catch `IOException` and `UnauthorizedAccessException` separately. Also validate JSON parse before consuming: if `JsonDocument.Parse()` throws, discard the read and try again on the next debounce tick. For Steam's binary VDF stats files (`stats.bin`/`achievements.dat`), treat parse failures as transient — never propagate them as "achievement state = unknown."

**Warning signs:**
- Occasional `IOException: The process cannot access the file` in logs
- JSON parse exceptions that are intermittent and unreproducible on demand
- Achievement popups sometimes missing even though file changed

**Phase to address:**
File watcher foundation phase. Part of the initial robustness work.

---

### Pitfall 4: Exclusive Fullscreen Overlay Invisibility

**What goes wrong:**
An external `WS_EX_TOPMOST` borderless window — Hallmark's chosen overlay approach — is invisible to the user when a game runs in true exclusive fullscreen mode. The window exists and is topmost, but the game has taken exclusive control of the display adapter. No popup appears, no error is thrown. The app looks broken.

**Why it happens:**
DirectX exclusive fullscreen bypasses the Windows DWM compositor entirely. Topmost windows only work when DWM is compositing. Modern Windows 10/11 "Fullscreen Optimizations" silently converts many exclusive fullscreen games to optimized borderless, mitigating this significantly — but legacy games and DX9 titles still use true exclusive fullscreen.

**How to avoid:**
Document this as a known v1 limitation: "Popups are not visible in exclusive fullscreen mode. Use borderless windowed or windowed mode for the overlay to appear." Detect at startup whether Fullscreen Optimizations are enabled globally (registry key `HKCU\System\GameConfigStore\GameDVR_FSEBehaviorMode`). Display a one-time warning in the companion UI if the detected game window is in exclusive fullscreen mode. v2 can address with DLL injection/hook, but that's explicitly out of scope.

**Warning signs:**
- Users report "app is running but no popup appears" for games set to exclusive fullscreen
- Works in borderless windowed but not fullscreen mode
- No exception thrown — silent failure

**Phase to address:**
Overlay rendering phase. Document the limitation in the first popup implementation. The companion UI should surface it.

---

### Pitfall 5: Focus Stealing — Popup Pulls Focus from Game

**What goes wrong:**
The popup window activates when shown, yanking keyboard focus away from the game. The player's character stops moving, a keybind triggers in the wrong context, or the game minimizes. This is a game-breaking regression that makes the app feel actively hostile.

**Why it happens:**
Default `Window.Show()` or `CreateWindowEx` activates the new window. Developers test with desktop apps where focus doesn't matter, not during active gameplay.

**How to avoid:**
Set `WS_EX_NOACTIVATE` in the extended window style at creation time. In WPF, override `ShowActivated = false` and also handle `WM_MOUSEACTIVATE` to return `MA_NOACTIVATE` — without the message handler, mouse clicks on the popup steal focus even when `ShowActivated` is false. Use `SetWindowPos` with `SWP_NOACTIVATE` flag for repositioning. Never call `Window.Activate()` or `Focus()` on the overlay window. Test specifically: launch a game, trigger a popup, verify WASD input continues working uninterrupted.

**Warning signs:**
- Game pauses or input stops when popup appears
- Popup appears but game loses keyboard focus
- Avalonia/WPF issue: `Focusable: false` in XAML is not sufficient alone (confirmed bug in Avalonia — `WS_EX_NOACTIVATE` must be set explicitly via platform interop)

**Phase to address:**
Overlay rendering phase. The window creation code must set this flag before any popup is shown. It is not a "later polish" item — it makes the app unsafe to use while gaming if missed.

---

### Pitfall 6: Goldberg Save Path Non-Discovery — Silently Watches Wrong Location

**What goes wrong:**
App watches only the default Goldberg path (`%APPDATA%\Goldberg SteamEmu Saves\<appid>\`) but misses unlocks because the game's `local_save.txt` redirects saves to a directory beside the game executable. This is a silent failure: no error, no popup, no indication anything is wrong.

**Why it happens:**
Goldberg supports a `local_save.txt` file placed alongside `steam_api(64).dll` that overrides the default save location with a relative path beside the DLL. Many piracy scene releases use this feature, so the majority of real-world Goldberg games use custom paths, not the default. Achievement Watcher supports custom folder scanning as a manual workaround; Hallmark needs automatic discovery.

**How to avoid:**
When a Goldberg appID is detected (game process running + `steam_api.dll` / `steam_api64.dll` present in game directory), check for `local_save.txt` beside the DLL. If present, resolve the path specified in `local_save.txt` relative to the DLL location and add that as an additional watch path. Also check `%APPDATA%\Goldberg SteamEmu Saves\<appid>\`, `%APPDATA%\EMPRESS\<appid>\steam_settings\`, and `%PUBLIC%\EMPRESS\<appid>\steam_settings\`. Treat all discovered paths as peers — whichever one changes first wins for that session.

**Warning signs:**
- Goldberg detection claims to work but no popup fires for certain games
- User reports "works for one game but not another"
- `local_save.txt` exists in game folder but is not read

**Phase to address:**
Emulator path discovery phase (part of file watcher work). Every supported emulator path variant must be resolved before watching begins.

---

### Pitfall 7: Steam Library Moves / Multi-Library userdata Path Staleness

**What goes wrong:**
The app hard-codes `C:\Program Files (x86)\Steam\userdata\` or reads the Steam install path from registry once at startup. When the user moves their Steam library to another drive or adds a secondary library, the app watches the wrong path and misses real Steam achievements. Additionally, Steam changed the path of `libraryfolders.vdf` in a mid-2022 update from `SteamApps\` to `config\`, breaking tools that hard-coded the old path.

**Why it happens:**
Steam's installation path is user-configurable; secondary libraries can be on any drive. The VDF path for library configuration also changed in a Steam update, breaking older parsers.

**How to avoid:**
Read the Steam install path from `HKCU\Software\Valve\Steam\SteamPath` registry key. Then parse `<SteamPath>\config\libraryfolders.vdf` (check both new path and legacy `<SteamPath>\SteamApps\libraryfolders.vdf` for fallback). Handle both old format (flat key-value) and new format (nested object with `path` and `mounted` fields). `userdata\` is always under the main Steam install path, not under library folders — keep this distinction. Validate each library path exists before watching. Re-read `libraryfolders.vdf` if it changes (watch it too).

**Warning signs:**
- Works on default Steam install but fails when Steam is on `D:\`
- Works on single-library installs but misses games installed on secondary library
- Exception when parsing VDF after a Steam update

**Phase to address:**
Steam path discovery phase. Must be done before any real-user testing — nearly all power gamers have Steam on a non-C drive.

---

## Moderate Pitfalls

### Pitfall 8: DPI Scaling — Popup Looks Tiny or Blurry on 4K

**What goes wrong:**
On a 4K display at 150–200% scaling, a DPI-unaware or system-DPI-aware overlay window renders at 96 DPI and gets bitmap-scaled by Windows, producing a blurry, physically tiny popup. On a per-monitor setup where the game is on a 4K screen and the app's main window is on a 1080p screen, the overlay positions on the wrong monitor's DPI context.

**Why it happens:**
The default DPI awareness mode in .NET WPF is "System DPI Aware" (not per-monitor). An overlay that sets no DPI awareness is bitmap-scaled. The developer tests on a single 1080p monitor and never encounters the issue.

**How to avoid:**
Declare `PerMonitorV2` DPI awareness in the app manifest (`<dpiAwareness>PerMonitorV2</dpiAwareness>`). In WPF, handle `WM_DPICHANGED` to re-layout the popup when it moves to a different DPI monitor. Use `GetDpiForMonitor()` with the HWND of the popup to get the correct scale factor for rendering. Size the popup in logical pixels and let the framework scale; avoid hard-coded pixel dimensions. Test on a 4K + 1080p dual-monitor setup before release.

**Warning signs:**
- Popup looks correct on dev machine but "tiny" or "blurry" in user screenshots
- User reports popup on the wrong monitor
- DPI value returned is always 96 regardless of actual monitor DPI

**Phase to address:**
Overlay rendering phase. DPI awareness must be declared in the manifest at project creation — it cannot be added properly after the fact.

---

### Pitfall 9: Multi-Monitor — Popup Appears on Wrong Screen

**What goes wrong:**
The popup appears on the primary display while the game is running on a secondary display. In a multi-monitor setup where the primary is a vertical productivity monitor and the secondary is the gaming display, the popup is never seen during play.

**Why it happens:**
`SystemParameters.WorkArea` returns the primary monitor's area. Most overlay examples position to the primary monitor without considering where the game actually is.

**How to avoid:**
Enumerate monitors with `MonitorFromWindow()` using the game's HWND (obtained via process detection) to find the game's monitor, then query that monitor's work area with `GetMonitorInfo()`. If the game's HWND cannot be determined, fall back to the monitor containing the largest portion of the foreground window at popup-show time. Cache the result per game session; re-query only if the game window moves.

**Warning signs:**
- Popup works on single-monitor but wrong position on multi-monitor
- User reports "I can see it minimized in the taskbar but it's not on my game screen"

**Phase to address:**
Overlay rendering phase. Multi-monitor targeting is part of initial popup placement work.

---

### Pitfall 10: Always-On-Top Conflicts with Other Overlays

**What goes wrong:**
Discord overlay, NVIDIA GeForce Experience overlay, RTSS (RivaTuner Statistics Server), and Steam overlay all compete for topmost Z-order. Hallmark's popup can appear under Discord or be obscured by RTSS. In some configurations, the topmost fight causes flickering.

**Why it happens:**
Multiple applications with `WS_EX_TOPMOST` fight for Z-order. The last window to call `SetWindowPos(..., HWND_TOPMOST, ...)` wins momentarily, but another overlay's next redraw can push it back.

**How to avoid:**
This is partially unavoidable at v1 with an external window approach. Mitigate by calling `SetWindowPos` with `HWND_TOPMOST` at the moment the popup is shown (not at window creation), immediately before the animation starts. This maximizes the chance Hallmark wins the Z-order fight at the critical moment. Document in the README that if another overlay consistently covers Hallmark, the user should try disabling hardware-accelerated overlays (Discord: Settings → Advanced → Hardware Acceleration → Off) or placing Hallmark later in startup order.

**Warning signs:**
- Popup appears behind Discord overlay in testing
- Popup visible on desktop but not above game
- Z-order flickers during popup appearance

**Phase to address:**
Overlay rendering phase. Partially mitigated by show-time topmost assertion; fully mitigated only in v2 with DLL injection.

---

### Pitfall 11: Steam Hidden Achievements — Spoiler Reveal on Unlock

**What goes wrong:**
Steam allows developers to mark achievements as hidden. The Steam schema API returns placeholder text (`"hidden": 1`, name/description redacted or generic) for unearned hidden achievements. When a hidden achievement unlocks, the local file reveals only the achievement API name (e.g., `ACH_WIN_100_GAMES`) but no display name or description — because the schema may not have been fetched for hidden achievements. The popup renders with an ugly internal key as the title.

Alternatively, if the schema was fetched and cached before the unlock, the popup renders the full spoiler text that the game developer deliberately hid.

**Why it happens:**
The Steam Web API returns `"hidden": 1` with no description for locked hidden achievements. After unlock, the same API call returns full text. Tools that cache the schema at game launch cache the empty version and never re-fetch.

**How to avoid:**
When the schema for an achievement has `"hidden": 1` and the display name is the API key, re-fetch the schema for that specific achievement after the unlock event (single targeted API call). Implement a post-unlock schema refresh that runs in the background and updates the popup if the name arrives within the popup's display window. If the re-fetch fails or times out, display a generic "Achievement Unlocked" with the game name only — never display the raw API key.

**Warning signs:**
- Popup shows `ACH_SECRET_ENDING` instead of the achievement name
- Schema cache has empty display names for hidden achievements
- Re-fetch after unlock returns 404 or empty (private profile)

**Phase to address:**
Schema fetching and caching phase.

---

### Pitfall 12: Steam Web API Rate Limits — Schema Fetch Bursts

**What goes wrong:**
On first run with many watched games, the app fetches schemas for all games in parallel, hitting Steam's API rate limit (429). The IP gets temporarily banned for 6+ hours. All schema fetches fail, all game icon downloads fail, and the app looks broken. Subsequent runs hit the same problem if the cache is not persisted properly.

**Why it happens:**
The Steam Web API has an undocumented dynamic rate limit. Burst requests (e.g., 50 games × schema + icon = hundreds of requests in seconds) reliably trigger it. Official documentation says 100,000 requests/day but the practical per-minute burst limit is much lower and varies by time of day.

**How to avoid:**
Schema fetches must be serialized with a minimum inter-request delay (minimum 1 request/second; 2-second gap is safer). Prioritize the currently-running game first; defer all others to an idle background queue. Persist the schema cache to disk in `%APPDATA%\Hallmark\cache\schema\<appid>.json` so a clean app restart does not re-trigger mass fetches. Cache with an expiry of 7+ days (schemas change rarely). Implement exponential backoff on 429 responses. Since Hallmark is local-file-only for detection, schema fetches are a secondary enrichment step — they can fail silently without breaking unlock detection.

**Warning signs:**
- 429 HTTP responses during startup
- Schema cache files always zero-byte or missing on restart
- User reports "app worked yesterday but all achievements show no names today"

**Phase to address:**
Schema fetching and caching phase. Rate limit discipline must be in the initial schema implementation, not added after reports of bans.

---

### Pitfall 13: Achievement Icon Cache Bloat

**What goes wrong:**
Each Steam game has 30–200+ achievement icons, each typically 64×64 to 256×256 PNG, hosted on `cdn.akamai.steamstatic.com`. Naively downloading and persisting every icon for every watched game causes the cache to balloon to several gigabytes. Users with large libraries (500+ games) and many achievements (50+ per game) can see 10–20 GB of cached icons.

**Why it happens:**
Each icon URL is unique per achievement. Developers download all icons eagerly to avoid on-demand fetches during popups. 200 icons × 50KB avg × 200 games = 2 GB, which is nontrivial.

**How to avoid:**
Implement lazy icon loading: only fetch and cache the icon for the specific achievement being shown in a popup. Pre-fetch the icon for the current session's game (typically 30–200 icons) when the game launches, not for all watched games. Apply an LRU eviction policy capped at a configurable size (default: 500 MB). Store icons as WebP with lossy compression rather than raw PNG. On cache eviction, log which games were pruned so users understand the tradeoff.

**Warning signs:**
- `%APPDATA%\Hallmark\cache\icons\` folder grows without bound
- Slow disk I/O on first popup per game session (icon not yet cached)
- User reports "app is using 8 GB of disk space"

**Phase to address:**
Schema fetching and caching phase.

---

### Pitfall 14: Windows SmartScreen and AV False Positives

**What goes wrong:**
Users download Hallmark's `.exe` or installer from GitHub Releases, and Windows SmartScreen shows "Windows protected your PC — unrecognized app" with a hard Stop button. Less savvy users abandon the install. Worse, some antivirus products flag the binary as a trojan because it watches file system paths and creates topmost windows — both behaviors that malware also exhibits.

**Why it happens:**
SmartScreen uses download reputation tied to the binary's hash. Every new build starts with zero reputation. Unsigned binaries from new publishers get the worst treatment. AV heuristics flag "reads game files + injects overlay window" as suspicious regardless of intent.

**How to avoid:**
**Short term:** Document the SmartScreen bypass clearly in the README ("Click 'More info' → 'Run anyway'"). Include VirusTotal scan link in each GitHub Release. EV code signing eliminates SmartScreen for the app but costs $300–700/year — acceptable if the project gains traction, impractical at launch.
**Distribution framing:** Privacy-forward README language: "Hallmark reads only your local achievement files. It never connects to Steam's servers for authentication. It never transmits any data anywhere." Open-source mitigates fear but does not eliminate it for non-technical users.
**AV submissions:** After each release, submit the binary to Microsoft WDSI and major AV vendors for whitelisting.

**Warning signs:**
- User reports "my antivirus deleted it" immediately after discussion of the binary
- AV detections show up on VirusTotal for process injection or file read behaviors
- GitHub issues titled "is this a virus?"

**Phase to address:**
Distribution / release phase. Also: frame the README privacy section from the first public commit, before release.

---

### Pitfall 15: Goldberg unlock_time = 0 / Missing — False "Just Unlocked" State

**What goes wrong:**
Goldberg writes achievement records with `unlock_time: 0` or omits the `unlock_time` field entirely when the unlock time is unknown or when the game does not pass a timestamp. If the baseline state comparison uses `unlock_time > 0` as the "has been earned" signal, achievements with `unlock_time: 0` are treated as unearned, and the first file-change event fires a popup for them even if they were earned long ago.

**Why it happens:**
Developers assume `unlock_time = 0` means "not unlocked." In Goldberg's format, the earned state is in a separate `"earned": true` / `"Achieved": 1` boolean field, not inferred from the timestamp. Achievement Watcher has an explicit option to handle "emulator overwrites previous timestamp" cases.

**How to avoid:**
Never use `unlock_time` alone as the earned indicator. Use the dedicated boolean field (`"earned": true` in Goldberg's format, `"Achieved": 1` in Steam's stats VDF) as the primary indicator. Treat `unlock_time = 0` as "earned but timestamp unknown" — still seed it into the baseline and do not fire a popup for it. When comparing state after a file change, fire a popup only when the boolean field transitions from false → true, regardless of timestamp.

**Warning signs:**
- Popups fire on startup for Goldberg games where all achievements show `unlock_time: 0`
- Comparing lock/unlock state behaves correctly for Steam but wrong for Goldberg
- Logs show state comparison using timestamp field

**Phase to address:**
File watcher / state comparison phase. Part of the baseline seeding logic.

---

### Pitfall 16: Game Launcher Process vs. Actual Game Process

**What goes wrong:**
Steam state reports the game as running, but the executable Steam is tracking is the launcher stub (e.g., `EADesktop.exe` for EA games, or a Unity/Unreal launcher shim). The actual game process starts several seconds later as a child process. If game-running detection is tied to the launcher process, the watcher arms early and then misses the actual game's lifecycle. Conversely, if detection only looks at direct Steam-launched processes, games run via direct EXE (Goldberg-cracked, no Steam) are missed entirely.

**Why it happens:**
Many games use multi-process architectures: launcher → actual game. Steam's `GetCurrentGameLanguage()` and process monitoring hooks see the launcher process but not the child. Achievement Watcher's compatibility docs note this explicitly as a "hit and miss" situation.

**How to avoid:**
Implement hybrid detection as planned in the PROJECT.md: Steam IPC state when available (gives accurate appID), plus OS process tree scanning as fallback. For process-tree matching, when a known launcher process is detected, walk its child processes looking for the actual game binary. Maintain a small hardcoded list of known launcher-to-game mappings (EA App → game binary pattern). Accept that for unknown launchers, the watcher may arm slightly late — which is acceptable because achievement unlocks typically happen well into a session, not at launch.

**Warning signs:**
- Works for most games but misses all EA App titles
- Companion view shows game name but achievement list never populates
- Process list shows launcher process but game process is a child

**Phase to address:**
Process detection / companion view phase. Hybrid detection is already planned; the launcher-child resolution is the key implementation detail.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hard-code Steam install path to `C:\Program Files (x86)\Steam\` | No registry read needed | Breaks for every user with Steam on a non-default path — majority of power users | Never |
| Watch only `%APPDATA%\Goldberg SteamEmu Saves\` | Simple single-path watch | Misses `local_save.txt` redirected paths — breaks most piracy scene releases | Never |
| Download all icons on first run eagerly | Popups are always instant | Multi-GB cache bloat, API rate limit bans on first launch | Never |
| Single global debounce timer instead of per-file timer | Simpler code | A second game unlocking during the debounce window for the first game's file is silently dropped | Never |
| Use `unlock_time > 0` as the "earned" indicator | Simpler comparison | Breaks all Goldberg games that write `unlock_time: 0` | Never |
| No baseline seeding; treat all current state as new | Simpler watcher setup | "200 achievement spam" on first launch for any user with existing achievements | Never |
| `ShowActivated = true` on popup window | Default behavior | Steals keyboard focus from game — game-breaking regression | Never |
| System DPI awareness only (not per-monitor) | Default WPF behavior | Blurry/tiny on 4K, wrong position on multi-monitor | MVP only if tested single 1080p |
| Schema fetched on-demand with no caching | Always fresh data | Hits API rate limit constantly; slow popups | Never (cache always required) |
| No retry on file read `IOException` | Simpler read code | Random silent failures when file is locked mid-write | Never |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Steam Web API `GetSchemaForGame` | Fetching all watched games in parallel on startup | Serial queue with 1–2 second delay; currently-running game first |
| Steam Web API `GetSchemaForGame` | Caching only in-memory; lost on restart | Persist to `%APPDATA%\Hallmark\cache\schema\<appid>.json` with 7-day expiry |
| Steam Web API hidden achievements | Caching empty name at game launch, never re-fetching | Re-fetch specific achievement schema after unlock event |
| Goldberg `achievements.json` | Assuming path is always `%APPDATA%\Goldberg SteamEmu Saves\` | Check `local_save.txt` beside `steam_api.dll` for path override |
| `libraryfolders.vdf` | Hard-coding path or old format only | Check both `config\` and legacy `SteamApps\` paths; handle old and new VDF object formats |
| CreamAPI | Not watching `%APPDATA%\CreamAPI\<appid>\` | Add CreamAPI path as a peer watch target alongside Steam and Goldberg paths |
| EMPRESS Goldberg variant | Using standard Goldberg paths | Also check `%APPDATA%\EMPRESS\<appid>\steam_settings\` and `%PUBLIC%\EMPRESS\<appid>\steam_settings\` |
| Windows `FileSystemWatcher` | Default 8KB internal buffer | Set `InternalBufferSize = 65536` (64KB) to prevent buffer overflow in large game directories |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Eager icon download for all watched games at startup | Long startup, API ban, high disk I/O | Lazy load: only current session's game icons, LRU cache cap | Any user with 50+ watched games |
| Synchronous file read on FileSystemWatcher callback thread | UI freeze or watcher queue backup | Always read files on a background thread; marshal popup trigger to UI thread | Under any CPU load |
| Polling `libraryfolders.vdf` on a tight loop | Unnecessary disk I/O | Watch `libraryfolders.vdf` with its own watcher; only re-parse on change | Always — polling is wasteful |
| Spawning new `HttpClient` per schema fetch | Socket exhaustion (`SocketException`) | Single shared `HttpClient` instance (static or `IHttpClientFactory`) | Under concurrent fetch load |
| JSON parsing on every file event without hash check | Re-parsing identical files repeatedly (emulator writes same state multiple times) | Hash file contents before parse; skip if hash matches previous | Games that write achievement file frequently |
| NAudio cold-start audio underrun | Pop/click on first sound play after app cold-start | Pre-initialize audio device during app startup (warm the output stream silently) | Always on first popup after fresh launch |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| README does not address "is this a virus?" proactively | Users spread FUD; project gets killed by reputation | Add prominent "Privacy" section to README before first public release: local-only, no telemetry, no Steam auth |
| Binary published without VirusTotal scan link | AV false positives kill installs silently | Run VirusTotal on every GitHub Release artifact; link the scan result in the release notes |
| Watching paths outside user's own `%APPDATA%` and Steam install without documentation | AV heuristics flag broad directory watching | Scope watchers narrowly to discovered paths only; document all watched paths in README |
| Storing Steam API key in plain config file | Users paste their API key into a world-readable config | Store API key in Windows Credential Manager or at minimum user-profile-only permissions file; never in a shared or repo-tracked location |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No test / preview trigger | User installs app, earns an achievement, nothing happens — cannot distinguish "broken" from "working but no unlock yet" | Provide a "Send test popup" button in settings that fires a fake popup immediately |
| Silent schema fetch failure | Popup fires with raw achievement API key as title; user thinks app is broken | Show "Fetching achievement info..." placeholder text if schema is not yet loaded; retry in background |
| Companion view opens but immediately closes | Game process detected then immediately lost (launcher exits, game re-spawns as child) | Debounce game-closed events by 3–5 seconds before dismissing companion view |
| No indicator that file watcher is active | User cannot tell if the app is watching the right paths | Show watched paths in the companion view (collapsed by default, expandable) or in a diagnostics panel |
| Popup appears in middle of screen during a cutscene | Immersion broken at a scripted story moment | There is no universal fix, but respect a "do not disturb" mode that queues popups and shows them after the current in-game UI element clears (out of scope for v1 — document) |
| App processes unlock events for old achievements after reinstall | User uninstalled game, deleted save, reinstalled — old achievements file still present in %APPDATA% | Treat achievements earned before app installation date as already-seeded baseline |

---

## "Looks Done But Isn't" Checklist

- [ ] **Baseline seeding:** Does the first run of the watcher on an account with existing achievements produce zero popups? Verify with a test account that has 50+ existing unlocks.
- [ ] **Debounce:** Does a single unlock produce exactly one popup? Trigger a manual file write and check logs for event count.
- [ ] **Focus safety:** After a popup appears, is keyboard input still received by the game? Manually test in a real game — not a test harness.
- [ ] **Exclusive fullscreen detection:** Does the companion UI warn when the game is running in exclusive fullscreen? Test with a DX9 game in fullscreen mode.
- [ ] **Goldberg local_save.txt path:** Does a game with `local_save.txt` redirecting to `.\save\` produce a popup? Test with a Goldberg game that uses this config.
- [ ] **libraryfolders.vdf multi-library:** Does a game installed on a secondary Steam library (D: drive) trigger a popup? Test on a machine with a non-default Steam library.
- [ ] **DPI scaling:** Does the popup render sharply and at correct physical size on a 4K display at 150% scaling?
- [ ] **Hidden achievement schema:** Does a hidden achievement popup show a human-readable name (after re-fetch) rather than the raw API key?
- [ ] **Rate limit resilience:** Does the app handle a 429 from Steam API gracefully without crashing or spinning?
- [ ] **Test popup button:** Can the user fire a test popup from the settings UI without needing to actually unlock an achievement?

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| First-launch spam (no baseline seeding) | HIGH — requires seeding redesign | Add baseline state serialization; on next launch, seed from on-disk state; existing users need to re-seed once via a migration step |
| API rate limit ban | LOW | Implement backoff + wait; user is unaffected after ban expires; fix burst behavior in next release |
| Goldberg path miss | MEDIUM — requires path discovery redesign | Ship patch that adds `local_save.txt` resolution; existing installations need to re-discover paths |
| Focus stealing | MEDIUM — requires window recreation | `WS_EX_NOACTIVATE` cannot be added to an existing window; must destroy and recreate the overlay window with correct style; ship as hotfix |
| VDF parse failure after Steam update | LOW | Add fallback path check; Steam updates rarely change VDF paths but the 2022 `libraryfolders.vdf` move is a precedent |
| DPI blurriness | LOW | Adding per-monitor DPI awareness is a manifest + layout change; testable before ship |
| Icon cache bloat | LOW | Implement LRU eviction and cap; existing inflated caches need a one-time prune on update |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| First-launch baseline seeding | File watcher foundation | Zero popups on fresh install for account with 200+ existing achievements |
| FileSystemWatcher multi-fire debounce | File watcher foundation | Exactly 1 popup per unlock in logs |
| File locked / partial read | File watcher foundation | No IOException crashes during active game session |
| Exclusive fullscreen invisibility | Overlay rendering | Companion UI warning visible when game is in exclusive fullscreen |
| Focus stealing | Overlay rendering | WASD input continuous during popup display in real game |
| Goldberg local_save.txt path discovery | Emulator path discovery | Test game with `local_save.txt` redirect fires popup |
| Steam library path / VDF parsing | Steam path discovery | Game on secondary library fires popup |
| DPI scaling | Overlay rendering | Popup renders sharply at 4K 150% scaling |
| Multi-monitor popup placement | Overlay rendering | Popup appears on same monitor as game |
| Always-on-top conflicts | Overlay rendering (document mitigation) | Popup visible above Discord overlay in most configs |
| Hidden achievement spoiler handling | Schema fetching / caching | Hidden achievement popup shows real name after re-fetch |
| Steam API rate limits | Schema fetching / caching | App survives 429 without crashing; no burst requests at startup |
| Icon cache bloat | Schema fetching / caching | Cache does not grow unbounded; LRU eviction confirmed in tests |
| SmartScreen / AV false positives | Distribution / release phase | VirusTotal clean on release binary |
| Goldberg unlock_time = 0 | File watcher / state comparison | Goldberg game with `unlock_time: 0` does not fire popup on startup |
| Launcher vs. game process detection | Process detection / companion view | EA App game fires popup when achievement earned in child process |
| Silent failure UX | First-run / companion UI | Test popup button present and works with zero configuration |
| No visible watcher state | Companion UI / diagnostics | Watched paths visible in UI |

---

## Sources

- xan105/Achievement-Watcher Wiki — "Achievement do not unlock": https://github.com/xan105/Achievement-Watcher/wiki/Achievement-do-not-unlock
- xan105/Achievement-Watcher Wiki — Compatibility: https://github.com/xan105/Achievement-Watcher/wiki/Compatibility
- Goldberg Steam Emulator README: https://github.com/su6ur6an/goldberg_emulator/blob/master/Readme_release.txt
- Goldberg gbe_fork basic configuration: https://deepwiki.com/Detanup01/gbe_fork/2.2-basic-configuration
- SteamShutdown library path detection: https://deepwiki.com/akorb/SteamShutdown/2.2.2-library-path-detection
- Steam libraryfolders.vdf path change issue (HXE project): https://github.com/HaloSPV3/HXE/issues/218
- FileSystemWatcher multi-fire: https://failingfast.io/a-robust-solution-for-filesystemwatcher-firing-events-multiple-times/
- FileSystemWatcher debounce (Atomic Object): https://spin.atomicobject.com/2010/07/08/consolidate-multiple-filesystemwatcher-events/
- WS_EX_NOACTIVATE focus prevention: https://learn.microsoft.com/en-us/answers/questions/905270/prevent-the-window-from-stealing-focus
- Avalonia WS_EX_NOACTIVATE bug: https://github.com/AvaloniaUI/Avalonia/issues/17097
- Windows exclusive fullscreen / DWM overlay: https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/
- Per-Monitor DPI awareness: https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows
- VRR + overlay stutter: https://forums.developer.nvidia.com/t/fix-vrr-for-overlays-always-on-top-windows/296168
- Steam Web API rate limiting (429): https://steamcommunity.com/discussions/forum/1/601902348018676495/
- OBS layered window capture removal: https://docs.retroachievements.org/general/tutorials/how-to-configure-obs-studio.html
- Steam Achievement Notifier (stream mode reference): https://github.com/SteamAchievementNotifier/SteamAchievementNotifier
- SmartScreen reputation for open-source: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation
- NAudio audio pop/underrun: https://github.com/naudio/NAudio/issues/32
- Windows audio ducking: https://learn.microsoft.com/en-us/answers/questions/3924313/windows-11-audio-ducking-even-though-no-change-in
- Steam hidden achievements reveal: https://steamcommunity.com/groups/SteamClientBeta/discussions/3/6861841362663371591/
- CreamAPI path (`%appdata%/CreamAPI/<appid>/`): https://github.com/NaughtDZ/creamapi/blob/master/cream_api.ini

---
*Pitfalls research for: Windows desktop achievement notifier / overlay (Steam + Steam emulators)*
*Researched: 2026-05-07*

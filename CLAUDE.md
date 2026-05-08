<!-- GSD:project-start source:PROJECT.md -->
## Project

**Hallmark**

Hallmark is a Windows app that brings PSN/Xbox-grade achievement satisfaction to PC gaming. When a Steam achievement unlocks — whether from a legitimate copy or a Goldberg/CreamAPI-emulated copy — Hallmark fires a premium signature-style popup with a godly sound effect, and shows a session-focused companion view of the current game's achievements. For PC gamers who want the moment-to-moment payoff that current PC achievement systems rarely deliver.

**Core Value:** Make PC achievement unlocks feel as satisfying as a PS5 trophy ding — every time, in every supported game.

### Constraints

- **Platform**: Windows-only for v1 — where the games are, where the file paths and process model are stable.
- **Overlay tech**: External borderless always-on-top window, no DLL injection — trades exclusive-fullscreen edge cases for fast ship and zero anti-cheat risk.
- **Detection**: Local file watcher only (no Steam Web API in v1) — required for real-time popup latency, offline support, and emulator coverage in one mechanism.
- **Distribution**: Free, open-source on GitHub — community-extensible store coverage is the long-term path.
- **Goldberg / emulator stance**: Passive detection only — app reads emulator output paths if they exist; does not install, configure, or recommend emulator setup.
- **Customization**: Signature style locked — no user-editable themes, sounds, positions, or animations in v1.
- **Pace**: Hobby project — polish over speed; no fixed deadline.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Tauri | 2.11.1 | App framework, window management, IPC | Sub-10MB installer, ~14–50MB RAM, Rust backend is native, WebView2 renders animated UI with full CSS/JS power. External overlay window is first-class. Proven for game overlays in 2026. |
| Rust | stable (1.85+) | Backend, file watcher, process detection, audio, Win32 calls | Required by Tauri; also the right language for low-latency file watching (notify), process scanning (sysinfo), and raw Win32 HWND manipulation for focus suppression. |
| React + Vite | React 19, Vite 6 | Frontend UI framework | Best-in-class animation ecosystem (Framer Motion, CSS keyframes), fast HMR, familiar to most contributors. Tauri ships with React templates. |
| notify + notify-debouncer-full | 8.2.0 (stable) / 9.0.0-rc.4 (next) | File system watcher | Uses ReadDirectoryChangesW on Windows natively. RecommendedWatcher auto-selects best backend per platform. notify-debouncer-full handles rapid multi-write bursts without event spam. |
| rodio | 0.22.2 | Audio playback (WAV/OGG one-shot SFX) | Pure Rust, uses cpal → WASAPI on Windows. 0.22.x fixed buffer size defaults and mixer reliability. Plays concurrent sounds without clipping. No external DLL dependency. |
| sysinfo | 0.39.0 | Process enumeration and game detection | Cross-platform Rust API over EnumProcesses. Exposes process name, PID, and command-line args on Windows. Maintain one `System` instance and refresh on a 2–3 s poll interval. |
| tauri-plugin-updater | 2.10.1 | Auto-update via GitHub Releases | Bundled in the Tauri plugin workspace, uses cryptographic signatures, reads a `latest.json` hosted on GitHub Releases. Simpler than velopack for a single-developer OSS app. |
| Framer Motion | 12.x | Popup animation (enter/exit, spring physics) | Declarative enter/exit transitions with spring physics; ideal for PS5-style popup feel. Compiles into the WebView bundle — no native animation runtime needed. |
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| kira | 0.12.0 | Alternative audio if rodio latency unacceptable | kira has a dedicated audio thread, hard-realtime mixer, and more expressive per-sound control (volume ramps, pitch). Prefer over rodio if you find WASAPI shared-mode latency exceeds ~30ms in testing. |
| tauri-plugin-polygon | latest | Per-region click-through hit testing | Use if the polling-based `setIgnoreCursorEvents` workaround at 60fps proves CPU-visible on low-end machines. Defines polygon regions instead of polling cursor position. |
| windows-rs | 0.58+ | Raw Win32 API for HWND manipulation | Required for applying `WS_EX_NOACTIVATE` to the overlay window after creation (see Overlay Window section). Use via `windows::Win32::UI::WindowsAndMessaging`. |
| vdf-rs / steamfiles | community | Parsing Steam binary VDF (`UserGameStats_*.bin`) | If you decide to cross-validate unlock timestamps from Steam's appcache against file-watcher events. Low priority for v1; the file watcher is sufficient on its own. |
| Inno Setup | 6.x | Windows installer | Single-EXE installer, open source (MIT-like), produced by Tauri's `cargo tauri build` pipeline via `bundler = ["nsis"]` or `bundler = ["msi"]`. Inno Setup 6 is the dominant choice for OSS GitHub-distributed utilities. |
### Development Tools
| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo tauri dev` | Hot-reload development | Starts Rust backend + Vite frontend with live reload in WebView |
| `cargo tauri build` | Production build + installer | Produces signed NSIS (`.exe`) or WiX (`.msi`) installer, or `.zip` portable |
| GitHub Actions `tauri-action` | CI build + release | Official action that builds and uploads installer to GitHub Releases; integrates with tauri-plugin-updater's `latest.json` |
| `cargo clippy` + `cargo fmt` | Rust code quality | Run in CI; keep Rust code consistent |
| ESLint + Prettier | Frontend code quality | Standard for React/TypeScript projects |
## Installation
# Install Rust toolchain
# Install Tauri CLI
# Scaffold project (React + TypeScript + Vite)
# Core Rust dependencies (Cargo.toml)
# notify = "8.2"
# notify-debouncer-full = "0.7"  # Updated from 0.5: published stable, same notify 8.2 backend, audited 2026-05-08 (IN-05)
# rodio = { version = "0.22", features = ["vorbis", "wav"] }
# sysinfo = "0.39"
# windows = { version = "0.58", features = ["Win32_UI_WindowsAndMessaging"] }
# Frontend dependencies
## Alternatives Considered
| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Tauri v2 | Electron | If the team is JS/Node-only with zero Rust experience. Electron's ~200MB RAM / ~100MB installer is a significant penalty for a background utility running alongside games. |
| Tauri v2 | WPF / WinUI 3 | If animation richness requires DirectX compositing effects unavailable in WebView (e.g., DWM blur, GPU particles). WinUI 3 is still maturing (toolchain rough edges in 2026); WPF is XAML-only with limited modern animation. Neither gives you cross-contributor web skills. |
| Tauri v2 | Native Win32 C++ or Rust | If you need exclusive-fullscreen DLL-injection overlays (v2 scope). Overkill for v1 external overlay; loses all declarative UI tooling. |
| Tauri v2 | Avalonia | Avalonia is cross-platform .NET with good custom rendering, but the ecosystem for animated UI is thinner than web-based frameworks, and the team would need C# knowledge. |
| rodio 0.22 | kira 0.12 | kira is better for music/dynamic game audio. For one-shot SFX, rodio is simpler. Flip to kira if real-world WASAPI latency exceeds 30ms. |
| notify 8.2 | chokidar (Node.js) | Only if you chose Electron. In Tauri/Rust, notify is the natural choice and avoids Node process overhead. |
| tauri-plugin-updater | velopack | velopack has richer installer (MSI + startup shortcut) and faster delta updates, but is still pre-release (all versions are 0.0.x pre-release as of May 2026). Use it in v2 if you want a proper system-tray startup entry or silent background install. |
| Inno Setup / NSIS | MSIX | MSIX requires Windows Store signing or Developer Mode; adds friction for users. Inno Setup produces a familiar double-click `.exe` that works on all Windows 10/11. |
## What NOT to Use
| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Electron | ~150–300MB RAM idle while running alongside games; ~100MB installer; ships its own Chromium — wasted when WebView2 is already on every Windows 10/11 machine. | Tauri v2 |
| Steam Web API for real-time unlock detection | 1–5 minute polling lag breaks the popup feel; requires internet; doesn't cover Goldberg/emulated games; needs API key setup. | File watcher on local Steam `appcache/stats` and emulator paths |
| DLL injection overlay (v1) | Anti-cheat risk (VAC, Easy Anti-Cheat detect foreign DLLs); per-renderer complexity (DirectX 9/11/12/Vulkan); deferred intentionally. | External borderless always-on-top window (WS_EX_TOPMOST) |
| FMOD / irrKlang | FMOD is commercial (free tier has attribution requirements + binary size); irrKlang is proprietary closed-source. Neither fits an OSS project. | rodio (pure Rust, MIT) or kira (MIT) |
| WMI for process enumeration | WMI COM calls have 100–500ms startup latency on Windows and require COM initialization. sysinfo wraps EnumProcesses/NtQuerySystemInformation which is much faster. | sysinfo 0.39 |
| ETW for process detection | Event Tracing for Windows gives near-instant launch notifications but requires elevated privileges and significant boilerplate. Overkill for 2–3s poll intervals. | sysinfo polling loop |
| `focus: false` alone in Tauri config | Known open bug — Tauri windows steal focus on Windows regardless of this setting (GitHub issue #7519, #11566). Must combine with raw HWND `WS_EX_NOACTIVATE` via windows-rs. | windows-rs `SetWindowLongW` after window creation |
| velopack (v1) | All releases are pre-release 0.0.x versions as of May 2026; no stable semver guarantee. | tauri-plugin-updater 2.10.1 |
| `fs.watch` / chokidar | Node-only; not available in Tauri Rust backend. Chokidar has known polling fallbacks on Windows; notify uses ReadDirectoryChangesW natively. | notify-rs |
## Stack Patterns by Variant
- Create a secondary `WebviewWindow` with: `decorations: false`, `transparent: true`, `always_on_top: true`, `skip_taskbar: true`, `focused: false`
- After creation, get HWND via `window.hwnd()`, then call `SetWindowLongW(hwnd, GWL_EXSTYLE, existing_style | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT)` via windows-rs to prevent any focus steal and enable click-through
- Use `setIgnoreCursorEvents(true)` via JS API for a simpler click-through on transparent regions (sufficient for a non-interactive notification popup)
- Animate entry/exit entirely in the WebView with Framer Motion
- Create a standard `WebviewWindow` with: `decorations: false`, `always_on_top: true` — this window IS interactive (user scrolls achievements)
- Do NOT set `WS_EX_NOACTIVATE` here; users need to click it
- Show/hide on game launch/exit events emitted by the process scanner
- Use `RecommendedWatcher` from notify-rs in the Rust backend
- Watch paths listed in the "Emulator File Paths" section below
- Wrap with `notify-debouncer-full` at ~500ms debounce to suppress multi-write bursts (Steam sometimes writes the stats file several times per unlock)
- On debounced event, diff the previous achievement state against current to find newly unlocked achievements
- Keep a single `rodio::OutputStream` alive for the process lifetime (dropping it silences audio)
- Pre-load sound into a `Rodio::Decoder` from a bundled asset at startup; clone the `SoundData` for each play call
- Call `sink.append()` from the Rust command handler to play without blocking the event loop
- Poll `sysinfo::System::refresh_processes()` every 2–3 seconds on a dedicated `tokio` task
- On each refresh, scan process names against a known-game list (built from Steam ACF manifests + Goldberg save directories)
- To map a PID to a Steam appID: (1) read command-line args for `AppId=` or the path to a game's install dir, (2) scan `<steam_root>/steamapps/appmanifest_*.acf` files for matching `installdir`, (3) fall back to checking if the process path is inside a known Goldberg save path
## Emulator File Paths Reference
| Source | Watch Path | File Format | Notes |
|--------|-----------|-------------|-------|
| Steam (legitimate) | `%STEAM%\appcache\stats\UserGameStats_<userid>_<appid>.bin` | Binary VDF (KeyValue) | File is written on achievement unlock. Must parse binary VDF to extract unlock status. userid = Steam64 ID. |
| Goldberg SteamEmu | `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json` | JSON | JSON array of achievement objects. Field `"earned": true` or `"earned": 1` indicates unlock. Path confirmed by Goldberg Readme_release.txt and achievement-watchdog. |
| CreamAPI | `%APPDATA%\CreamAPI\<appid>\` | JSON (achievement data files within dir) | Config is INI (`cream_api.ini` next to DLL), but achievement state stored in per-appid folder. Watch directory for any JSON changes. |
| SmartSteamEmu | `%APPDATA%\SmartSteamEmu\<persona>\remote_<appid>\` | Unknown (binary/text) | Per-persona storage. Watch `%APPDATA%\SmartSteamEmu\` recursively. Achievement Watcher uses this path. |
| CODEX / SKIDROW | `%PUBLIC%\Documents\Steam\CODEX\<appid>\` or `%APPDATA%\Steam\CODEX\<appid>\` | JSON | Watch both paths; CODEX uses the same JSON schema as Goldberg. |
## Steam Achievement Schema (for Popup Metadata)
## Overlay Window Technical Details
## Version Compatibility
| Package | Compatible With | Notes |
|---------|----------------|-------|
| tauri 2.11.1 | tauri-plugin-updater 2.10.1 | Same major version; check plugins-workspace for lockstep releases |
| notify 8.2.0 | Rust 1.85+ (MSRV) | notify 9.0.0-rc.4 is available but pre-release; use 8.2.0 for stability |
| rodio 0.22.2 | cpal 0.16 | rodio 0.22 updated cpal dependency; do not mix older cpal |
| sysinfo 0.39.0 | Rust 1.85+ | Breaking API changes between 0.3x and 0.39; check migration if upgrading |
| windows-rs 0.58 | Tauri 2.x HWND | Tauri 2.x returns `HWND` from the `windows` crate; must match the same `windows` crate version used internally |
| kira 0.12.0 | cpal 0.15+ | If swapping from rodio to kira, verify cpal version alignment |
## Sources
- Tauri GitHub releases (`github.com/tauri-apps/tauri/releases`) — Tauri 2.11.1 confirmed May 6, 2026
- Tauri v2 docs (`v2.tauri.app/learn/window-customization/`, `/reference/config/#windowconfig`) — window config options
- Tauri GitHub issue #7519, #11566 — confirmed `focus: false` non-functional on Windows
- Tauri GitHub discussion #7951 — overlay notification workaround patterns
- `docs.rs/notify/latest/notify/` — notify 8.2.0 docs, Windows backend, large-file limitation
- `github.com/notify-rs/notify/releases` — notify 9.0.0-rc.4 (pre-release), stable 8.2.0
- `docs.rs/crate/rodio/latest/source/CHANGELOG.md` — rodio 0.22.2, buffer fix, mixer reliability
- `docs.rs/kira/latest/kira/` — kira 0.12.0, Windows WASAPI via cpal, format support
- `docs.rs/crate/sysinfo/latest/source/CHANGELOG.md` — sysinfo 0.39.0, Windows cmd-line confirmed
- `blog.manasight.gg/why-i-chose-tauri-v2-for-a-desktop-overlay/` — real-world Tauri v2 game overlay (2026), 14MB RAM on Windows 11
- Tauri vs Electron comparison (multiple sources) — confirmed Tauri WebView2 ~14–50MB RAM vs Electron ~200–300MB
- `github.com/xan105/Achievement-Watcher/wiki/Compatibility` — emulator scan paths reference
- `github.com/50t0r25/achievement-watchdog` — Goldberg path confirmed `%appdata%\GSE saves\`
- Steam community discussion on `appcache\stats\UserGameStats_USERID_APPID.bin` — confirmed binary VDF path
- `github.com/velopack/velopack/releases` — velopack confirmed all 0.0.x pre-release as of May 2026
- `v2.tauri.app/plugin/updater/` — tauri-plugin-updater 2.10.1 confirmed stable
- WebSearch: CreamAPI `%APPDATA%\CreamAPI\<appid>\`, SmartSteamEmu `%APPDATA%\SmartSteamEmu\`
- WebSearch: CODEX paths `%PUBLIC%\Documents\Steam\CODEX\` and `%APPDATA%\Steam\CODEX\`
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

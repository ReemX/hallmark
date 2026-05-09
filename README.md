# Hallmark

PSN/Xbox-grade achievement satisfaction for PC gaming. When a Steam achievement unlocks — whether from a legitimate copy or a Goldberg/CreamAPI-emulated copy — Hallmark fires a premium signature-style popup with a godly sound effect, and shows a session-focused companion view of the current game's achievements.

**Core value:** Make PC achievement unlocks feel as satisfying as a PS5 trophy ding — every time, in every supported game.

## Install

Hallmark ships as either a Windows installer or a portable .zip. Choose the one that fits.

### Installer (recommended)

1. Download `hallmark-setup.exe` from the [latest release](https://github.com/ReemX/hallmark/releases/latest).
2. Double-click the .exe. Windows SmartScreen will warn:
   > Windows protected your PC
   > Microsoft Defender SmartScreen prevented an unrecognized app from starting.
3. Click **More info**, then click **Run anyway**.

   *Why the warning?* Hallmark is unsigned for v1 — code-signing certificates cost $200+/year and are deferred until the project gains traction. The download is hash-verified by the auto-updater after first install (any future updates are signed by an Ed25519 key whose private half is held only by GitHub Actions). Notepad++, OBS Launcher, and many other open-source utilities ship unsigned for the same reason.
4. The installer drops Hallmark in `%LOCALAPPDATA%\Hallmark` — no admin prompt, no system-wide changes.
5. Pin the tray icon to the always-visible area of the notification tray. Right-click for the menu; left-click shows the companion window.

### Portable

1. Download `hallmark-portable-<version>.zip` from the same release page.
2. Extract anywhere — Downloads, Documents, a USB stick.
3. Run `hallmark.exe` directly. No installation step.
4. State (achievement history, schema cache, settings) persists at `%APPDATA%\com.hallmark.app\`. Same location whether running portable or installed — your data follows you.
5. **Auto-update is disabled in portable mode.** To upgrade, download the newer .zip and extract over the old folder. Your `%APPDATA%` state is preserved.

### First-run

On first launch, a one-time wizard scans for Steam, Goldberg, CreamAPI, and SmartSteamEmu directories on your system and shows what was detected. If nothing is found, the wizard suggests installing Steam or launching a game to populate the watch paths. You can dismiss it; if the next launch still detects zero sources, the wizard reappears.

## Auto-update

Installed copies (not portable) check for new releases on each Hallmark launch. If a newer version is published on the [Releases page](https://github.com/ReemX/hallmark/releases), the next time the companion window opens, an in-app modal prompts you to install. The update downloads, verifies the Ed25519 signature, replaces the binary, and restarts **Hallmark only** — your PC is unaffected.

Snooze the prompt with **Later** to defer until next launch.

Updates are stable-channel only. Pre-release tags on GitHub are not picked up by the updater.

## Supported sources

| Source | Path | Notes |
|--------|------|-------|
| Steam (legitimate) | `%STEAM%\appcache\stats\UserGameStats_<userid>_<appid>.bin` | Binary VDF |
| Goldberg SteamEmu | `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json` | JSON |
| CreamAPI | `%APPDATA%\CreamAPI\<appid>\` | JSON per-appid folder |
| SmartSteamEmu | `%APPDATA%\SmartSteamEmu\<persona>\remote_<appid>\` | Binary/text |

## Development

Use `cargo tauri dev` to launch with frontend hot-reload. **Do NOT use `cargo run` directly** — it skips the Vite frontend build and the popup will render as an empty gray rectangle.

### Prerequisites

- [Rust stable](https://rustup.rs/)
- [Node.js LTS](https://nodejs.org/) + [pnpm](https://pnpm.io/)
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/)
- Windows 10/11 (Windows-only app)

### Setup

```sh
pnpm install
cargo tauri dev
```

### Build

```sh
cargo tauri build
```

Produces a signed NSIS installer at `src-tauri/target/release/bundle/nsis/hallmark-setup.exe`. The GitHub Actions workflow (`release.yml`) builds and uploads installers + portable .zip + `latest.json` automatically on tag push.

## Contributing

Contributions are welcome. Open an issue or PR on [GitHub](https://github.com/ReemX/hallmark). The project follows a hobby-project pace — polish over speed, no fixed deadline.

## License

MIT — see [LICENSE](./LICENSE) if present, or assume MIT per `package.json`.

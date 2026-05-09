# Phase 4: Polish & Distribution - Research

**Researched:** 2026-05-09
**Domain:** Tauri 2.x desktop-app distribution + tray-utility polish + auto-update + first-run UX
**Confidence:** HIGH for plugin/CLI/config surface; MEDIUM on auto-launch crate's HKCU fallback semantics; LOW on portable-zip CI emission (no native bundle target — must be hand-rolled)

## Summary

Phase 4 wraps the Hallmark detection-pipeline + popup/companion stack (Phases 1-3, locked) with the surfaces a stranger needs to install, verify, configure, and update the app: a system tray icon + checkable autostart menu, a borderless rounded-card Settings window, a first-run path-discovery wizard, an in-app auto-updater modal, an NSIS installer + portable .zip dual-output GitHub release pipeline, and a final-asset SFX swap. The Tauri 2.11.1 + tauri-plugin-updater 2.10.1 stack pinned in CONTEXT.md is current and aligned with crates.io 2026 — no version drift to surface.

Three findings deserve up-front planner attention: (1) **`tauri-plugin-autostart` does NOT pin HKCU** — it delegates to the `auto-launch` crate which tries HKLM first and falls back to HKCU on permission failure. For a per-user no-UAC install (D-22), the fallback path is what we'd hit naturally, but D-07 explicitly says "never HKLM". The clean path is to skip the plugin and use `winreg` directly so HKCU is *guaranteed* not best-effort. (2) **Tauri does NOT natively emit a portable .zip bundle target** — only `nsis` and `msi`. DIST-01's portable .zip must be a custom CI step that zips the unsigned built `.exe` (and any sidecar assets). The `tauri-action` "uploadUpdaterJson" feature handles `latest.json`, but the portable .zip is YOUR responsibility. (3) **Test-popup injection at adapter→dedup boundary (D-04) needs a sender clone, not a parallel channel** — the existing `mpsc::Sender<RawUnlockEvent>` (`raw_tx` in `lib.rs::run`) is the natural seam; no architectural rework needed.

**Primary recommendation:** Use `winreg 0.56` directly for D-07 autostart (skip `tauri-plugin-autostart`). Use `tauri-plugin-updater 2.10.1` + GitHub Releases `download/latest.json` static endpoint. Build the portable .zip as a `windows-latest` `pwsh` post-step in the GH Actions workflow, archiving `target/release/hallmark.exe` + bundled `assets/`. Inject test popup via a clone of `raw_tx` stored in `AppState`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tray icon + menu | Rust (Tauri backend) | — | `tauri::tray::TrayIconBuilder` is OS-native; menu events handled in setup() closure |
| "Fire test popup" trigger | Rust (Tauri backend) | — | Synthesizes `RawUnlockEvent`, sends via `raw_tx` clone — same channel real adapters use |
| Start-with-Windows toggle | Rust (Tauri backend) | — | HKCU registry write via `winreg`; NO frontend involvement (tray-only per D-09) |
| Settings window UI | Rust (window builder) + Frontend (React) | — | Borderless rounded card built in `ui.rs` (matches popup/companion pattern); React renders content |
| First-run wizard UI | Rust (window builder) + Frontend (React) | — | Same builder pattern as Settings; SQLite `settings.first_run_done` flag is source-of-truth |
| Path-rescan command | Rust (Tauri command) | Frontend (invoke) | Calls existing `paths::discover()` — no new logic; React displays |
| Update check | Rust (Tauri backend via plugin) | Frontend (modal) | `app.updater().check().await` runs in Rust; React displays modal sheet over companion |
| Update install + restart | Rust (plugin) | — | `update.download_and_install()` + `app.restart()` |
| NSIS installer | Build pipeline (tauri-bundler) | — | Configured via `bundle.windows.nsis` in tauri.conf.json |
| Portable .zip | CI workflow (custom step) | — | Tauri does NOT emit portable bundles; custom `pwsh` zip step required |
| Code-signing of updater payload | CI (env var → tauri-bundler) | — | `TAURI_SIGNING_PRIVATE_KEY` env var; signing done by `tauri build`, not a separate step |
| GitHub Release publication | CI workflow (tauri-action) | — | `tauri-apps/tauri-action@v0` creates release + uploads NSIS + sigs + latest.json |
| Portable-mode detection | Rust startup logic | — | At `lib.rs::run()` startup, decide install-vs-portable via exe-path heuristic |
| SFX final assets | Build assets (filesystem) | — | Drop-in replacement at `assets/sfx/*.wav` — `audio.rs` API unchanged |

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| POL-01 | "Fire test popup" button in tray menu / settings — emits a sample unlock through the full pipeline | `tauri::tray::TrayIconBuilder` + `MenuBuilder` (Tauri 2.x docs); inject point: clone of `raw_tx` (mpsc::Sender<RawUnlockEvent>) — see Code Examples §Test-popup injection |
| POL-02 | Start-with-Windows option (registry HKCU\...\Run entry, user-toggleable) | `winreg 0.56` direct — `RegKey::open_subkey_with_flags(...).set_value("Hallmark", &cmd)`. Plugin alternative (`tauri-plugin-autostart 2.5.1`) does not guarantee HKCU and is rejected for D-07 — see Don't Hand-Roll table for reasoning |
| DIST-01 | NSIS installer + portable `.zip` build via Tauri bundler — both per release | NSIS: `bundle.targets: ["nsis"]` + `bundle.windows.nsis.installMode: "perUser"`. Portable .zip: custom CI step (Tauri has no native portable target) — see Architecture Patterns §Portable .zip |
| DIST-02 | Auto-updater wired to GitHub Releases `latest.json` via `tauri-plugin-updater`; user prompted to install | `tauri-plugin-updater = "2.10"` + endpoint `https://github.com/<user>/<repo>/releases/latest/download/latest.json` + Ed25519 pubkey. API: `app.updater()?.check().await?` then `update.download_and_install(...)` + `app.restart()` — see Code Examples §Updater wiring |
| DIST-03 | GitHub Actions release workflow (`tauri-action`) builds and attaches installer + portable artifacts on tag push | `tauri-apps/tauri-action@v0`. Signing via `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` env vars. `uploadUpdaterJson: true` (default) generates and uploads `latest.json`. Portable .zip: post-tauri-action `pwsh` step + `gh release upload` |
| DIST-04 | First-run path-discovery wizard scans + surfaces detected sources | Reuses `paths::discover()` (already returns full DiscoveredPaths). Wizard window: `WebviewWindowBuilder` mirroring popup/companion patterns. Trigger: SQLite `settings.first_run_done` row in existing `settings` table (no new migration needed) |
</phase_requirements>

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Tray icon and menu
- D-01 Tray menu structure (locked): `Show companion / Fire test popup / Settings… / ☑ Start with Windows / Quit` with inline checkable autostart toggle.
- D-02 Tray icon presence: Always-on-top tray icon with Hallmark monochrome glyph. Right-click menu, left-click = "Show companion".
- D-03 Quit semantics: clean shutdown — drains popup queue, joins tasks. Companion X = hide (Phase 2 behavior). Quit is tray-only.

#### Test popup trigger (POL-01)
- D-04 Inject point: synthesize `RawUnlockEvent` at the **adapter→dedup boundary** (the same `mpsc::Sender<RawUnlockEvent>` adapters feed). Hits real dedup, schema, audio, popup-queue, monitor placement.
- D-05 Test fixture data: hardcoded sample `app_id` + `ach_api_name = "HALLMARK_TEST_UNLOCK"` + bundled placeholder icon + canned title/desc. SchemaCache short-circuits to bundled fixture for this api_name.
- D-06 One-shot: each click = exactly one popup (subject to dedup TTL — second click within TTL is correctly suppressed).

#### Start-with-Windows (POL-02)
- D-07 Registry pattern: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark = "<install-path>\hallmark.exe" --silent`. Toggle off = removes named value. Per-user only — **never HKLM**.
- D-08 `--silent` startup: tray-only mode; companion does NOT auto-open. File watcher + game-detection + popup-queue all initialize. Matches Discord/Slack/Steam.
- D-09 Tray menu item state sync: "☑ Start with Windows" reflects live registry state (read on tray menu open). No SQLite shadow flag — registry is source of truth.

#### Settings window
- D-10 Window form factor: borderless rounded card, `decorations: false`, fixed 520×580, centered on tray-icon's monitor.
- D-11 Settings panels: (1) Detected sources read-only list (`Steam`, `Goldberg`, `CreamAPI`, `SmartSteamEmu`) + Rescan button — NO emulator setup help. (2) Updates: current version + Check button + result inline. (3) About: version, GitHub link, license, credits.
- D-12 NOT in v1 settings: autostart toggle (tray-only), log viewer (deferred), theme/sound knobs (out of scope), channel selector (stable only), companion size/position reset, auto-update on/off.

#### First-run wizard (DIST-04)
- D-13 Surface: standalone borderless rounded-card window separate from companion.
- D-14 Trigger lifecycle: SQLite `settings` row `first_run_done`. Wizard fires when unset. Set on dismissal **only if ≥1 path detected**. If 0 paths, flag stays unset → wizard re-fires next launch.
- D-15 N>0 paths: "Welcome to Hallmark" + "We found these achievement sources on your system:" + explicit list + "Get started" CTA.
- D-16 N=0 paths: header acknowledges no sources, body lists what was scanned per-source, one-liner explainer, "Rescan" + "Continue anyway" buttons. NO emulator setup instructions.
- D-17 Settings → Detected sources panel offers same Rescan capability. Wizard is first-launch-only.

#### Auto-updater (DIST-02)
- D-18 Update prompt UX: on launch, background-check `latest.json`. If newer, modal sheet over companion the next time companion opens (NOT during gameplay — companion is hidden). Modal: release notes (truncated, "Read more on GitHub" link) + "Install" / "Later" buttons. "Later" = session-snooze.
- D-19 Channel: stable only for v1. tauri-plugin-updater reads single `latest.json`. Prereleases NOT picked up.
- D-20 Install flow: "Install" → tauri-plugin-updater downloads → `app.restart()`. Hallmark process only — never the OS.
- D-21 Updater signing keypair: Ed25519 generated via `tauri signer generate` ONCE. Private key pasted directly into `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Local copy DELETED after upload. Public key committed in `tauri.conf.json`. CI is the only signer.

#### NSIS installer + portable .zip (DIST-01)
- D-22 Installer scope: per-user. NSIS `installMode: "perUser"`. Default install dir `%LOCALAPPDATA%\Hallmark`. No UAC. Updater can write to install dir without admin.
- D-23 Portable .zip behavior: self-contained, extracts anywhere, runs from any path. State at `%APPDATA%\com.hallmark.app\` — same as installed mode. Updater **disabled** in portable mode. Detection: at startup, compare exe path to known install location.
- D-24 Code signing: unsigned for v1. SmartScreen warning documented in README. CI workflow includes commented signtool placeholders for future contributor.

#### GitHub Actions release pipeline (DIST-03)
- D-25 Trigger: tag push `v*.*.*` → tauri-action workflow. Builds NSIS + portable on `windows-latest`, signs `latest.json`, uploads to matching release.
- D-26 `workflow_dispatch` fallback for emergency reruns.
- D-27 Pre-release tags ignored by updater feed.

#### SFX final assets (RESEARCH FLAG D-28)
- D-28 NOT LOCKED — researcher recommends. Preference: (1) Procedural refinement of `gen_sfx.exe` parameters; (2) CC0 royalty-free pack curation (freesound.org CC0 filter). NEVER rip copyrighted sources (DMCA/legal hard NO).
- D-29 Asset format: WAV/OGG, 44.1kHz/16-bit, mono or stereo. Bundled at `assets/sfx/{standard,rare,celebration}.{ext}`. Replacement is drop-in; no `audio.rs` changes.

### Claude's Discretion
- Exact tray icon glyph design (within Hallmark monochrome theme).
- Whether "Updates" panel shows last-checked timestamp.
- First-run wizard exact copy (within established voice).
- About panel exact links + license SPDX.
- NSIS installer wizard pages — tauri-action defaults unless friction emerges.
- Portable-mode detection heuristic (exe-folder-writable vs. `--portable` flag).
- Whether "Fire test popup" short-circuits Steam Web API rarity vs. uses cached/zero rarity.

### Deferred Ideas (OUT OF SCOPE)
- Diagnostic log viewer in Settings (v1.1).
- Update-channel selector stable/prerelease toggle (v2 contingent on traction).
- Auto-update on/off toggle (v1.1).
- Code signing + paid cert (deferred indefinitely).
- Truly-portable mode (state-beside-exe) (deferred to power-user request).
- Companion size/position reset button (first user request).
- Telemetry / crash reporting (NOT COMING — explicit out of scope per PROJECT.md).
</user_constraints>

## Project Constraints (from CLAUDE.md)

The following CLAUDE.md directives flow into Phase 4 plans. The planner MUST verify these are honored:

- **GSD enforcement** — all file changes must go through a GSD command (this Phase 4 work is `/gsd-execute-phase`).
- **Stack pins:** Tauri 2.11.x, React 19/Vite 6, Rust 1.85+, notify 8.2, rodio 0.22, sysinfo 0.39, **tauri-plugin-updater 2.10.1**, windows-rs 0.58, winreg 0.56 (Phase 4 adds the plugin + winreg; everything else is reused). [VERIFIED: Cargo.toml inspection]
- **What NOT to use** (CLAUDE.md "What NOT to Use" table — Phase 4 must not regress): Electron, DLL-injection overlay, Steam Web API as primary detection, FMOD/irrKlang, WMI, ETW, velopack (still pre-release), `fs.watch`/chokidar, MSIX. [CITED: CLAUDE.md]
- **Inno Setup vs NSIS:** CLAUDE.md cites Inno Setup 6.x as "dominant choice for OSS GitHub-distributed utilities", but Tauri's first-class Windows installer target is **NSIS**, not Inno. CLAUDE.md's Inno-first wording is a minor mis-prioritization given the bundler reality; Phase 4 uses NSIS via `tauri-bundler`. [CITED: CLAUDE.md vs verified via tauri-docs]
- **Goldberg / emulator stance — passive only:** D-11 Settings panel must NOT include emulator setup help. D-15/D-16 wizard names emulators but does NOT instruct on setup.
- **Signature style locked:** D-12 forbids theme/sound knobs in Settings. D-29 SFX swap is a drop-in asset replacement, not a customization mechanism.
- **OSS redistribution boundary:** D-28 SFX cannot bundle copyrighted material. PROJECT.md "Free, open-source on GitHub" + DMCA risk = hard NO on PS5/Xbox SFX rips.
- **TTS / global instructions:** No impact on Phase 4 (used only when user says "speak"). [CITED: ~/.claude/CLAUDE.md]

## Standard Stack

### Core (additions for Phase 4 — all others reused from Phases 1-3)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri-plugin-updater | 2.10.1 | In-app auto-updater. Reads `latest.json`, verifies Ed25519 sig, downloads + installs + restarts. | Official Tauri plugin; integrates with bundler-generated `.sig` files; matches CONTEXT.md D-21 / CLAUDE.md pin. **[VERIFIED: crates.io API 2026-04-04]** |
| winreg | 0.56.0 | Direct HKCU\Run registry read/write for D-07 autostart toggle. | Idiomatic Rust binding for Win32 registry; already in `Cargo.toml` (winreg = "0.56" target-windows-only). **[VERIFIED: Cargo.toml line 49 + crates.io 2026-03-14]** |

### Already-present dependencies (no version bump needed for Phase 4)

| Library | Version | Phase 4 Use |
|---------|---------|-------------|
| tauri | 2.11 | Tray API (`tauri::tray::TrayIconBuilder`), Menu API (`tauri::menu::{MenuBuilder, MenuItem, CheckMenuItem, PredefinedMenuItem}`), `WebviewWindowBuilder` for Settings + wizard windows, `app.restart()` for updater. |
| windows | 0.58 | (Reused for HWND patches if needed; Phase 4 may not need new features.) |
| serde / serde_json | 1.0 | Tauri command payloads, latest.json parsing. |
| reqwest | 0.13 | Already present from Phase 2 schema fetcher; updater plugin has its own HTTP client (tauri-plugin-http) — no new dep. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tauri-plugin-process | 2.3.1 | Provides `relaunch()` JS-side (Rust uses `app.restart()` directly). | Add IF the React update modal calls relaunch from JS. With Rust-driven flow (D-20: backend orchestrates install + restart), this plugin is OPTIONAL — Rust's `app.restart()` is sufficient. **[VERIFIED: crates.io 2025-10-27]** |
| tauri-plugin-autostart | 2.5.1 | Cross-platform autostart helper. | **NOT recommended for Hallmark.** See Don't Hand-Roll table — plugin's underlying `auto-launch` crate tries HKLM first, which violates D-07. Use `winreg` directly. **[VERIFIED: crates.io 2025-10-27]** |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `winreg` direct write | `tauri-plugin-autostart` | Plugin is one less dep to add but does NOT guarantee HKCU (auto-launch crate fallback semantics). For D-07's "never HKLM" hard rule, direct `winreg` is the only way to be sure. |
| `tauri-plugin-updater` 2.10.1 | `velopack` | velopack has richer installer (delta updates, auto-shortcut), but **all releases are 0.0.x pre-release as of May 2026** (CLAUDE.md "What NOT to Use"). v2 candidate. |
| NSIS via Tauri bundler | Inno Setup 6 | Inno is "dominant choice" for OSS utilities (CLAUDE.md), but Tauri's first-class Windows target is NSIS — using Inno would require ditching `tauri-action`'s NSIS pipeline. NSIS is the lower-friction path. |
| Custom CI portable .zip step | `tauri-bundler` portable target | Tauri does NOT have a portable target (verified — only `nsis` and `msi`). Custom `pwsh` step in CI is the only path. |
| Static `latest.json` from GitHub Releases | Dynamic update server | A custom server allows release channels and analytics; for hobby-OSS scope (D-19) static is correct — zero infra cost, GitHub CDN. |
| `tauri-action@v0` + manual `gh release` for portable | Pure custom CI script | tauri-action handles the NSIS + sig + latest.json upload correctly; the portable .zip is the only piece needing custom logic. Hybrid (tauri-action + post-step) is minimum-friction. |

**Installation:**
```bash
# In src-tauri/Cargo.toml — add:
tauri-plugin-updater = "2.10"
# winreg already pinned at 0.56 (Phase 1) — no change
```
```json
// In package.json — add:
"@tauri-apps/plugin-updater": "^2"
```

**Version verification:** Verified against crates.io API on 2026-05-09:
- `tauri-plugin-updater` 2.10.1 (2026-04-04) — current latest **[VERIFIED: crates.io API]**
- `tauri-plugin-autostart` 2.5.1 (2025-10-27) — current latest **[VERIFIED: crates.io API]**
- `winreg` 0.56.0 (2026-03-14) — current latest, matches existing pin **[VERIFIED: crates.io API]**
- `tauri-plugin-process` 2.3.1 (2025-10-27) — current latest **[VERIFIED: crates.io API]**

## Architecture Patterns

### System Architecture Diagram

```
                  ┌─────────────────────────────────┐
                  │   Tag push v*.*.* / dispatch    │
                  └────────────┬────────────────────┘
                               ▼
                  ┌─────────────────────────────────┐
                  │  GitHub Actions: tauri-action   │
                  │  windows-latest                 │
                  │  TAURI_SIGNING_PRIVATE_KEY      │
                  │   ├─→ tauri build (NSIS)        │
                  │   │    ├─→ hallmark-setup.exe   │
                  │   │    └─→ hallmark-setup.exe.sig│
                  │   ├─→ uploadUpdaterJson         │
                  │   │    └─→ latest.json (signed) │
                  │   └─→ post-step: pwsh           │
                  │        └─→ hallmark-portable.zip│
                  └────────────┬────────────────────┘
                               ▼
                  ┌─────────────────────────────────┐
                  │   GitHub Release v*.*.*         │
                  │   Assets:                       │
                  │   - hallmark-setup.exe          │
                  │   - hallmark-setup.exe.sig      │
                  │   - hallmark-portable.zip       │
                  │   - latest.json                 │
                  └────────────┬────────────────────┘
                               │
              ┌────────────────┴────────────────┐
              ▼                                 ▼
    ┌──────────────────┐            ┌──────────────────────┐
    │  User downloads  │            │  Existing install:   │
    │  installer .exe  │            │  app.updater()       │
    │  → %LOCALAPPDATA%│            │  fetches latest.json │
    │     \Hallmark    │            │  verifies pubkey sig │
    │   (no UAC, D-22) │            │  downloads NSIS .sig │
    └────────┬─────────┘            │  app.restart()       │
             │                      └──────────┬───────────┘
             ▼                                 │
    ┌──────────────────────────────────────────▼───────────┐
    │              Hallmark process startup                 │
    │   lib.rs::run() — Phase 1-3 pipeline +                │
    │                   Phase 4 additions                   │
    │   ┌────────────────────────────────────────────────┐  │
    │   │ 1. Init tracing                                │  │
    │   │ 2. Open SqliteStore (existing)                 │  │
    │   │ 3. paths::discover() (existing — DiscoveredPaths)│  │
    │   │ 4. Create popup + companion windows (existing) │  │
    │   │ 5. NEW: Detect portable mode (exe-path heuristic)│ │
    │   │ 6. NEW: Read first_run_done from settings table│  │
    │   │    └─→ if unset OR (set AND 0 paths): show wizard│ │
    │   │ 7. Build adapters + spawn watcher/pipeline tasks│  │
    │   │ 8. Spawn popup_queue + game_detect (existing)  │  │
    │   │ 9. NEW: Build tray icon + menu                 │  │
    │   │    ├─→ Show companion → existing window.show()│  │
    │   │    ├─→ Fire test popup → raw_tx.send(synthetic)│  │
    │   │    ├─→ Settings… → create Settings window     │  │
    │   │    ├─→ ☑ Start with Windows → toggle HKCU\Run │  │
    │   │    └─→ Quit → app.exit(0) (clean shutdown)    │  │
    │   │10. NEW: Register tauri-plugin-updater         │  │
    │   │    └─→ if not portable mode:                  │  │
    │   │        bg-check latest.json on startup        │  │
    │   │        on companion-open: show modal if newer │  │
    │   └────────────────────────────────────────────────┘  │
    └───────────────────────────────────────────────────────┘
                               │
       ┌───────────────────────┴────────────────────────┐
       │                                                │
       ▼                                                ▼
  Existing:                                       NEW (Phase 4):
  - Watcher (4 adapters)                          - Settings window (React)
  - Popup queue                                   - First-run wizard window (React)
  - Companion window                              - Update modal (React, in companion)
  - SQLite store                                  - Tray icon + menu (Rust/native)
  - SchemaCache                                   - HKCU registry helper (winreg)
  - AudioDispatcher                               - Test-popup synthesizer
                                                  - Portable-mode detector
```

### Recommended Project Structure

```
src-tauri/
├── src/
│   ├── lib.rs              # Phase 4: extend run() setup() — add tray, updater, autostart, wizard logic
│   ├── tray.rs             # NEW — TrayIconBuilder + MenuBuilder + on_menu_event handler
│   ├── autostart.rs        # NEW — winreg HKCU\Run read/write helpers
│   ├── test_trigger.rs     # NEW — synthesize RawUnlockEvent + send via raw_tx clone
│   ├── first_run.rs        # NEW — first-run flag read/write, wizard window builder
│   ├── settings_window.rs  # NEW — Settings WebviewWindowBuilder
│   ├── portable_mode.rs    # NEW — exe-path heuristic + AppState flag
│   ├── updater_glue.rs     # NEW (optional) — background-check task, modal-trigger logic
│   ├── ui.rs               # EXISTING — companion + popup builders. Phase 4 may add settings/wizard helpers here OR put them in their own files (planner choice — see § Structure note below).
│   ├── store/
│   │   ├── mod.rs          # EXISTING. No new migration needed — settings table reused for first_run_done.
│   │   └── queries.rs      # EXISTING. Add: get_first_run_done, set_first_run_done, get_last_update_check (optional)
│   └── ...                 # Phase 1-3 modules unchanged
├── Cargo.toml              # Add tauri-plugin-updater = "2.10". winreg already present.
├── tauri.conf.json         # Flip bundle.active: true; add bundle.windows.nsis; add createUpdaterArtifacts; add plugins.updater
├── capabilities/
│   ├── companion.json      # EXISTING. Add: process:allow-restart, updater:default
│   ├── popup.json          # EXISTING. No change.
│   ├── settings.json       # NEW — capability for Settings window
│   └── wizard.json         # NEW — capability for first-run wizard window
├── icons/
│   └── tray.ico            # NEW — monochrome glyph for tray icon (16x16 + 32x32)
└── windows/
    └── (no .nsh hooks needed for v1)

src/                        # Frontend
├── main-companion.tsx      # EXISTING. Add: UpdateModal logic — listen for "update-available" event, render modal.
├── main-popup.tsx          # EXISTING. No change.
├── main-settings.tsx       # NEW — React entry point for settings window
├── main-wizard.tsx         # NEW — React entry point for first-run wizard
├── components/
│   ├── (existing components unchanged)
│   ├── UpdateModal.tsx     # NEW
│   ├── SettingsSourceRow.tsx  # NEW
│   └── WizardSourceRow.tsx    # NEW
├── styles/
│   ├── companion.css       # EXISTING
│   ├── popup.css           # EXISTING
│   ├── settings.css        # NEW
│   └── shared.css          # NEW (optional — extracted card-chrome rules per UI-SPEC § Component Inventory)
└── types.ts                # EXISTING. Extend: DiscoveredPathsView, UpdateInfo, FirstRunState

settings.html               # NEW — entry HTML for settings window (mirrors index.html / popup.html pattern)
wizard.html                 # NEW — entry HTML for wizard window

vite.config.ts              # EXISTING. Add 2 new entries: settings + wizard (rollupOptions.input)

.github/                    # NEW DIRECTORY (does not exist yet)
└── workflows/
    └── release.yml         # NEW — tauri-action workflow (see Code Examples)

assets/sfx/                 # EXISTING. Phase 4 D-29 — final asset swap.
```

**Structure note:** Whether new windows live in `ui.rs` or each in its own file is the planner's call. Recommendation: **own file per window** (`settings_window.rs`, `first_run.rs`) because each carries its own command handlers, state, and event listeners — `ui.rs` was already specialized to popup+companion HWND patches, and mixing settings/wizard there would dilute it.

### Pattern 1: Tray icon + menu with checkable autostart item

**What:** A persistent tray icon that exposes the 5 menu items from D-01, with the "Start with Windows" item being a `CheckMenuItem` that reflects live HKCU registry state on each menu open.

**When to use:** Anywhere a tray-resident desktop app needs an inline toggle that's the source of truth for an OS-level setting (autostart, theme, etc.).

**Example:**
```rust
// src-tauri/src/tray.rs — Phase 4 NEW
// Source: tauri-docs § learn/system-tray.mdx + start/migrate/from-tauri-1.mdx (Context7 verified)
use tauri::{
    image::Image,
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

use crate::autostart;

pub fn build_tray(app: &App) -> tauri::Result<()> {
    let app_handle = app.handle();

    // Header item (non-clickable, shows app name)
    let header = MenuItem::with_id(app_handle, "header", "Hallmark", false, None::<&str>)?;
    let show = MenuItem::with_id(app_handle, "show_companion", "Show companion", true, None::<&str>)?;
    let test = MenuItem::with_id(app_handle, "fire_test", "Fire test popup", true, None::<&str>)?;
    let settings = MenuItem::with_id(app_handle, "open_settings", "Settings…", true, None::<&str>)?;

    // Read live HKCU state for the checkable item
    let autostart_on = autostart::is_enabled().unwrap_or(false);
    let autostart = CheckMenuItemBuilder::new("Start with Windows")
        .id("toggle_autostart")
        .checked(autostart_on)
        .build(app_handle)?;

    let quit = MenuItem::with_id(app_handle, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app_handle)?;

    let menu = MenuBuilder::new(app_handle)
        .items(&[&header, &sep, &show, &test, &sep, &settings, &autostart, &sep, &quit])
        .build()?;

    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray.ico"))?;

    let _tray = TrayIconBuilder::with_id("hallmark-tray")
        .tooltip("Hallmark")
        .icon(tray_icon)
        .menu(&menu)
        .menu_on_left_click(false)  // left-click → custom action below, not menu
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_companion" => { let _ = show_companion(app); }
            "fire_test" => { let _ = crate::test_trigger::fire(app); }
            "open_settings" => { let _ = crate::settings_window::open(app); }
            "toggle_autostart" => {
                let now_on = autostart::is_enabled().unwrap_or(false);
                if now_on {
                    let _ = autostart::disable();
                } else {
                    let _ = autostart::enable();
                }
                // Note: the CheckMenuItem state will refresh next menu-open via the
                // re-read pattern; for instant feedback we'd need to rebuild the menu.
                // D-09 says read on tray-menu-open, so this is correct.
            }
            "quit" => { app.exit(0); }
            _ => (),
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click on icon → Show companion (D-02)
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                let _ = show_companion(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_companion(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("companion") {
        w.show()?;
        w.set_focus()?;
    }
    Ok(())
}
```

**Refresh-on-open caveat:** `CheckMenuItem.checked()` is set at build time. To reflect live registry state on each menu open, you have two options: (a) rebuild the menu in a `tray.set_menu(...)` call from a periodic task (overkill); (b) accept that the check state lags one menu-open after a toggle (acceptable per D-09 wording — "reflects live registry state ... read on tray menu open"). Option (b) is correct: rebuild the menu inside the toggle handler so the next open shows the updated state. **Confirm in plan.**

### Pattern 2: HKCU\Run autostart via winreg (D-07)

**What:** Direct registry write to `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark`. No HKLM, no plugin abstraction.

**When to use:** When you need a per-user autostart that is GUARANTEED never to attempt HKLM (D-07's hard rule).

**Example:**
```rust
// src-tauri/src/autostart.rs — Phase 4 NEW
// Source: winreg crate docs (docs.rs/winreg/0.56) — VERIFIED against existing
// usage in src-tauri (winreg = "0.56" target-windows-only, see Cargo.toml line 49).
#[cfg(target_os = "windows")]
use winreg::{enums::*, RegKey};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Hallmark";

#[cfg(target_os = "windows")]
pub fn is_enabled() -> anyhow::Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
        Ok(key) => match key.get_value::<String, _>(VALUE_NAME) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

#[cfg(target_os = "windows")]
pub fn enable() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let value = format!(r#""{}" --silent"#, exe.display());
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(RUN_KEY)?;
    key.set_value(VALUE_NAME, &value)?;
    tracing::info!(value = %value, "autostart enabled (HKCU\\...\\Run)");
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn disable() -> anyhow::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
        match key.delete_value(VALUE_NAME) {
            Ok(()) => tracing::info!("autostart disabled"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!("autostart already disabled");
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> anyhow::Result<bool> { Ok(false) }
#[cfg(not(target_os = "windows"))]
pub fn enable() -> anyhow::Result<()> { Ok(()) }
#[cfg(not(target_os = "windows"))]
pub fn disable() -> anyhow::Result<()> { Ok(()) }
```

**Why double-quote the path:** Windows installations may live at paths containing spaces (e.g. `C:\Users\First Last\AppData\Local\Hallmark\hallmark.exe`). Quoting the executable token prevents the shell from splitting the value on spaces. The `--silent` flag lives outside the quotes (D-08).

### Pattern 3: Test-popup injection at the adapter→dedup boundary (D-04)

**What:** Synthesize a `RawUnlockEvent` and push it into the existing `raw_tx: mpsc::Sender<RawUnlockEvent>` that all source adapters feed. The dedup, store, popup_queue, and audio dispatcher all see this event the same way they see real Goldberg unlocks.

**When to use:** Anywhere a self-test must traverse the production pipeline without involving a real game / file event.

**Example:**
```rust
// src-tauri/src/test_trigger.rs — Phase 4 NEW
// Source: existing src-tauri/src/sources/mod.rs::RawUnlockEvent — VERIFIED.
// Inject seam: src-tauri/src/lib.rs::run() line 223 creates `raw_tx`; clone
// stored in AppState alongside session_id.

use tauri::{AppHandle, Manager};
use crate::sources::{RawUnlockEvent, SourceKind};

pub const TEST_API_NAME: &str = "HALLMARK_TEST_UNLOCK";
pub const TEST_APP_ID: u64 = 480;  // Spacewar — official Steam test app

pub fn fire(app: &AppHandle) -> anyhow::Result<()> {
    // AppState (Phase 4 extension) holds a clone of raw_tx for this purpose.
    let state = app.state::<crate::AppState>();
    let raw_tx = state.raw_tx.clone();  // Phase 4 adds this field — see lib.rs § integration

    let evt = RawUnlockEvent {
        app_id: TEST_APP_ID,
        ach_api_name: TEST_API_NAME.into(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
        source: SourceKind::Goldberg,  // any source — dedup ignores source for matching
    };

    // Use blocking_send because tray-event handlers are sync.
    if let Err(e) = raw_tx.blocking_send(evt) {
        tracing::warn!(error = %e, "test-popup send failed (channel closed?)");
        anyhow::bail!("test channel closed");
    }
    tracing::info!("test popup fired (synthetic event injected at adapter→dedup boundary)");
    Ok(())
}
```

**Schema short-circuit (D-05):** `SchemaCache::lookup(480, "HALLMARK_TEST_UNLOCK")` will return `None` from the existing cache logic. Two paths:

1. **Pre-seed the cache at startup** — insert a synthetic schema row into `schema_cache` for `(480, "HALLMARK_TEST_UNLOCK")` with bundled icon path and canned title/desc. Idempotent SQLite write. Fastest, requires no schema-cache code changes.
2. **Add a fixture branch in `SchemaCache::lookup`** — special-case the test api_name. More invasive; requires changing `schema/mod.rs`.

Recommendation: **Option 1 (pre-seed)**. Phase 1-3 contracts stay locked; Phase 4 only adds a row.

### Pattern 4: tauri-plugin-updater wiring (DIST-02)

**What:** Register the updater plugin, configure endpoint and pubkey, run a background check on startup, and trigger a modal in the companion the first time the companion opens after a newer version is detected.

**When to use:** Any Tauri 2 app distributing through GitHub Releases.

**Example:**
```rust
// src-tauri/src/lib.rs — Phase 4 EXTENSION inside run()
// Source: tauri-docs § plugin/updater.mdx — VERIFIED via Context7.

use tauri_plugin_updater::UpdaterExt;

// Inside tauri::Builder::default() chain:
.plugin(tauri_plugin_updater::Builder::new().build())

// Inside setup():
let app_handle_updater = app.handle().clone();
tauri::async_runtime::spawn(async move {
    // Skip in portable mode — D-23 requires updater disabled when self-contained
    if crate::portable_mode::is_portable() {
        tracing::info!("portable mode detected; updater check skipped");
        return;
    }
    match app_handle_updater.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                tracing::info!(version = %update.version, "update available");
                // Stash for later — modal fires on next companion-open (D-18)
                let state = app_handle_updater.state::<crate::AppState>();
                state.pending_update.lock().await.replace(update);
                // Emit event to companion so it can display the modal IF currently visible
                let _ = app_handle_updater.emit_to("companion", "update-available", ());
            }
            Ok(None) => tracing::info!("no update available"),
            Err(e) => tracing::warn!(error = %e, "update check failed"),
        },
        Err(e) => tracing::warn!(error = %e, "updater not available"),
    }
});
```
```json
// tauri.conf.json — Phase 4 ADDITIONS
{
  "bundle": {
    "active": true,                    // flip from false (Phase 1-3 default)
    "createUpdaterArtifacts": true,    // generates .sig files
    "targets": ["nsis"],               // explicit; portable .zip is custom CI step
    "windows": {
      "nsis": {
        "installMode": "perUser"        // D-22 — no UAC
      }
    }
  },
  "plugins": {
    "updater": {
      "pubkey": "<paste output of `tauri signer generate` public key here>",
      "endpoints": [
        "https://github.com/<owner>/<repo>/releases/latest/download/latest.json"
      ]
    }
  }
}
```

**Install + restart on user-confirm:**
```rust
// In a Tauri command invoked by the React modal "Install" button:
#[tauri::command]
async fn install_pending_update(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let update_opt = state.pending_update.lock().await.take();
    if let Some(update) = update_opt {
        update.download_and_install(
            |chunk_length, content_length| {
                tracing::debug!(chunk_length, content_length, "update downloading");
            },
            || tracing::info!("update download finished"),
        ).await.map_err(|e| e.to_string())?;
        tracing::info!("update installed; restarting Hallmark");
        app.restart();  // never returns
    }
    Ok(())
}
```

**Capability:** Add `"updater:default"` and (if React calls relaunch directly) `"process:allow-restart"` to the companion capability JSON.

### Pattern 5: Static `latest.json` from GitHub Releases

**What:** A signed JSON file at the predictable URL `https://github.com/<owner>/<repo>/releases/latest/download/latest.json` that the updater reads to decide if a newer version exists.

**Schema (verified via Tauri docs):**
```json
{
  "version": "1.2.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2026-05-09T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<contents of hallmark-setup.exe.sig>",
      "url": "https://github.com/<owner>/<repo>/releases/download/v1.2.0/hallmark-setup.exe"
    }
  }
}
```

**Generation:** `tauri-action` with `uploadUpdaterJson: true` (default) generates this automatically by taking the produced bundle's `.sig` content and the release URL. **No manual step required** — the GH Actions workflow just needs `TAURI_SIGNING_PRIVATE_KEY` in env and the action does the rest.

**The `releases/latest/download/<asset-name>` URL pattern is GitHub's, not Tauri's** — GitHub serves the asset from the most recent non-prerelease release at that path. This is exactly D-19 + D-27's behavior: prerelease tags are skipped automatically.

### Pattern 6: First-run wizard trigger via SQLite settings flag (D-14)

**What:** A boolean flag in the existing `settings` table (no new migration) that decides whether to show the wizard.

**Example:**
```rust
// src-tauri/src/store/queries.rs — Phase 4 EXTENSION
// Source: existing src-tauri/src/store/migrations/001_initial.sql settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)

pub fn get_first_run_done(conn: &Connection) -> anyhow::Result<bool> {
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = 'first_run_done'",
        [],
        |r| r.get::<_, String>(0),
    );
    match result {
        Ok(v) => Ok(v == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub fn set_first_run_done(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('first_run_done', '1')",
        [],
    )?;
    Ok(())
}
```
```rust
// src-tauri/src/lib.rs::run() setup() — Phase 4 EXTENSION (after path discovery)
let first_run_done = store.with_conn(|c| crate::store::queries::get_first_run_done(c))?;
let any_path_detected = !discovery.steam_libraries.is_empty()
    || !discovery.goldberg_save_roots.is_empty()
    || !discovery.cream_api_appid_dirs.is_empty()
    || !discovery.sse_appid_dirs.is_empty()
    || discovery.steam_legit_appcache_stats.is_some();

if !first_run_done {
    crate::first_run::open_wizard(app_handle.clone(), any_path_detected)?;
} else if !any_path_detected {
    // D-14: re-fire wizard on subsequent launches if 0 paths still detected
    crate::first_run::open_wizard(app_handle.clone(), false)?;
}
```

**On dismissal:**
```rust
// In Tauri command "wizard_dismiss":
#[tauri::command]
async fn wizard_dismiss(
    app: AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    // Caller passes its own decision via a separate `wizard_paths_found` payload field
    // OR re-derive here:
    let any_path = !state.cached_discovery.steam_libraries.is_empty() /* ... */;
    if any_path {
        state.store.with_conn(|c| crate::store::queries::set_first_run_done(c))
            .map_err(|e| e.to_string())?;
    }
    if let Some(w) = app.get_webview_window("wizard") { w.close().ok(); }
    Ok(())
}
```

### Pattern 7: Portable-mode detection via exe-path heuristic (D-23)

**What:** At startup, compare the running executable's parent directory against the expected installed location (`%LOCALAPPDATA%\Hallmark`). If different, assume portable.

**Example:**
```rust
// src-tauri/src/portable_mode.rs — Phase 4 NEW
pub fn is_portable() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,  // failed to read; default to non-portable (safest)
    };
    let exe_parent = match exe.parent() {
        Some(p) => p,
        None => return false,
    };
    let installed = match dirs::data_local_dir() {
        Some(p) => p.join("Hallmark"),
        None => return false,
    };
    // Canonicalize both to handle case/short-path differences on Windows.
    let canon_exe = exe_parent.canonicalize().ok();
    let canon_inst = installed.canonicalize().ok();
    match (canon_exe, canon_inst) {
        (Some(a), Some(b)) => a != b,  // different parent ⇒ portable
        _ => false,  // either path doesn't exist; assume installed
    }
}
```

**Alternative considered:** `--portable` CLI flag. Rejected by claude's-discretion in favor of the heuristic — users running from a USB stick or extracted .zip don't pass flags.

### Pattern 8: Vite multi-entry config for two new windows

**What:** Add `settings.html` + `wizard.html` as Vite rollup inputs so each gets its own bundle, mirroring the existing companion + popup pattern.

**Example:**
```typescript
// vite.config.ts — Phase 4 EXTENSION (existing structure preserved)
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        companion: resolve(__dirname, "index.html"),
        popup: resolve(__dirname, "popup.html"),
        settings: resolve(__dirname, "settings.html"),  // NEW
        wizard: resolve(__dirname, "wizard.html"),      // NEW
      },
    },
  },
});
```

### Anti-Patterns to Avoid

- **Using `tauri-plugin-autostart` while D-07 says "never HKLM":** the plugin's auto-launch crate tries HKLM first. Even though it'd fall back to HKCU on a per-user install (no admin), the hard rule says the registry hive is part of the contract. Direct `winreg` is the only way to know for sure.
- **Synthesizing the test event into `popup_queue` directly:** bypasses dedup and SchemaCache short-circuit logic, defeating the "fires through full pipeline" success criterion (ROADMAP SC#1).
- **Putting `first_run_done` in a new SQLite table:** the existing `settings(key, value)` table is already used for `completion_<app_id>` flags (Phase 2). Add a row, not a table.
- **Calling `app.restart()` while a popup is mid-animation:** drain the popup queue with a timeout first (matches D-03 Quit semantics — apply the same drain logic to the updater install path).
- **Code-signing the executable with a self-signed cert:** SmartScreen does NOT trust self-signed certs and will issue the same "Windows protected your PC" warning as unsigned. The "self-signed warm-up" path requires a real EV cert. Stay unsigned for v1 (D-24).
- **Bundling SFX from PS5/Xbox/copyrighted games:** DMCA + contributor legal risk. CC0 royalty-free or procedural only.
- **Setting `autostart` registry value to a path WITHOUT quotes:** spaces in the install path (e.g., user folder names) split the value at the shell.
- **Creating Settings/wizard windows in `app.windows[]` declaratively in tauri.conf.json:** Phase 2 created popup + companion programmatically so HWND patches apply post-build. Stay consistent — programmatic creation in `setup()`.
- **Hosting `latest.json` anywhere other than the GitHub Release assets:** GitHub's `releases/latest/download/<file>` redirect is the only zero-infra-cost CDN that handles prerelease filtering correctly per D-27.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tray icon on Windows | Custom Win32 `Shell_NotifyIconW` | `tauri::tray::TrayIconBuilder` | Tauri 2 wraps tray-icon crate cross-platform; you'd reinvent menu event routing, light/dark icon support, and HiDPI handling |
| Update protocol + signature verification | Custom HTTP + Ed25519 verifier | `tauri-plugin-updater 2.10.1` | Plugin already implements minisign-style signature verify, Windows installer-replace dance, restart sequencing |
| Cross-platform autostart abstraction | Hand-rolled per-OS code | (Hallmark is Windows-only; just use `winreg`) | NOT applicable — but if portability ever matters, `tauri-plugin-autostart` is the right shape; just override its config to force per-user. Hallmark v1 is Windows-only per PROJECT.md. |
| GitHub Release upload + asset attachment | Custom `gh release` scripting | `tauri-apps/tauri-action@v0` | Action handles tagging, multi-platform matrix, NSIS + sig + latest.json upload. The portable .zip is the ONE custom step. |
| HKCU registry read/write | Manual `windows-rs` Registry API calls | `winreg 0.56` | winreg is idiomatic Rust, type-safe key/value handling, error mapping. Already a pinned dep. |
| WAV/OGG decoding for the test popup SFX | Custom decoder | rodio (existing dep) | Phase 2 dispatcher handles this; D-29 swap is asset-only. |
| First-run UX flag persistence | New SQLite table | Existing `settings` table row | Phase 1 schema already has `settings(key, value)`. Reusing it is one less migration. |
| Settings storage for Phase 4 prefs | New table per setting | `settings` table key/value | Same pattern — `last_update_check`, `wizard_seen_version`, etc., all live as rows. |
| Portable .zip emission | Custom .NET ZipFile / 7z scripting | PowerShell `Compress-Archive` in CI | One-liner; no extra dependency in CI. |
| `latest.json` generation | Manual JSON construction in CI | `tauri-action` `uploadUpdaterJson: true` | Default behavior; reads .sig files automatically. |

**Key insight:** Phase 4 is integration work, not novel engineering. Every problem in this phase has a stable Tauri/Rust solution as of 2026; the failure mode is composing them out-of-order or reinventing what already works. The two genuine custom bits are (1) the test-popup synthesizer (10 lines of Rust) and (2) the portable .zip CI step (10 lines of YAML).

## Runtime State Inventory

> NOT a rename/refactor phase. Section omitted per Step 2.5 trigger.

## Common Pitfalls

### Pitfall 1: `auto-launch` crate's HKLM-first default

**What goes wrong:** Plan adopts `tauri-plugin-autostart` for D-07. On a per-user install (no admin), the plugin's HKLM write fails and falls back to HKCU — happens to land in the right hive accidentally. On a future per-machine install (deferred but plausible), it would write HKLM and silently violate D-07.

**Why it happens:** Plugin documentation does not surface the registry-hive default; you have to read the auto-launch crate source to discover the fallback semantics.

**How to avoid:** Use `winreg` directly. Skip `tauri-plugin-autostart`.

**Warning signs:** Reviewing the plan and seeing "tauri-plugin-autostart" listed as a dep without an accompanying `WindowsEnableMode::CurrentUser` config (which the Tauri plugin doesn't even expose). [VERIFIED: tauri-apps/plugins-workspace/v2/plugins/autostart/src/lib.rs source inspection]

### Pitfall 2: CheckMenuItem state lag

**What goes wrong:** User toggles "Start with Windows", but the menu still shows the old check state on next open.

**Why it happens:** `CheckMenuItem.checked()` is set at build time. Rebuilding the menu inside the toggle handler fixes the next-open state but doesn't refresh the currently-open menu (which is fine — menu closed by then).

**How to avoid:** In the toggle handler, call `tray.set_menu(rebuild_menu(...))` after the registry write. Confirm in plan with explicit code.

**Warning signs:** Plan's tray code shows `on_menu_event` toggling state without ever calling `set_menu` to rebuild.

### Pitfall 3: Test popup tier mismatch

**What goes wrong:** Test popup fires but renders as `tier="standard"` even when the test fixture is supposed to demonstrate the "rare" or "completion" treatment.

**Why it happens:** `classify_tier` (existing in `schema/mod.rs`) decides tier from `global_pct`. The pre-seeded fixture row has `global_pct: NULL`, which routes to "standard".

**How to avoid:** D-04 + D-05 specify a single test popup at standard tier — that's correct. If the test should demonstrate rare-tier later (deferred), add a separate test fixture with `global_pct: 5.0` and a separate menu item. **Do not pre-emptively complicate v1.**

**Warning signs:** Plan adds a "Fire rare test popup" or "Fire completion test popup" item without explicit user request.

### Pitfall 4: `--silent` argv parsing in Rust without a CLI plugin

**What goes wrong:** `--silent` is set in HKCU\Run but Hallmark ignores it because no code looks at argv.

**Why it happens:** Tauri's `tauri::Builder` doesn't auto-parse argv. The CLI plugin is one option, but adding a plugin for one flag is overkill.

**How to avoid:** In `lib.rs::run()`, read `std::env::args().any(|a| a == "--silent")` once and stash on AppState. The companion-window auto-show logic checks this flag.

**Warning signs:** Plan adds `tauri-plugin-cli` for this. Reject — `std::env::args` is 1 line.

### Pitfall 5: Quit not draining popup queue

**What goes wrong:** User clicks Quit while a popup is still animating. `app.exit(0)` terminates immediately, the rodio sink drops mid-playback, and the popup webview disappears mid-animation — visually "glitchy" exit.

**Why it happens:** `app.exit(0)` is hard. D-03 says "drain popup queue with timeout, joins tokio tasks".

**How to avoid:** Wire Quit to a custom shutdown sequence: emit a "shutdown" event the popup_queue task listens for, await its completion with a 1-2 second timeout, then `app.exit(0)`. This pattern was deferred in Phase 2 (no Quit existed yet); Phase 4 implements it.

**Warning signs:** Plan's tray Quit handler is just `app.exit(0)` with no preamble.

### Pitfall 6: `latest.json` URL points at wrong release

**What goes wrong:** Updater fetches `latest.json` but the URL inside `latest.json.platforms.windows-x86_64.url` points at the wrong tag (e.g., previous release).

**Why it happens:** `tauri-action` substitutes `__VERSION__` correctly when configured, but typos in the workflow (`tagName: "v__VERSION__"` vs `"app-v__VERSION__"`) can desync `latest.json` from the actual asset URL.

**How to avoid:** Use the documented `tauri-action` defaults (no `tagName` substitution magic). Verify the first release manually before automating future tags. **Test from a stale install: install v1.0.0, push v1.0.1 tag, confirm updater finds and installs.**

**Warning signs:** Workflow YAML has hand-crafted `releaseBody` URL templates referencing version variables.

### Pitfall 7: Portable .zip includes the installer-only bundle

**What goes wrong:** Portable .zip CI step zips `target/release/bundle/nsis/hallmark-setup.exe` instead of `target/release/hallmark.exe`.

**Why it happens:** Confusing two outputs of `tauri build`: the raw executable (in `target/release/`) and the NSIS installer (in `target/release/bundle/nsis/`).

**How to avoid:** Portable .zip should contain `target/release/hallmark.exe` + the `assets/` directory if any runtime assets aren't bundled into the EXE (for Hallmark, SFX is `include_bytes!`'d so no separate asset dir needed; only the EXE).

**Warning signs:** CI step references `bundle/nsis/` in the portable .zip path.

### Pitfall 8: Updater bypasses portable mode

**What goes wrong:** Portable user gets an update prompt; clicks Install; updater downloads NSIS installer and tries to overwrite the portable .exe in place. Explosion.

**Why it happens:** Updater plugin has no concept of portable mode; it just runs.

**How to avoid:** Gate the entire updater registration behind `if !portable_mode::is_portable()` — D-23 says "Updater is disabled in portable mode".

**Warning signs:** Plan registers updater unconditionally; portable detection lives in a different module entirely.

### Pitfall 9: SFX swap breaks rodio's `include_bytes!`

**What goes wrong:** Designer drops new `popup-standard.wav` with a different format (e.g., float WAV vs. PCM 16-bit) and `audio.rs::AudioDispatcher::new()` panics at decode validation.

**Why it happens:** `audio.rs` line 60-69 (existing) calls `Decoder::try_from(...)` on each bundled WAV at startup; format mismatch errors there.

**How to avoid:** D-29 already specifies the format constraint (WAV PCM 16-bit, 44.1/48kHz). If the planner adopts CC0 packs, the curation step must convert to the spec format BEFORE committing.

**Warning signs:** Plan accepts arbitrary WAV files without format-validation in CI.

### Pitfall 10: GitHub Releases prerelease filtering is per-API, not per-URL

**What goes wrong:** Updater's endpoint URL points at `releases/latest/download/latest.json`. A maintainer marks a release as "Latest" in the GitHub UI for a prerelease — now the prerelease's `latest.json` is served as the stable feed.

**Why it happens:** GitHub's `latest` URL serves whatever is marked "Latest" in the UI, which is independent of the prerelease flag.

**How to avoid:** D-27 says "Releases marked `prerelease: true` on GitHub do NOT have their `latest.json` published as the stable feed". To enforce: never mark a prerelease as "Latest" in the UI. Document this in CONTRIBUTING.md or release-process docs. [CITED: GitHub Releases UI behavior — maintainer discipline required, not a Tauri/CI guarantee]

## Code Examples

Verified patterns from official sources collected above. Cross-references:
- Tray icon + menu: see Pattern 1 (Code Examples)
- HKCU autostart via winreg: see Pattern 2
- Test-popup injection: see Pattern 3
- Updater wiring: see Pattern 4
- `latest.json` schema: see Pattern 5
- First-run flag persistence: see Pattern 6
- Portable-mode detection: see Pattern 7
- Vite multi-entry config: see Pattern 8

### GitHub Actions release workflow (DIST-03 — full file)

**Source:** Tauri docs § distribute/Pipelines/github.mdx — VERIFIED via Context7. Adapted for Hallmark Windows-only build + custom portable-zip post-step.

```yaml
# .github/workflows/release.yml — Phase 4 NEW
name: release

on:
  push:
    tags:
      - 'v*.*.*'           # D-25 trigger pattern
  workflow_dispatch:        # D-26 emergency rerun

jobs:
  publish:
    permissions:
      contents: write
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 9

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: 'pnpm'

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Install frontend dependencies
        run: pnpm install --frozen-lockfile

      - name: Build + create release (NSIS + latest.json)
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          # D-21: Ed25519 keypair lives ONLY as a GitHub secret.
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          # === COMMENTED-OUT placeholders for future code-signing (D-24) ===
          # WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
          # WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Hallmark ${{ github.ref_name }}'
          releaseBody: 'See assets to download. Auto-update is wired in Hallmark v0.1+.'
          releaseDraft: false
          prerelease: false
          # uploadUpdaterJson defaults to true → latest.json + sig auto-uploaded.

      - name: Build portable .zip (custom — Tauri has no portable target)
        shell: pwsh
        run: |
          $version = $env:GITHUB_REF_NAME -replace '^v', ''
          $exe = "src-tauri/target/release/hallmark.exe"
          $stage = "portable-stage"
          New-Item -ItemType Directory -Force -Path $stage | Out-Null
          Copy-Item $exe $stage/hallmark.exe
          # If runtime assets exist OUTSIDE the EXE, copy them here. Hallmark
          # bundles SFX via include_bytes! so no extra files needed.
          $zipName = "hallmark-portable-$version.zip"
          Compress-Archive -Path "$stage/*" -DestinationPath $zipName
          echo "PORTABLE_ZIP=$zipName" >> $env:GITHUB_ENV

      - name: Upload portable .zip to release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        shell: pwsh
        run: |
          gh release upload ${{ github.ref_name }} ${{ env.PORTABLE_ZIP }} --clobber
```

**Why `gh release upload` separately:** `tauri-action` only uploads artifacts it generated. The portable .zip is post-action, so we use `gh` CLI (preinstalled on `windows-latest` runners) to attach it to the release the action just created.

### Tauri capability for Settings window

```json
// src-tauri/capabilities/settings.json — Phase 4 NEW
// Source: Phase 2 capabilities/companion.json structure — VERIFIED.
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "settings-capability",
  "description": "Settings window — read-only paths panel, update check, about. Custom drag region.",
  "windows": ["settings"],
  "permissions": [
    "core:default",
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-start-dragging",
    "updater:default"
  ]
}
```

### Tauri capability for first-run wizard window

```json
// src-tauri/capabilities/wizard.json — Phase 4 NEW
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "wizard-capability",
  "description": "First-run welcome wizard. Shows path-discovery results.",
  "windows": ["wizard"],
  "permissions": [
    "core:default",
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-show",
    "core:window:allow-close",
    "core:window:allow-start-dragging"
  ]
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri 1.x `SystemTrayMenu` | Tauri 2 `tauri::tray::TrayIconBuilder` + `tauri::menu::MenuBuilder` | Tauri 2.0 (Oct 2024) | Module restructure; v1 examples in stale blogs are misleading. Always check Tauri 2 docs. [CITED: tauri-docs migration guide] |
| Tauri 1.x `app.windows` declarative-only | Programmatic `WebviewWindowBuilder` in `setup()` | Tauri 2.0 | Programmatic creation lets HWND patches apply post-build (Phase 2 popup window already does this) |
| velopack as updater | `tauri-plugin-updater 2.10.1` | velopack remained pre-release; updater plugin stabilized 2024 | Hobby OSS scope wins from stable plugin; defer velopack. [CITED: CLAUDE.md What NOT to Use] |
| Manual `latest.json` upload step | `tauri-action` `uploadUpdaterJson: true` default | tauri-action v0.5+ | One less custom CI step. |
| Tauri 1.x updater config under `tauri.updater` | Tauri 2 config under `plugins.updater` + `bundle.createUpdaterArtifacts` | Tauri 2.0 | Migration trap — old tauri.conf.json keys are silently ignored |
| Manual NSIS template fork for per-user install | `bundle.windows.nsis.installMode: "perUser"` | Tauri 1.5+ | One config flag replaces a custom .nsi |
| `tauri-plugin-process` `relaunch()` from JS | `app.restart()` from Rust | Tauri 2.0 | Both work; Rust call avoids needing an extra plugin if backend orchestrates the install |

**Deprecated/outdated:**
- `tauri.updater.endpoints` (Tauri 1) — replaced by `plugins.updater.endpoints` (Tauri 2).
- `tauri.bundle.windows.useBootstrapper` — removed in Tauri 2.
- `tauri-plugin-window-state` Phase 2 considered — Phase 4 doesn't need it (Settings + wizard are fixed-size; companion already has its own prefs persistence).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `tauri-action` `uploadUpdaterJson: true` default (May 2026 behavior) generates a single platform key (`windows-x86_64`) when the workflow only builds Windows. | Pattern 5, GH Actions example | Low — if action emits unexpected platform keys, the updater silently ignores non-matching entries. Verify on first release. **[ASSUMED]** |
| A2 | Setting `bundle.targets: ["nsis"]` in tauri.conf.json prevents msi-only artifacts from being generated and uploaded. | Architecture Patterns, tauri.conf.json | Low — extra .msi in the release is a minor cosmetic issue. **[ASSUMED]** |
| A3 | The `tauri.updater.endpoints` static URL `https://github.com/<owner>/<repo>/releases/latest/download/latest.json` resolves correctly even when the previous release was a prerelease (i.e., GitHub serves only non-prerelease "latest"). | Pattern 5 | Medium — Pitfall 10 documents the maintainer-discipline workaround. **[CITED: GitHub Releases standard behavior]** |
| A4 | rodio 0.22 will decode user-replaced WAV files at startup and surface format mismatches as `Err` (no panic). | Pitfall 9 | Low — confirmed by inspection of existing `audio.rs::new()` line 60-69 — uses `Decoder::try_from` + `anyhow::bail!`. **[VERIFIED: src-tauri/src/audio.rs]** |
| A5 | Pre-seeding `schema_cache` for the test fixture row at every startup is idempotent (`INSERT OR REPLACE`) and does not race with Phase 2's schema-resolve task for app_id 480. | Pattern 3 schema short-circuit | Low — Phase 2 schema resolver only runs on game-start (app_id 480 = Spacewar; user is unlikely to actually launch Spacewar). Even if it runs, cache writes are idempotent. **[VERIFIED: schema_cache PRIMARY KEY (app_id, ach_api_name) in 002 migration]** |
| A6 | Tauri 2.11.1's `WebviewWindowBuilder` supports the same constructor pattern Phase 2 uses for popup + companion (`decorations: false`, `inner_size`, `center`, etc.) for Settings + wizard. | Pattern 1, Project Structure | Low — Phase 2 already shipped this pattern. **[VERIFIED: src-tauri/src/ui.rs]** |
| A7 | The `--silent` flag is parsed correctly by `std::env::args()` even when launched via HKCU\Run with quoted exe path + unquoted arg. | Pitfall 4, Pattern 2 | Low — Windows shell preserves --silent as separate arg via standard quoting rules. **[ASSUMED]** Verify empirically with Win+R `"C:\path\hallmark.exe" --silent` once installer ships. |
| A8 | `auto-launch` crate's HKLM-first behavior in `tauri-plugin-autostart` is unchanged in 2.5.1 (Oct 2025). | Pitfall 1, Don't Hand-Roll table | Medium — if upstream fixes this, the plugin becomes acceptable. **[CITED: plugins-workspace/v2/plugins/autostart/src/lib.rs HEAD inspection 2026-05-09]** |
| A9 | The "Hallmark" registry value name in HKCU\Run survives a sysprep / user-profile-restore round-trip without corruption. | Pattern 2 | Low — Windows registry values are just text; no known sysprep gotchas. **[ASSUMED]** |
| A10 | freesound.org's CC0 sounds, when downloaded today, are bundleable into Hallmark's GitHub repo without per-asset attribution beyond CC0 standard practice. | D-28 SFX path 2 | Medium — verify each downloaded asset's per-page license string at the time of download (some uploaders mark CC0 in metadata but include "please credit" in description — D-28 says license terms must permit OSS-redistribution; standard CC0 does, descriptive notes do not override the license). **[ASSUMED]** Recommendation: only use sounds whose license page literally says "Creative Commons 0" with no extra clauses. **[CITED: freesound.org/help/faq + audiocommons.github.io/2019/01/04/cc-licenses]** |

**Items needing user confirmation before plan execution:** A8 (autostart-crate behavior unchanged) is the most likely to flip if a maintainer decides to fix the upstream issue between research and execution. Re-check at plan-creation time. A10 (CC0 attribution discipline) requires the planner to define a verification step in the SFX-curation task IF the SFX path goes royalty-free instead of procedural.

## Open Questions

1. **SFX direction (D-28 RESEARCH FLAG):**
   - **What we know:** Procedural option keeps zero licensing risk. CC0 packs from freesound.org are OSS-compatible if the literal license page says "Creative Commons 0" (not "CC-BY"). Procedural state-of-the-art for "premium UI ding" is achievable with `gen_sfx`-style additive synthesis (sine + saw + envelope + reverb tail) — comparable to procedural sound libraries used in mobile-game SFX.
   - **What's unclear:** Whether procedural can hit "PS5-grade" without a sound designer's ear. Realistic answer: probably no, but it'll be 80% of the way there.
   - **Recommendation:** Phase 4 ships option 1 (procedural refinement of `gen_sfx` parameters) for v1. Tune sub-bass riser, attack envelope, and reverb tail. If user listens to the result and is unsatisfied, fall back to CC0 curation in a v1.1 follow-up. **DO NOT block Phase 4 release on subjective audio quality** — the locked detection + popup pipeline is the hero feature.

2. **Tray icon glyph design (Claude's discretion):**
   - **What we know:** Monochrome white-on-dark / black-on-light glyph at 16×16 + 32×32. Tauri `Image::from_bytes(include_bytes!(...))` accepts ICO files.
   - **What's unclear:** Specific iconography. Trophy? Medal? Seal? Custom monogram?
   - **Recommendation:** Simple seal/medal outline glyph. Two assets at `icons/tray-light.ico` + `icons/tray-dark.ico` if Tauri 2 supports light/dark tray variants — verify in plan. If not, single neutral grey icon.

3. **Settings "Updates" panel last-checked timestamp (Claude's discretion):**
   - **What we know:** `last_update_check` could be a `settings` row written after each background check. UI-SPEC's copywriting table includes "Last checked: {relative time or 'just now'}".
   - **Recommendation:** Add the row, render the timestamp. Minor UX polish; one extra `set_last_update_check` call.

4. **Whether the `Fire test popup` rarity behavior should round-trip to live Web API (Claude's discretion):**
   - **What we know:** D-05 says "Schema lookup short-circuits to the bundled fixture for this api_name (does NOT round-trip Web API)". So no API call.
   - **Recommendation:** **No round-trip.** The test popup demonstrates that the local pipeline works; Web API is a separate concern (its own outage doesn't block the test).

5. **NSIS branding image (Claude's discretion):**
   - **What we know:** `bundle.windows.nsis.installerIcon` + `headerImage` + `sidebarImage` configurable. tauri-action defaults are unbranded.
   - **Recommendation:** Add a single 150×57 sidebar BMP if a Hallmark logo asset is available; otherwise skip and keep the default look. v1 polish, not a blocker.

## Environment Availability

> Phase 4 introduces external CI dependencies but no new local-machine ones. The local dev loop is unchanged from Phases 1-3.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.85+ | All Rust code | ✓ | 1.85+ | — |
| pnpm 9 | Frontend build | ✓ | (existing — `pnpm-lock.yaml` present) | — |
| `tauri-cli` | `cargo tauri build` | ✓ | (Phase 1 setup) | — |
| Tauri signer keypair | DIST-02 signing | ✗ (must be generated) | — | **REQUIRED** — generate via `tauri signer generate -w ~/.tauri/hallmark.key`, paste into GH secret, delete local copy |
| GitHub repo with Releases enabled | DIST-03 publication | (assumed yes — repo exists) | — | — |
| GitHub Actions enabled | DIST-03 workflow | (assumed yes — public OSS repo) | — | — |
| `gh` CLI on `windows-latest` runner | Portable .zip upload step | ✓ (preinstalled on GitHub-hosted runners) | — | — |
| `pwsh` on `windows-latest` runner | Portable .zip Compress-Archive step | ✓ (Windows Server 2022 default) | — | — |
| GH Secrets: `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | DIST-02 signing in CI | ✗ (must be set manually) | — | **REQUIRED** — set as repo secret before first tag push. Workflow will fail without it. |
| `assets/sfx/popup-{standard,rare,100pct}.wav` (final, not placeholder) | D-29 final asset | ✗ (placeholders in repo) | — | Phase 4 task: produce final assets via D-28 chosen path |
| `icons/tray.ico` (or `icons/tray-light.ico` + `icons/tray-dark.ico`) | Tray icon glyph | ✗ (no asset committed) | — | Phase 4 task: produce monochrome glyph |
| Optional: Windows code-signing certificate | Future SmartScreen-warning suppression | ✗ (deferred D-24) | — | Document SmartScreen workaround in README; ship unsigned |

**Missing dependencies with no fallback:**
- Tauri signer keypair (required by DIST-02; planner must include "generate keypair, set GH secrets, delete local copy" as an explicit task or pre-flight step).
- `TAURI_SIGNING_PRIVATE_KEY` + `_PASSWORD` GH secrets must be created manually before first release tag.

**Missing dependencies with fallback:**
- Final SFX assets — fallback is to ship Phase 2 placeholders (rejected by D-29 wording but technically possible; decisions explicitly want a swap).
- Tray icon glyph — fallback is `app.default_window_icon()` (existing icon.ico) but it's the full app icon, not a tray-optimized glyph. Acceptable v1 fallback if no glyph designer available.

## Validation Architecture

> Skipped per `.planning/config.json` — `workflow.nyquist_validation: false`.

## Security Domain

> `security_enforcement` not explicitly set in `.planning/config.json`. Conservative inclusion below for the Phase 4 attack surface.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | yes | Threat-modelling for Phase 4 surface — see Threat Patterns below |
| V2 Authentication | no | No user auth — local-only desktop app |
| V3 Session Management | no | No session concept beyond Phase 1's `session_id` (already covered) |
| V4 Access Control | yes | HKCU registry write privilege check; per-user install ensures no admin escalation |
| V5 Input Validation | yes | `latest.json` and update payload validated by tauri-plugin-updater Ed25519 sig before any binary execution |
| V6 Cryptography | yes | Ed25519 keypair (NEVER hand-roll); `tauri signer` does the cryptography. Private key NEVER on local disk after upload (D-21) |
| V7 Errors & Logging | yes | `tracing` already wired; updater check failures logged at `warn` per existing convention |
| V8 Data Protection | yes | SQLite DB at `%APPDATA%\com.hallmark.app\` (per-user dir); registry value at HKCU (per-user); no globally-readable storage |
| V9 Communications | yes | HTTPS-only `connect-src` already in tauri.conf.json CSP; updater downloads from GitHub Releases over HTTPS; sig verifies the binary itself |
| V10 Malicious Code | yes | Updater's Ed25519 sig + pubkey-pinning is the malware-via-update prevention. Documentation must call out: "Lose the private key = orphan all installs" (D-21) |
| V11 Business Logic | yes | First-run flag must not be settable via untrusted input; only via the wizard's own dismissal command |
| V12 Files & Resources | yes | NSIS install path is per-user (LOCALAPPDATA); no admin-write surface |
| V13 API & Web Service | no | No web service exposed |
| V14 Configuration | yes | tauri.conf.json `pubkey` is the trust anchor; commit to repo (it's public); private key NEVER committed |

### Known Threat Patterns for Tauri 2 / desktop-app distribution

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious update served to all users | T (Tampering) | Ed25519 sig verification by tauri-plugin-updater (D-21) — pubkey pinned in tauri.conf.json |
| Loss of signing private key | D (Denial of Service — orphans installs) | Documented operational risk (D-21 note); accept-and-document for hobby OSS scope |
| HKLM\Run write attempt despite D-07 | E (Elevation of Privilege — needs admin) | Use `winreg` directly (Pitfall 1); never adopt the autostart plugin without verifying its hive |
| Tampered `latest.json` URL substitution (DNS hijack) | S/T (Spoofing/Tampering) | HTTPS to github.com + GitHub's TLS pinning at OS level + signature verification of the resulting payload |
| Updater downloads malicious .exe | T | Sig verification — even if URL was somehow tampered, the unsigned binary fails verification and updater aborts |
| User runs unsigned installer (SmartScreen warning) | I (Information Disclosure — no real impact) | Documented in README per D-24; future signing cert mitigates |
| Test-popup synth bypasses real schema | T (irrelevant — local user) | N/A — test popup is a feature, not a vector |
| Wizard `Continue anyway` swallows errors | I (debug info loss) | Tracing logs preserved; user can re-trigger via Settings → Rescan |
| Portable .zip extracted to a network share with malicious DLLs alongside | T | Out of scope — user-controlled environment |
| First-run flag manipulated by a malicious local actor | T (low impact — wizard re-fires) | Acceptable; flag is convenience not a security boundary |

**Critical:** D-21 mandates that the Ed25519 private key NEVER lives on local disk after CI secret upload. This is the hardest invariant to maintain across contributors over time. Document explicitly in the release-process docs.

## Sources

### Primary (HIGH confidence)
- Context7 `/tauri-apps/tauri-docs` — fetched topics: updater plugin (latest.json schema, signing, endpoints, restart API), tray icon (TrayIconBuilder + Menu/CheckMenu), autostart plugin (Rust+JS API), NSIS installer config (installMode, languages), tauri-action workflow examples, signing env vars (TAURI_SIGNING_PRIVATE_KEY).
- crates.io API — verified `tauri-plugin-updater 2.10.1` (2026-04-04), `tauri-plugin-autostart 2.5.1` (2025-10-27), `winreg 0.56.0` (2026-03-14), `tauri-plugin-process 2.3.1` (2025-10-27).
- `src-tauri/src/lib.rs` — verified existing `setup()` extension point, `raw_tx` mpsc channel position (line 223), AppState shape.
- `src-tauri/src/sources/mod.rs` — verified `RawUnlockEvent` struct + `SourceKind` enum.
- `src-tauri/src/watcher/mod.rs` — verified `run_watcher` + `run_pipeline` adapter→dedup boundary.
- `src-tauri/src/store/migrations/001_initial.sql` + `002_schema_cache.sql` — verified `settings(key, value)` table reuse for first_run_done.
- `src-tauri/src/audio.rs` — verified WAV decode validation at startup (lines 60-69).
- `src-tauri/Cargo.toml` — verified existing `winreg = "0.56"` pin under target-windows.
- `src-tauri/tauri.conf.json` — verified `bundle.active: false` Phase 1-3 default + existing CSP that already permits HTTPS connect.
- `package.json` + `vite.config.ts` — verified existing multi-entry rollup pattern (companion + popup) extends naturally to settings + wizard.

### Secondary (MEDIUM confidence)
- `github.com/tauri-apps/plugins-workspace/blob/v2/plugins/autostart/src/lib.rs` — inspected via WebFetch; confirmed plugin does NOT pin HKCU and delegates to auto-launch crate.
- `docs.rs/auto-launch/latest/auto_launch/` — confirmed crate's HKLM-first / HKCU-fallback default.
- `github.com/tauri-apps/tauri-action/blob/dev/action.yml` — verified inputs: `tagName`, `releaseName`, `releaseBody`, `releaseDraft`, `prerelease`, `uploadUpdaterJson` (default true), `updaterJsonPreferNsis`, `uploadUpdaterSignatures`.
- `freesound.org/help/faq` — confirmed CC0 redistribution permits OSS bundling without attribution.
- WebSearch: `tauri portable zip Windows bundle target` — confirmed Tauri has NO portable bundle target; .zip is custom step.

### Tertiary (LOW confidence — flagged for verification at plan time)
- `latest.json` precise field set returned by `tauri-action`'s auto-generation when only Windows is built — likely a single `windows-x86_64` platform key, but verify on first release.
- Whether Tauri 2.11.1 supports separate light/dark tray icons via `icon_as_template` parameter — likely macOS-specific, but verify Windows behavior in plan.

## Metadata

**Confidence breakdown:**
- Standard stack (versions, plugin APIs, config schema): HIGH — Context7 + crates.io + source inspection all aligned.
- Architecture patterns (tray, autostart, updater, first-run, portable detection): HIGH — every pattern verified against Tauri 2 docs and existing Hallmark code.
- Pitfalls (HKLM fallback, CheckMenuItem lag, portable updater bypass, SFX format mismatch): HIGH — derived from official documentation + existing code; not speculative.
- Code-signing-OFF acceptance (D-24 SmartScreen behavior): HIGH — well-known Windows behavior.
- SFX direction (D-28): MEDIUM — procedural is verifiably licensing-clean; CC0 path requires per-asset license-page verification at curation time.
- tauri-action edge cases (updaterJsonPreferNsis behavior with nsis-only target, latest.json version-substitution): MEDIUM — documented but untested for Hallmark's exact configuration.
- Long-term auto-launch crate behavior (A8): MEDIUM — could flip if upstream fixes the HKLM-default.

**Research date:** 2026-05-09
**Valid until:** 2026-06-09 (30 days for stable Tauri 2.11 + plugin 2.10 ecosystem; re-verify if Tauri 2.12 ships before then or `tauri-plugin-updater` 2.11+ releases)

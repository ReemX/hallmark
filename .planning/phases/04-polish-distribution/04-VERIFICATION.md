---
phase: 04-polish-distribution
verified: 2026-05-09T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Click tray 'Fire test popup' and observe popup fires through full pipeline"
    expected: "Popup appears with 'Test Achievement' / 'Hallmark is working correctly on your system.' + standard SFX"
    why_human: "Requires running app with Tauri dev server; cannot verify popup rendering or audio programmatically"
  - test: "Toggle 'Start with Windows' in tray menu on and off; verify HKCU registry entry"
    expected: "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\\Hallmark entry appears when toggled on, disappears when toggled off"
    why_human: "Registry write requires live Windows session with running Hallmark process"
  - test: "GitHub Actions workflow registration and first release smoke test (DIST-01 + DIST-03)"
    expected: "`git tag v0.1.0 && git push --tags` triggers release.yml; Release page shows hallmark-setup.exe, hallmark-setup.exe.sig, latest.json, hallmark-portable-0.1.0.zip"
    why_human: "Workflow exists locally and on remote master but gh api reports total_count=0 Actions workflows (likely first-repo onboarding lag). Actual tag-push smoke test not yet run."
  - test: "In-app update prompt fires when newer version is on GitHub Releases (DIST-02)"
    expected: "Companion window shows UpdateModal with version and Install button; Install triggers download + restart"
    why_human: "Requires two real published releases and a network connection to GitHub Releases CDN"
  - test: "First-run wizard appears on fresh install with correct N>0 / N=0 conditional rendering (DIST-04)"
    expected: "N>0: 'Welcome to Hallmark' heading + found sources + 'Get started' CTA. N=0: 'No sources detected yet' + all sources with details + Rescan + Get started + Continue anyway"
    why_human: "Requires clearing first_run_done flag and running full Tauri app; React rendering cannot be verified statically"
  - test: "Audio quality of three SFX variants — audition popup-standard.wav, popup-rare.wav, popup-100pct.wav"
    expected: "Each plays without distortion; standard < 600 ms; rare < 900 ms; 100pct note: same file as rare (D-28 fallback, deferred to v1.1)"
    why_human: "Subjective audio quality requires human listening; popup-100pct.wav is a copy of popup-rare.wav as documented in assets/sfx/README.md"
---

# Phase 4: Polish & Distribution Verification Report

**Phase Goal:** Any user can install Hallmark from a GitHub Release via a double-click NSIS installer or a portable zip, verify their installation fires a popup immediately via the test trigger, opt into start-with-Windows, receive in-app update prompts, and be guided through path discovery on first run — making the public release genuinely usable without a README deep-dive.
**Verified:** 2026-05-09
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can click "Fire test popup" from tray menu and see a sample popup through the full pipeline (SC#1, POL-01) | ✓ VERIFIED | `test_trigger::fire()` clones `AppState.raw_tx` and calls `blocking_send` with a real `RawUnlockEvent` (app_id=480, "HALLMARK_TEST_UNLOCK"). `seed_test_fixture` pre-seeds schema_cache with display_name="Test Achievement". Tray menu `fire_test` item calls `crate::test_trigger::fire(app)`. Full pipeline is wired. |
| 2 | Start-with-Windows toggle writes/removes `HKCU\...\Run\Hallmark` with `"<exe>" --silent` (SC#2, POL-02) | ✓ VERIFIED | `autostart.rs` implements `is_enabled()`, `enable()`, `disable()` via `winreg::RegKey::predef(HKEY_CURRENT_USER)` — HKLM never used. `format_run_value` quotes exe path. Tray toggle rebuilds menu on state change. Unit test `value_quoting_preserves_spaces_in_path` is present. |
| 3 | GitHub Actions workflow triggers on tag push and produces NSIS + portable .zip + latest.json (SC#3, DIST-01 + DIST-03) | ✓ VERIFIED (code); ? UNCERTAIN (runtime) | `.github/workflows/release.yml` exists (101 lines), triggers on `push: tags: v*.*.*` AND `workflow_dispatch`, runs `tauri-apps/tauri-action@v0` with `TAURI_SIGNING_PRIVATE_KEY`, adds portable .zip step via PowerShell `Compress-Archive`, uploads via `gh release upload`. Real pubkey is in `tauri.conf.json` (no PLACEHOLDER string). Runtime smoke test deferred per known gap (surfaced as human_verification). |
| 4 | When a newer version is available, Hallmark prompts in-app via tauri-plugin-updater (SC#4, DIST-02) | ✓ VERIFIED (code) | `updater_glue::spawn_background_check` calls `app.updater()?.check().await`, stashes `Update` on `AppState.pending_update`, emits `update-available` to companion. `main-companion.tsx` listens for event, sets `pendingUpdate`, shows `UpdateModal` on rising edge of companion visibility. `install_pending_update` command calls `update.download_and_install` then `app.restart()`. `manual_check_update` registered in `generate_handler!`. Runtime requires real GitHub release (human_verification). |
| 5 | On first launch, wizard scans for sources and surfaces found/not-found (SC#5, DIST-04) | ✓ VERIFIED (code) | `first_run.rs::open_wizard` builds 480×560 borderless window pointing at `wizard.html`. `lib.rs::run()` calls `open_wizard` when `first_run_done` is unset OR 0 paths detected. `FirstRunWizard.tsx` invokes `rescan_paths` on mount, conditionally renders N>0 ("Welcome to Hallmark") or N=0 ("No sources detected yet") variant. `wizard_dismiss` writes flag only when `cached_discovery` has ≥1 path. Runtime rendering needs human check. |

**Score:** 5/5 truths verified (code); 6 runtime behaviors need human testing

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/tray.rs` | Tray icon + D-01 menu + click handlers | ✓ VERIFIED | 203 lines, `TrayIconBuilder::with_id("hallmark-tray")`, `show_menu_on_left_click(false)`, all 6 menu IDs, `set_menu` on toggle, `test_trigger::fire` + `settings_window::open` calls, `app_clone.exit(0)` with 1.5s drain. No STUB. |
| `src-tauri/src/autostart.rs` | HKCU\Run write/remove | ✓ VERIFIED | 159 lines, `HKEY_CURRENT_USER` only, `format_run_value` quoting helper, `is_enabled`/`enable`/`disable` with `#[cfg(target_os = "windows")]` gates, cross-platform stubs. No HKLM. |
| `src-tauri/src/test_trigger.rs` | Synthetic unlock injector | ✓ VERIFIED | 179 lines, `blocking_send` (not try_send), `seed_test_fixture` uses `upsert_schema`, fixture copy exact match ("Test Achievement" / "Hallmark is working correctly on your system."), 3 unit tests present. |
| `src-tauri/src/portable_mode.rs` | `is_portable()` exe-path heuristic | ✓ VERIFIED | 114 lines, `current_exe()` + `dirs::data_local_dir()`, `is_portable_with` canonical compare, safe-default=false, 4 unit tests. `is_silent_launch` preserved. |
| `src-tauri/src/first_run.rs` | Wizard window builder | ✓ VERIFIED | 40 lines, `WebviewWindowBuilder`, `wizard.html`, 480×560, `decorations(false)`, idempotent. |
| `src-tauri/src/settings_window.rs` | Settings window builder | ✓ VERIFIED | 28 lines, `WebviewWindowBuilder`, `settings.html`, 520×580, `decorations(false)`, idempotent. |
| `src-tauri/src/updater_glue.rs` | Background-check + manual check | ✓ VERIFIED | 86 lines, `UpdaterExt`, `app.updater()?.check().await`, `emit_to("companion", "update-available", ...)`, `manual_check` returns `UpdateInfoView`, `persist_last_checked` writes to SQLite. |
| `src-tauri/src/lib.rs` | Integration spine with 7 Phase 4 modules | ✓ VERIFIED | All 7 `pub mod` declarations present (tray, autostart, test_trigger, first_run, settings_window, portable_mode, updater_glue). AppState has 8 fields including raw_tx, portable_mode, silent_launch, pending_update, cached_discovery. `tauri_plugin_updater::Builder` registered before invoke_handler. 8 commands registered (get_companion_state, set_companion_prefs_cmd, get_companion_prefs_cmd, rescan_paths, install_pending_update, wizard_dismiss, open_settings_window, manual_check_update). setup() calls tray::build_tray, updater_glue::spawn_background_check (gated on !portable_mode), first_run::open_wizard, test_trigger::seed_test_fixture. |
| `src-tauri/tauri.conf.json` | Real pubkey + NSIS + CSP | ✓ VERIFIED (with WARNING) | `bundle.active=true`, `targets=["nsis"]`, `createUpdaterArtifacts=true`. Pubkey is real base64 string (no PLACEHOLDER). CSP `connect-src` includes `https://github.com` + `https://objects.githubusercontent.com`. **WARNING:** `installMode` is `"currentUser"` in the file; plan 04-01b required `"perUser"`. Functionally equivalent in Tauri 2 ("currentUser" = perUser NSIS mode) but is a literal string mismatch from plan spec. |
| `.github/workflows/release.yml` | Tag-triggered release workflow | ✓ VERIFIED | 101 lines, triggers on `v*.*.*` and `workflow_dispatch`, `windows-latest`, `tauri-apps/tauri-action@v0`, `TAURI_SIGNING_PRIVATE_KEY` env var, portable .zip PowerShell step copies `src-tauri/target/release/hallmark.exe` (not bundle/ path), `gh release upload`, D-24 commented signtool placeholders. |
| `README.md` | Install + SmartScreen + portable + auto-update docs | ✓ VERIFIED | `## Install` heading, Installer + Portable subsections, SmartScreen "More info → Run anyway" wording, "Auto-update is disabled in portable mode", `## Auto-update` section, `cargo tauri dev` dev warning, `ReemX/hallmark` slug throughout. |
| `assets/sfx/popup-standard.wav` | Standard tier SFX, valid WAV | ✓ VERIFIED | File exists. README documents PCM 16-bit 48 kHz stereo. Maintainer-supplied (D-28 option 2). |
| `assets/sfx/popup-rare.wav` | Rare tier SFX, valid WAV | ✓ VERIFIED | File exists. |
| `assets/sfx/popup-100pct.wav` | 100% completion SFX, valid WAV | ✓ VERIFIED (with note) | File exists. Documented as copy of popup-rare.wav in assets/sfx/README.md. Dedicated celebration mix deferred to v1.1. This is an intentional documented decision, not a failure. |
| `assets/sfx/README.md` | License + format + provenance | ✓ VERIFIED | Documents PCM 16-bit 48 kHz stereo, D-28 maintainer-supplied path, license "unspecified for v1 — all rights reserved by maintainer", D-29 format constraint, DMCA hard rule, regeneration instructions. |
| `src/Settings.tsx` | Settings React page | ✓ VERIFIED | File exists with `invoke("rescan_paths")`, `invoke("manual_check_update")`, `invoke("install_pending_update")`, three sections (Detected Sources, Updates, About), MIT license in About. |
| `src/main-settings.tsx` | Settings entry point | ✓ VERIFIED | File exists. |
| `src/components/SettingsSourceRow.tsx` | Source row component | ✓ VERIFIED | File exists. |
| `src/components/UpdateModal.tsx` | Update modal with Framer Motion | ✓ VERIFIED | File exists, `invoke("install_pending_update")`, `motion.div` with scale 0.96→1.0 + opacity animation, Install and Later buttons. |
| `src/main-companion.tsx` | Companion extended with update modal | ✓ VERIFIED | `listen<UpdateInfo>("update-available", ...)`, `companionVisible` rising-edge logic, `AnimatePresence` + `UpdateModal` in all render branches. |
| `src/styles/settings.css` | Settings + modal + wizard CSS | ✓ VERIFIED | File exists with `.settings-shell`, `.update-modal-*`, `.wizard-shell` classes. |
| `src/FirstRunWizard.tsx` | Wizard React page N>0/N=0 | ✓ VERIFIED | File exists, `invoke("rescan_paths")`, `invoke("wizard_dismiss")`, "Welcome to Hallmark", "No sources detected yet", "Continue anyway", "Get started" all present. |
| `src/main-wizard.tsx` | Wizard entry point | ✓ VERIFIED | File exists. |
| `src/components/WizardSourceRow.tsx` | Wizard source row | ✓ VERIFIED | File exists. |
| `vite.config.ts` | 4-entry rollup config | ✓ VERIFIED | `settings: resolve(...)` and `wizard: resolve(...)` present. |
| `settings.html` | Vite entry for settings | ✓ VERIFIED | File exists at repo root. |
| `wizard.html` | Vite entry for wizard | ✓ VERIFIED | File exists (confirmed by `dist\settings.html` in dist, wizard.html at root). |
| `src-tauri/capabilities/settings.json` | Settings window capability | ✓ VERIFIED | File exists. |
| `src-tauri/capabilities/wizard.json` | Wizard window capability | ✓ VERIFIED | File exists. |
| `src-tauri/icons/tray.ico` | Tray icon asset | ✓ VERIFIED | File exists. |
| `src-tauri/src/store/queries.rs` | Phase 4 settings persistence helpers + tests | ✓ VERIFIED | `get_first_run_done`, `set_first_run_done`, `get_last_update_check`, `set_last_update_check` present. Tests `first_run_done_round_trip`, `last_update_check_round_trip`, `first_run_done_isolated_from_completion` at lines 372, 385, 398. |
| `src/types.ts` | Phase 4 type exports | ✓ VERIFIED | `SourceStatus`, `DiscoveredPathsView`, `UpdateInfo`, `FirstRunState` expected (verified via companion/wizard/settings imports working). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tray.rs::build_tray` | `autostart.rs::is_enabled/enable/disable` | Direct fn calls from `handle_menu_event` | ✓ WIRED | `crate::autostart::is_enabled()`, `crate::autostart::disable()`, `crate::autostart::enable()` all called in toggle handler |
| `tray.rs::fire_test handler` | `test_trigger::fire` | `crate::test_trigger::fire(app)` | ✓ WIRED | Present in `handle_menu_event` "fire_test" branch |
| `tray.rs::open_settings handler` | `settings_window::open` | `crate::settings_window::open(app)` | ✓ WIRED | Present in `handle_menu_event` "open_settings" branch |
| `test_trigger::fire` | `AppState.raw_tx` | `app.state::<crate::commands::AppState>().raw_tx.clone().blocking_send(evt)` | ✓ WIRED | Exact pattern in `test_trigger.rs` lines 33-34 |
| `test_trigger::seed_test_fixture` | `schema::cache::upsert_schema` | `store.with_conn(|c| upsert_schema(c, &row))` | ✓ WIRED | Line 90 in `test_trigger.rs` |
| `lib.rs::setup` | `tray::build_tray` | `tray::build_tray(app)` | ✓ WIRED | Line 490 in `lib.rs` |
| `lib.rs::setup` | `updater_glue::spawn_background_check` | `updater_glue::spawn_background_check(app_handle.clone())` gated on `!portable_mode` | ✓ WIRED | Lines 497-501 in `lib.rs` |
| `lib.rs::setup` | `first_run::open_wizard` | `first_run::open_wizard(app_handle.clone(), any_path_detected)` | ✓ WIRED | Lines 511-519 in `lib.rs` |
| `updater_glue::spawn_background_check` | `UpdaterExt::check` | `app.updater()?.check().await` | ✓ WIRED | Lines 14-21 in `updater_glue.rs` |
| `updater_glue` | `emit_to("companion", "update-available", ...)` | `app.emit_to("companion", "update-available", payload)` | ✓ WIRED | Line 43 in `updater_glue.rs` |
| `main-companion.tsx` | `update-available` event | `listen<UpdateInfo>("update-available", ...)` | ✓ WIRED | Lines 46-52 in `main-companion.tsx` |
| `UpdateModal.tsx` | `install_pending_update` command | `invoke("install_pending_update")` | ✓ WIRED | Line 22 in `UpdateModal.tsx` |
| `lib.rs::install_pending_update` | `update.download_and_install` then `app.restart()` | Body at lines 141-160 in `lib.rs` | ✓ WIRED | Real implementation (not stub) present |
| `Settings.tsx` | `rescan_paths` command | `invoke<DiscoveredPathsRust>("rescan_paths")` | ✓ WIRED | Present in `useEffect` and `handleRescan` |
| `FirstRunWizard.tsx` | `wizard_dismiss` command | `invoke("wizard_dismiss")` | ✓ WIRED | Both "Get started" and "Continue anyway" call `handleDismiss` which invokes `wizard_dismiss` |
| `tauri.conf.json plugins.updater.endpoints` | GitHub Releases latest.json | `"https://github.com/ReemX/hallmark/releases/latest/download/latest.json"` | ✓ WIRED | Exact URL in `tauri.conf.json` line 36 |
| `.github/workflows/release.yml` | `tauri-apps/tauri-action@v0` | `uses: tauri-apps/tauri-action@v0` | ✓ WIRED | Line 46 in release.yml |
| `autostart::enable` | `winreg::RegKey::predef(HKEY_CURRENT_USER)` | `RegKey::predef(HKEY_CURRENT_USER)` | ✓ WIRED | Line 54 in `autostart.rs`; HKLM absent (grep confirms) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `Settings.tsx` | `view: DiscoveredPathsView` | `invoke("rescan_paths")` → `crate::paths::discover()` | Yes — real filesystem scan via `tokio::task::spawn_blocking` | ✓ FLOWING |
| `main-companion.tsx` | `pendingUpdate: UpdateInfo` | `listen("update-available")` emitted by `updater_glue::spawn_background_check` after real HTTP check | Yes — real GitHub Releases check (requires network) | ✓ FLOWING (code path complete; runtime gated on network) |
| `FirstRunWizard.tsx` | `view: DiscoveredPathsView` | `invoke("rescan_paths")` on mount | Yes — same real scan path | ✓ FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED for most items — requires running Tauri app with real audio device, real registry, and real network. Items routed to human_verification section.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `tray.rs` no STUB strings | grep -q "STUB" src-tauri/src/tray.rs | No match | ✓ PASS |
| `autostart.rs` no STUB, no HKLM | grep -q "HKEY_LOCAL_MACHINE" src-tauri/src/autostart.rs | No match | ✓ PASS |
| `test_trigger.rs` no STUB | grep -q "STUB" src-tauri/src/test_trigger.rs | No match | ✓ PASS |
| `settings_window.rs` no STUB | grep -q "STUB" src-tauri/src/settings_window.rs | No match | ✓ PASS |
| `updater_glue.rs` no STUB | grep -q "STUB" src-tauri/src/updater_glue.rs | No match | ✓ PASS |
| `first_run.rs` no STUB | grep -q "STUB" src-tauri/src/first_run.rs | No match | ✓ PASS |
| pubkey not PLACEHOLDER | grep -q "PLACEHOLDER_REPLACE_AT_RELEASE" tauri.conf.json | No match | ✓ PASS |
| release.yml exists + tauri-action | File exists; grep "tauri-action" | Match at line 46 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| POL-01 | 04-03 | Fire test popup via tray | ✓ SATISFIED | `test_trigger::fire` injects synthetic RawUnlockEvent via `raw_tx.blocking_send`; `seed_test_fixture` pre-seeds schema_cache; tray menu wires to it |
| POL-02 | 04-02 | Start-with-Windows registry toggle | ✓ SATISFIED | `autostart::enable`/`disable` write `HKCU\...\Run\Hallmark` with quoted exe path + `--silent`; tray menu toggles and rebuilds on state change |
| DIST-01 | 04-06 | NSIS installer + portable .zip | ✓ SATISFIED (code) | `bundle.targets=["nsis"]`, `createUpdaterArtifacts=true` in `tauri.conf.json`; `.github/workflows/release.yml` produces portable .zip via Compress-Archive. End-to-end runtime deferred (human_verification) |
| DIST-02 | 04-04 | Auto-updater wired to GitHub Releases | ✓ SATISFIED (code) | `updater_glue::spawn_background_check` + `install_pending_update` command + `UpdateModal` in companion + real pubkey in `tauri.conf.json`. Runtime requires real release (human_verification) |
| DIST-03 | 04-06 | GitHub Actions release workflow on tag push | ✓ SATISFIED (code) | `.github/workflows/release.yml` exists and is correctly structured. Actual tag push deferred (human_verification — known Actions onboarding lag) |
| DIST-04 | 04-05 | First-run path-discovery wizard | ✓ SATISFIED (code) | `first_run.rs::open_wizard` + `FirstRunWizard.tsx` N>0/N=0 conditional rendering + `wizard_dismiss` writes first_run_done. Runtime rendering requires human check |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `tauri.conf.json` | 28 | `"installMode": "currentUser"` vs plan-specified `"perUser"` | ⚠ Warning | In Tauri 2 NSIS bundler, "currentUser" and "perUser" are functionally identical (both install to `%LOCALAPPDATA%\Hallmark` without UAC). No functional impact. |
| `assets/sfx/popup-100pct.wav` | — | Is a copy of `popup-rare.wav` | ℹ Info | Documented intentional decision in `assets/sfx/README.md`. Dedicated celebration mix deferred to v1.1. popup_queue celebration ordering still works. |
| `assets/sfx/README.md` | 29 | License "unspecified for v1, all rights reserved by maintainer" | ⚠ Warning | Maintainer-supplied audio with unspecified license means forks cannot legally redistribute the binary without swapping assets. Documented as known v1 limitation. |

### Human Verification Required

#### 1. Test popup fires through full pipeline (POL-01 / SC#1)

**Test:** Launch Hallmark via `cargo tauri dev`. Click tray icon right-click → "Fire test popup".
**Expected:** Popup appears with title "Test Achievement", description "Hallmark is working correctly on your system.", and the standard SFX plays. Popup animates in and out. Wait 11s, click again — second popup fires. Click twice within 10s — second click suppressed by dedup TTL.
**Why human:** Requires running Tauri app with audio device and real popup window rendering.

#### 2. Start-with-Windows toggle (POL-02 / SC#2)

**Test:** Right-click tray → "Start with Windows" to enable. Run `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"`. Toggle off. Run query again.
**Expected:** Hallmark entry appears with quoted exe path + `--silent` when on; entry absent when off.
**Why human:** Requires live Windows session with running Hallmark process and real registry access.

#### 3. GitHub Actions workflow first run (DIST-03 / SC#3)

**Test:** Confirm workflow is indexed at https://github.com/ReemX/hallmark/actions (or trigger via workflow_dispatch). OR run `git tag v0.1.0 && git push --tags`.
**Expected:** Workflow completes; GitHub Release page shows `hallmark-setup.exe`, `.sig`, `latest.json`, `hallmark-portable-0.1.0.zip`.
**Why human:** `gh api repos/ReemX/hallmark/actions/workflows` currently returns total_count=0 despite release.yml existing on remote master — known first-repo Actions onboarding delay. Web UI confirmation or actual tag push required.

#### 4. In-app update prompt (DIST-02 / SC#4)

**Test:** Install v0.1.0 from GitHub Release. Push v0.1.1 tag. Open v0.1.0 companion window.
**Expected:** UpdateModal appears with "Update available", version "0.1.1", notes, and "Install and Restart Hallmark" button. Click Install — Hallmark restarts as v0.1.1.
**Why human:** Requires two real published releases and GitHub network connectivity. Cannot be simulated without a live release.

#### 5. First-run wizard conditional rendering (DIST-04 / SC#5)

**Test (N>0 case):** Clear `hallmark.db` first_run_done flag. Launch with Steam installed. Wizard opens.
**Expected:** "Welcome to Hallmark" heading; found sources listed with ✓; single "Get started" button.
**Test (N=0 case):** Run on a machine with no Steam/Goldberg/CreamAPI/SSE.
**Expected:** "No sources detected yet" heading; all 4 sources with ✗ + detail; "Rescan", "Get started", "Continue anyway" buttons. Click "Continue anyway" → wizard closes; relaunch → wizard re-opens.
**Why human:** Requires running Tauri app on a clean or specific configuration; React rendering cannot be statically verified.

#### 6. Audio quality (D-28 / SFX)

**Test:** Fire test popup (standard), trigger rare-tier unlock, observe popup-100pct.wav at 100% completion.
**Expected:** Standard and rare SFX play without distortion and feel premium. Note: popup-100pct.wav is currently a copy of popup-rare.wav (documented in assets/sfx/README.md — dedicated celebration deferred to v1.1).
**Why human:** Subjective audio quality; format is technically valid (PCM 16-bit 48 kHz stereo).

## Gaps Summary

No BLOCKER gaps identified. All code artifacts are substantive and wired. The phase goal is observably implemented in the codebase.

The six human verification items are runtime confirmation items that cannot be resolved programmatically:
- Items 1, 2, 5, 6 require a running Tauri app session (local smoke tests)
- Items 3, 4 require network/GitHub Actions infrastructure (first-release gates)

The `installMode: "currentUser"` discrepancy from plan spec is functionally harmless (Tauri 2 treats "currentUser" identically to "perUser" for NSIS; installs to `%LOCALAPPDATA%\Hallmark` without UAC either way).

The SFX license being "unspecified for v1" is a known documented decision, not a code defect. Forks redistributing the binary are advised in `assets/sfx/README.md` to swap these assets.

---

_Verified: 2026-05-09_
_Verifier: Claude (gsd-verifier)_

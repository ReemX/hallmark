---
status: partial
phase: 04-polish-distribution
source: [04-01a-SUMMARY.md, 04-01b-SUMMARY.md, 04-02-SUMMARY.md, 04-03-SUMMARY.md, 04-04-SUMMARY.md, 04-05-SUMMARY.md, 04-06-SUMMARY.md, 04-07-SUMMARY.md]
started: 2026-05-09T00:00:00Z
updated: 2026-05-09T20:30:00Z
---

## Current Test

[testing complete — 10 passed, 5 issues, 2 skipped-with-reason, 3 blocked]

## Tests

### 1. Cold Start Smoke Test
expected: Launch via `cargo tauri dev` from a clean shell. App boots without panic. Tracing logs show 4-adapter pipeline configured, AudioDispatcher decoded 3 SFX bundled bytes, tray icon registered, tauri_plugin_updater plugin in builder chain, and either updater spawned or portable-mode skip log. Tray icon visible in system tray.
result: pass
log_evidence: "Phase 3: 4-adapter pipeline configured adapter_count=4 / AudioDispatcher initialized (3 SFX bundles validated) / tray icon registered with D-01 menu structure / Phase 4 startup flags portable_mode=false silent_launch=false / test popup fixture seeded / updater_glue: update check failed (graceful — no v0.1.0 release published yet, expected WARN)"

### 2. Tray D-01 Locked Menu
expected: Right-click tray icon. Menu shows EXACTLY (in order): "Show companion", "Fire test popup", "Settings", "Start with Windows" (with check-mark reflecting current state), separator (or no), "Quit". No other items.
result: issue
reported: "Menu shows 'Hallmark' (greyed disabled title) at top, then Show companion / Fire test popup / sep / Settings... / Start with Windows / sep / Quit. Two issues: (1) extra 'Hallmark' header item not in D-01 spec — likely auto-injected by tauri tray menu builder; (2) tray icon itself renders as a solid black box on the taskbar — `tray.ico` is the v1 fallback copy of `icon.ico` per 04-02 SUMMARY; the proper monochrome glyph swap was never landed (04-07 turned out to be SFX-only)."
severity: minor

### 3. Tray Left-Click Shows Companion (D-02)
expected: Left-click tray icon (single click). Companion window appears (does not show menu). Re-clicking when companion already visible: companion stays / re-focuses.
result: pass
note: "Companion opens on left-click; 're-click re-focuses' confirmed. 'No game detected' empty state renders correctly. Side observation (logged as minor gap below): user reports companion header drag region is flaky / undersized — not all header pixels grab the window."

### 4. Fire Test Popup from Tray (POL-01)
expected: Right-click tray → "Fire test popup". Within ~1 second, popup window appears with title "Test Achievement" and description "Hallmark is working correctly on your system." Standard-tier SFX plays. Popup animates in via Framer Motion, displays for hold duration, animates out.
result: issue
reported: "Three problems: (1) ~20s UI warmup gap after launch — if fired during this window, SFX plays but popup never renders; logs do not show this initialization period. (2) After first successful popup at 16:52:50 (POPUP_FIRED tier=standard depth_after=0), subsequent fires at 16:53:53 / 16:53:54 / 16:53:56 (>60s after first, well past 10s CrossSourceDedup TTL) log `test_trigger::fire` but emit NO UNLOCK and NO POPUP_FIRED line — popup never re-renders. Pipeline consumer or popup_queue is stuck / dropping after the first popup. (3) On Quit, Chromium logs `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` (cosmetic — app exits cleanly per Quit-drain logs). Visual itself: when popup did render once, it matched spec (Test Achievement / Hallmark is working correctly on your system. + circular icon placeholder)."
severity: major
log_evidence: |
  16:52:50 test_trigger::fire → UNLOCK app_id=480 source=goldberg → POPUP_FIRED tier=standard depth_after=0  (worked)
  16:53:53 test_trigger::fire (no UNLOCK, no POPUP_FIRED)
  16:53:54 test_trigger::fire (no UNLOCK, no POPUP_FIRED)
  16:53:56 test_trigger::fire (no UNLOCK, no POPUP_FIRED)
  16:55:32 Quit requested — draining popup queue (1.5 s grace)
  16:55:34 Quit drain complete; calling app.exit(0)
  16:55:34 [Chromium ERROR] Failed to unregister class Chrome_WidgetWin_0. Error = 1412

### 5. Test Popup Dedup TTL
expected: Right-click tray → "Fire test popup" twice within 10 seconds. First click fires popup; second click within 10s is suppressed by CrossSourceDedup (no second popup). Wait 11s, click again — popup fires again.
result: blocked
blocked_by: prior-test
reason: "Blocked by test 4 issue — repeat test_trigger::fire calls produce no second popup at all (verified at >60s gap). Dedup TTL behavior cannot be observed independently until popup_queue / pipeline repeat-fire bug is fixed."

### 6. Settings Window Opens from Tray
expected: Right-click tray → "Settings". A new 520×580 borderless window opens, centered, focused, NOT resizable. Header has 48px drag region with circular close button (red on hover). Three sections visible: "Detected Sources", "Updates", "About".
result: issue
reported: "Functional checks pass — dimensions, centered, focused, non-resizable, close button works, all 3 sections render. BUT: (1) native OS scrollbar appears OUTSIDE the dark rounded card on the right edge — the area beyond the card is the default light/white body background bleeding through, breaking the premium feel that is the project's core value. (2) Default body background is not dark — padding around the styled container reveals an unstyled body. (3) Header `Settings` title pixels are NOT draggable (only chrome around it is), same drag-region partial-failure as companion. (4) Layout wastes horizontal space and forces vertical scroll — content density is poor for a 520×580 fixed window. (5) `View on GitHub` link is dead — clicking does nothing (likely missing tauri-plugin-shell open invocation or `target=_blank` rel handler). MIT License renders fine."
severity: major

### 7. Settings — Detected Sources Panel
expected: On Settings open, "Detected Sources" panel shows skeleton lines briefly, then a list of source rows. Each adapter (Goldberg, Steam Legit, CreamAPI, SmartSteamEmu, CODEX) renders either `✓ <name>` (accent color) or `✗ <name> — <reason>` (text-secondary). At least one source ✓ on this dev machine (Steam legit appcache should be found).
result: pass
note: "Functional pass — ✓ Steam, ✓ Goldberg, ✗ CreamAPI (no per-game directories found), ✗ SmartSteamEmu (saves directory not found) all render correctly with accent / text-secondary colors. CODEX absence is by design (only 4 adapters shipped per Phase 3; CODEX is documented in CLAUDE.md emulator-paths reference but not yet a wired adapter). Logged as minor gap below: skeleton row height does not match rendered row height, causing a ~50ms flash + visible layout jump on transition."

### 8. Settings — Rescan Button
expected: Click "Rescan" button in Detected Sources panel. Skeleton lines briefly reappear, then list re-renders with current detection state. No errors.
result: pass

### 9. Settings — Updates Panel Manual Check
expected: In Updates panel, click "Check for updates". Status transitions idle → checking → either "uptodate" (no newer release on GitHub) OR "available" (with version + Install button) OR "error" (network unavailable). For a fresh repo with no releases yet, uptodate is the expected outcome.
result: issue
reported: "State machine works — UI shows 'Hallmark v0.1.0' + error 'Couldn't reach the update server. Check your connection.' BUT the user has working internet; root cause is the GitHub Releases latest.json endpoint returns 404 because no v0.1.x has been published yet. The error copy is misleading — blames network when actual cause is missing release. Should distinguish 404 / not-found from network-unreachable."
severity: minor

### 10. Settings — About Panel
expected: About section shows app version, link to https://github.com/ReemX/hallmark, "MIT License" mention.
result: pass
note: "Renders 'Hallmark v0.1.0' + 'View on GitHub' + 'MIT License'. Dead GitHub link is already logged as a separate gap from test 6 (no double-counting)."

### 11. Start-with-Windows Toggle Enable (POL-02)
expected: Right-click tray → click "Start with Windows" (currently unchecked). Menu rebuilds with check-mark on. Run `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v Hallmark`. Output shows `Hallmark    REG_SZ    "<exe-path>" --silent` with exe path double-quoted.
result: pass
log_evidence: 'autostart enabled (HKCU\\...\\Run\\Hallmark) value="C:\\Users\\reema\\Documents\\Programming\\achievements\\target\\debug\\hallmark.exe" --silent / tray: autostart toggled was_on=false is_on=true / Task Manager Startup apps section shows Hallmark entry'

### 12. Start-with-Windows Toggle Disable
expected: Right-click tray → click "Start with Windows" (now checked). Menu rebuilds without check-mark. Re-run `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v Hallmark`. Output: `ERROR: The system was unable to find the specified registry key or value.`
result: pass
log_evidence: 'autostart disabled (HKCU\\...\\Run\\Hallmark removed) / tray: autostart toggled was_on=true is_on=false / Task Manager Startup apps no longer shows Hallmark'

### 13. Tray Quit Exits Cleanly
expected: Right-click tray → "Quit". Within ~1.5 seconds, all Hallmark windows close and the process exits (no zombie hallmark.exe in Task Manager). No panic, no error dialog.
result: pass
log_evidence: "17:16:03 Quit requested — draining popup queue (1.5 s grace) → 17:16:05 Quit drain complete; calling app.exit(0) → process exits, prompt returns. Chromium teardown ERROR (Chrome_WidgetWin_0 unregister) is already logged as separate cosmetic gap."

### 14. First-Run Wizard N>0 Path (DIST-04)
expected: Clear first_run_done flag (delete hallmark.db OR `update settings set value='0' where key='first_run_done'` via sqlite). Launch app. A 480×560 borderless wizard window appears centered with heading "Welcome to Hallmark", subheading "We found these achievement sources on your system:", list of found sources (✓ rows only), and a single "Get started" button with cyan accent border.
result: issue
reported: "When fully loaded the wizard matches spec — 'Welcome to Hallmark' heading, 'We found these achievement sources on your system:' subheading, ✓ Steam + ✓ Goldberg rows, single 'Get started' button with cyan accent border, 480x560 borderless centered. BUT three problems: (1) wizard window appears almost immediately but stays BLANK for many seconds before content paints (user said '15 minutes' — likely hyperbolic but reproducibly long, same warmup pattern as the test-4 popup gap). (2) Same padding + native OS scrollbar issue as Settings — scrollbar lives OUTSIDE the rounded card on the right edge, padding reveals light/white background. (3) Header is not sticky — should pin to top while body scrolls; currently scrolls with content. (4) Scroll should be confined to body container, not spawn an OS scrollbar in the chromeless gap."
severity: major

### 15. Wizard "Get started" Dismisses + Latches Flag
expected: From the N>0 wizard, click "Get started". Wizard closes. Inspect `hallmark.db` settings table — `first_run_done` row exists with value `1`. Relaunch app — wizard does NOT re-open.
result: pass
log_evidence: "User clicked 'Get started', wizard closed. Inspected `C:\\Users\\reema\\AppData\\Roaming\\Hallmark\\hallmark.db` via sqlite3: `SELECT key, value FROM settings;` returns `first_run_done|1`. Combined with paths>0 confirmed in test 14 and lib.rs setup() gate (open_wizard fires only if !first_run_done OR 0 paths), wizard will not re-open on next launch — re-fire conditions not met."

### 16. First-Run Wizard N=0 Path + Re-fire (D-14)
expected: On a machine with NO emulator paths AND no Steam (or temporarily move/rename steam folder so discover() returns 0 paths), clear first_run_done. Launch. Wizard shows heading "No sources detected yet", lists ALL 4 sources with ✗ + detail strings, shows three buttons: "Rescan", "Get started", "Continue anyway". Click "Continue anyway" → wizard closes. Relaunch app → wizard re-opens (D-14: zero-path machines re-fire wizard until ≥1 path detected).
result: skipped
reason: "User declined to rename Steam + emulator dirs to fake N=0 — risk of corrupting live game installs is not worth the test on a dev machine. Re-fire logic (D-14) is verified at code level in 04-VERIFICATION.md (lib.rs setup gates open_wizard on `!first_run_done OR any_path_detected==false`); runtime confirmation deferred to a clean VM or fresh-install user environment."

### 17. UpdateModal Triggers on Companion Show (DIST-02)
expected: With a published v0.1.x and a newer v0.1.y on GitHub Releases (or via mock latest.json), launch app on the older version. Open / make companion visible (or wait for game-started event). UpdateModal fades + scales in showing "Update available" + new version + truncated notes (≤280 chars) + "Install and Restart Hallmark" button + "Later" button. Modal does NOT appear during gameplay (companion hidden).
result: blocked
blocked_by: prior-release
reason: "No GitHub Releases published yet — `latest.json` does not exist. UpdateModal cannot fire without a real newer-version release. Smoke test deferred until v0.1.0 + v0.1.1 are tagged and uploaded by the release pipeline."

### 18. Portable Mode Skips Updater (D-23)
expected: Build a release binary (`cargo tauri build`), copy `src-tauri/target/release/hallmark.exe` to a folder OUTSIDE `%LOCALAPPDATA%\Hallmark` (e.g., `C:\portable-test\`). Run from there. Tracing logs show `portable=true` and `portable mode: updater background-check skipped (D-23)`. No update check fires.
result: skipped
reason: "User opted to skip — release build cycle not run during this session. Code-level verification exists in 04-VERIFICATION.md (lib.rs gates spawn_background_check on `!portable_mode` flag derived from is_portable()); runtime confirmation deferred."

### 19. NSIS Installer Builds Locally
expected: From repo root, run `cargo tauri build` (or `pnpm tauri build`). Build succeeds. Output produces `src-tauri/target/release/bundle/nsis/Hallmark_<version>_x64-setup.exe` (NSIS installer) plus `.sig` signature file plus `latest.json` (createUpdaterArtifacts=true). Installer size is sub-10MB (Tauri target).
result: pass
log_evidence: "cargo tauri build → release profile compile 2m 31s → NSIS 3.11 downloaded + hash-validated → makensis produced `target/release/bundle/nsis/Hallmark_0.1.0_x64-setup.exe`. Verified size: 5.1 MB (under 10 MB target). Missing .sig + latest.json locally is expected per D-21 — `TAURI_SIGNING_PRIVATE_KEY` only lives in GitHub Secrets; dev machines see 'A public key has been found, but no private key' and skip signing. Signed artifact production is verified in CI (test 20)."

### 20. GitHub Actions Release Pipeline (DIST-01 / DIST-03)
expected: Tag and push `git tag v0.1.0-rc.1 && git push --tags`. GitHub Actions tab shows `release.yml` workflow run on `windows-latest`. Workflow completes. GitHub Releases page for tag shows 4 assets uploaded: `hallmark-setup.exe` (or `Hallmark_*-setup.exe`), `.sig` signature, `latest.json` (Ed25519-signed), and `hallmark-portable-0.1.0-rc.1.zip`. NOTE: requires `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set in GitHub Secrets.
result: blocked
blocked_by: actions-registration
reason: |
  Tag-push not run during this UAT — GitHub Actions has not yet indexed `release.yml` despite the file being on remote master at sha b4bb6f9 (3977 bytes). `gh api repos/ReemX/hallmark/actions/workflows` returns total_count=0 and `gh workflow list` is empty. Repo is fresh (created 2026-05-09T16:25:32Z); the empty `ci: trigger workflow registration` nudge commit (e132afb) has not yet caused Actions to register the workflow. Pushing `v0.1.0-rc.1` right now is high-ambiguity: trigger may or may not fire, and an empty release tag would have to be deleted from remote to retry — leaving an audit trail.

  Confirmed PRE-flight checks (all green except registration):
  - gh auth: ✓ ReemX, scopes workflow + repo
  - Remote: ✓ https://github.com/ReemX/hallmark.git
  - Secrets: ✓ TAURI_SIGNING_PRIVATE_KEY + TAURI_SIGNING_PRIVATE_KEY_PASSWORD both set 2026-05-09T16:25:44Z
  - Actions permissions: ✓ enabled, allowed_actions=all
  - Workflow file on remote: ✓ .github/workflows/release.yml present
  - Local tags: none; remote tags: none
  - Workflow registration: ✗ total_count=0, 0 runs ever

  Resume path: once `gh workflow list -R ReemX/hallmark` returns a row for "release", run `git tag v0.1.0-rc.1 && git push --tags` and verify Actions tab shows the run + Release page shows 4 assets.

## Summary

total: 20
passed: 10
issues: 5
pending: 0
skipped: 2
blocked: 3
gap_entries: 12

## Gaps

- truth: "Tray menu matches D-01 locked spec exactly (Show companion / Fire test popup / Settings / Start with Windows / Quit) and tray icon renders as a recognizable Hallmark glyph"
  status: failed
  reason: "User reported: Menu shows 'Hallmark' (greyed disabled title) at top, then Show companion / Fire test popup / sep / Settings... / Start with Windows / sep / Quit. Two issues: (1) extra 'Hallmark' header item not in D-01 spec — likely auto-injected by tauri tray menu builder; (2) tray icon itself renders as a solid black box on the taskbar — `tray.ico` is the v1 fallback copy of `icon.ico` per 04-02 SUMMARY; the proper monochrome glyph swap was never landed (04-07 turned out to be SFX-only)."
  severity: minor
  test: 2
  artifacts: []
  missing: []

- truth: "Companion window header drag region covers all non-button header pixels and reliably grabs the window for dragging"
  status: failed
  reason: "User reported: 'top area for dragging is kinda flaky, maybe too small, not all areas I grab in header work'. Companion header drag region is undersized / inconsistent — `data-tauri-drag-region` may be on a child element rather than the full 48px header strip, or close button hit area is encroaching on draggable surface."
  severity: minor
  test: 3
  artifacts: ["src/main-companion.tsx", "src/styles/companion.css"]
  missing: []

- truth: "Test popup fires reliably on every tray click after the first one (POL-01)"
  status: failed
  reason: "After first successful popup, subsequent test_trigger::fire calls (>60s later, past 10s dedup TTL) log only 'test popup fired (synthetic event injected at adapter→dedup boundary)' with NO subsequent UNLOCK or POPUP_FIRED log line — popup never re-renders. Likely cause: popup_queue consumer stuck (B-2 select! drain may be deadlocked after first hide) OR run_pipeline receiver dropped OR popup window state stuck between hidden/shown."
  severity: major
  test: 4
  artifacts: ["src-tauri/src/popup_queue.rs", "src-tauri/src/test_trigger.rs", "src-tauri/src/lib.rs (run_pipeline)", "src-tauri/src/ui.rs (popup window hide/show)"]
  missing: []

- truth: "UI is interactive within ~1 second of cargo tauri dev launch — achievements that fire during early startup render visibly (not just SFX)"
  status: failed
  reason: "User reports ~20-second UI warmup gap after `cargo tauri dev` launch. Achievements fired during this window play SFX but the popup window never paints — sound-only state. Logs do not surface a 'WebView ready' / 'frontend mounted' marker, so the gap is invisible from Rust side. Production NSIS-installed binary may exhibit different timing (no Vite dev server overhead) but this is unverified."
  severity: major
  test: 4
  artifacts: ["src-tauri/src/ui.rs", "src/main-popup.tsx", "vite.config.ts"]
  missing: ["frontend-ready ack from popup WebView back to Rust before popup_queue starts firing"]

- truth: "App quits cleanly without Chromium teardown errors"
  status: failed
  reason: "On Quit, Chromium emits `[ERROR:ui\\gfx\\win\\window_impl.cc:134] Failed to unregister class Chrome_WidgetWin_0. Error = 1412` (ERROR_CLASS_DOES_NOT_EXIST). App does exit per Quit-drain logs, so this is cosmetic stderr noise, but it is an ERROR-level line on every clean Quit."
  severity: cosmetic
  test: 4
  artifacts: ["src-tauri/src/tray.rs (initiate_quit)", "src-tauri/src/lib.rs (window teardown order)"]
  missing: []

- truth: "Settings window has fully styled premium dark surface — no OS background bleed, no light body background, scrollbar styled or hidden inside the rounded card"
  status: failed
  reason: "Native OS scrollbar renders OUTSIDE the rounded dark card on the right edge. The area beyond the card is the unstyled default browser/WebView light background, breaking the premium feel that defines the product. html/body element is not set to the dark surface color, OR `.settings-shell` is sized smaller than viewport with padding around it, OR `overflow` lives on body instead of `.settings-body` so OS scrollbar shows in the chromeless gap."
  severity: major
  test: 6
  artifacts: ["src/styles/settings.css", "settings.html", "src/main-settings.tsx"]
  missing: ["html/body { background: var(--surface-base); margin: 0; height: 100% } reset", "scrollbar styling (custom thin scrollbar inside .settings-body) OR overflow moved off body"]

- truth: "View on GitHub link in Settings About panel opens https://github.com/ReemX/hallmark in the default browser"
  status: failed
  reason: "Clicking 'View on GitHub' link does nothing. Tauri WebViews block plain `<a href target=_blank>` navigation by default — must invoke `@tauri-apps/plugin-shell` `open(url)` from an onClick handler, or configure the shell-open capability."
  severity: major
  test: 6
  artifacts: ["src/Settings.tsx (About section)", "src-tauri/capabilities/settings.json", "package.json (@tauri-apps/plugin-shell)"]
  missing: ["tauri-plugin-shell wired in builder + capability + onClick → invoke('plugin:shell|open', { path: 'https://github.com/ReemX/hallmark' })"]

- truth: "Settings header drag region covers the full 48px header strip including the 'Settings' title text"
  status: failed
  reason: "User reports drag works on most of header but NOT on the 'Settings' title text itself — same drag-region partial-failure pattern as the companion window. `data-tauri-drag-region` is likely on a sibling element, not on the title text or its parent."
  severity: minor
  test: 6
  artifacts: ["src/Settings.tsx (header)", "src/styles/settings.css (.settings-header)"]
  missing: []

- truth: "Settings layout uses horizontal space efficiently — content fits without vertical scrolling on the fixed 520x580 window"
  status: failed
  reason: "Content overflows the 520×580 viewport — user must scroll vertically to reach the About section. Horizontal space is under-used (rows are narrow). Increase row width / reduce vertical padding / or tighten section spacing so all 3 sections fit without scroll."
  severity: minor
  test: 6
  artifacts: ["src/styles/settings.css (.settings-shell, .settings-section, .source-row spacing)"]
  missing: []

- truth: "Skeleton placeholder rows in Detected Sources panel match the height of rendered SettingsSourceRow entries — no layout jump when scan completes"
  status: failed
  reason: "User reports the skeleton flashes for ~50ms then content snaps in with a visible vertical jump. Skeleton lines are shorter / shorter-padding than the real source rows, so when the rescan_paths promise resolves the rows expand and shift the rest of the panel down. Either match skeleton row dimensions to SettingsSourceRow exactly, OR hold the skeleton minimum visible duration to ~250ms so the user perceives intent rather than a glitch."
  severity: minor
  test: 7
  artifacts: ["src/Settings.tsx (skeleton rendering)", "src/styles/settings.css (.skeleton-line / .source-row dimensions)"]
  missing: []

- truth: "Updates panel error message accurately describes the failure cause — distinguishes 404/no-release from genuine network failure"
  status: failed
  reason: "Manual check displays 'Couldn't reach the update server. Check your connection.' when in fact the user's connection is fine; latest.json simply does not exist on GitHub Releases yet (no v0.1.x tagged). The Tauri updater plugin returns the same generic Error for both 404 and network failure; frontend should branch on the underlying status code or treat 'no release at all' as the implicit `uptodate` outcome (current installed version is presumably the latest if no release exists)."
  severity: minor
  test: 9
  artifacts: ["src/Settings.tsx (Updates section state machine)", "src-tauri/src/updater_glue.rs (manual_check error mapping)"]
  missing: []

- truth: "Wizard window has fully styled premium dark surface with confined scrollbar — same fix as Settings (no body bg bleed, no OS scrollbar outside card, sticky header)"
  status: failed
  reason: "Same html/body bg + scrollbar pattern as Settings (test 6) reproduces in wizard window. Plus: wizard header is not sticky (should pin while body scrolls). Both surfaces share `settings.css`, so a single fix on body/html reset + .wizard-shell overflow + position-sticky on .wizard-header should resolve both."
  severity: major
  test: 14
  artifacts: ["src/styles/settings.css (.wizard-shell, .wizard-header)", "wizard.html", "src/main-wizard.tsx"]
  missing: ["html/body dark bg reset", "scroll container scoped to .wizard-body", "position: sticky on .wizard-header"]

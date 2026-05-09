---
status: diagnosed
phase: 04-polish-distribution
source: [04-01a-SUMMARY.md, 04-01b-SUMMARY.md, 04-02-SUMMARY.md, 04-03-SUMMARY.md, 04-04-SUMMARY.md, 04-05-SUMMARY.md, 04-06-SUMMARY.md, 04-07-SUMMARY.md]
started: 2026-05-09T00:00:00Z
updated: 2026-05-09T21:15:00Z
diagnosis_complete: 2026-05-09T21:15:00Z
debug_sessions:
  - .planning/debug/popup-repeat-fire-stuck.md
  - .planning/debug/webview-warmup-blank-screen.md
  - .planning/debug/settings-wizard-css-surface-regression.md
  - .planning/debug/drag-region-undersized.md
  - .planning/debug/tray-menu-extra-header-and-black-icon.md
  - .planning/debug/github-link-dead.md
  - .planning/debug/updates-error-wording-misleading.md
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
  status: diagnosed
  reason: "User reported: Menu shows 'Hallmark' (greyed disabled title) at top + tray icon renders as a solid black box."
  severity: minor
  test: 2
  root_cause: |
    TWO sub-defects with separate root causes:
    (1) Hallmark header is HAND-CODED in tray.rs — line 62 `MenuItem::with_id(app, "header", "Hallmark", false, ...)`, line 77 places it first in the items array, lines 151-153 handle the dead branch. NOT auto-injected by Tauri. UNDERLYING SPEC CONFLICT: 04-CONTEXT.md D-01 (lines 28-38) actually MANDATES the header — but 04-UAT test 2 expectation and the user's reading both say no header. Spec reconciliation needed before code change. Recommended decision: drop the header (matches user expectation + Discord/Slack/Steam conventions; tooltip already provides identification on hover).
    (2) tray.ico (and icon.ico — byte-identical) is a fully-transparent 6-frame placeholder ICO from Phase 1 commit 452d29b explicitly labeled 'replaced when real artwork lands'. ICO binary parse: 16x16/32x32/etc. all alpha=0 + AND mask all zeros → legacy GDI tray-icon host renders as solid 16×16 black square. icon.ico LOOKS fine in dev because dev mode uses Chromium default for window chrome; only tray.ico hits a real renderer in dev (via include_bytes!). Phase 04-07 was supposed to land real artwork but pivoted to SFX-only.
  artifacts:
    - "src-tauri/src/tray.rs:62 — hand-coded Hallmark header MenuItem"
    - "src-tauri/src/tray.rs:71 — sep1 separator (only meaningful with header)"
    - "src-tauri/src/tray.rs:77-78 — &header, &sep1 in items array"
    - "src-tauri/src/tray.rs:151-153 — dead 'header' arm in handle_menu_event"
    - "src-tauri/src/tray.rs:3-14 — doc comment lists header in menu spec"
    - "src-tauri/icons/tray.ico — fully-transparent 6-frame placeholder, all pixels alpha=0"
    - "src-tauri/icons/icon.ico — byte-identical placeholder (will render same in NSIS-installed binary)"
    - ".planning/phases/04-polish-distribution/04-CONTEXT.md:28-38 — D-01 spec contradiction with UAT test 2"
  missing:
    - "Spec reconciliation: pick 'no header' OR 'header' and converge 04-CONTEXT D-01 + tray.rs + 04-UAT test 2 expectation"
    - "Real multi-resolution ICO with non-zero alpha at 16x16 + 32x32 minimum (e.g., `magick convert glyph.png -define icon:auto-resize=256,128,64,48,32,24,16 tray.ico`) for BOTH tray.ico AND icon.ico"
  debug_session: .planning/debug/tray-menu-extra-header-and-black-icon.md

- truth: "Companion window header drag region covers all non-button header pixels and reliably grabs the window for dragging"
  status: diagnosed
  reason: "Drag works only in flex gaps; title text + badge pixels do not drag."
  severity: minor
  test: 3
  root_cause: |
    Tauri 2's `data-tauri-drag-region` is per-element (NOT inherited). In CompanionHeader.tsx, the attribute is only on the outer <header>; the title <div> (`flex: 1`, covers most of header width) and the badge <div> are siblings without the attribute, so clicks on them hit the child and never register as drag. The empty flex-gap pixels between siblings fall directly on the parent header — the only draggable surface. No CSS pointer-events override, no -webkit-app-region remnant; pure markup omission.
  artifacts:
    - "src/components/CompanionHeader.tsx:6 — title <div> missing data-tauri-drag-region"
    - "src/components/CompanionHeader.tsx:8 — badge <div> missing data-tauri-drag-region"
  missing:
    - "Add data-tauri-drag-region to companion title <div> and badge <div>"
  debug_session: .planning/debug/drag-region-undersized.md

- truth: "Test popup fires reliably on every tray click after the first one (POL-01)"
  status: diagnosed
  reason: "Repeat test_trigger::fire calls past dedup TTL produce no UNLOCK or POPUP_FIRED log line."
  severity: major
  test: 4
  root_cause: |
    SQLite UNIQUE INDEX `idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)` is a SECOND dedup layer beyond the in-memory 10s CrossSourceDedup. test_trigger::fire always synthesizes the same `(480, HALLMARK_TEST_UNLOCK)` pair; session_id is constant for the process lifetime (lib.rs:286-289). After in-memory dedup expires (10s TTL works correctly), the event reaches `record_unlock` which uses INSERT OR IGNORE → returns Ok(false) → run_pipeline silently `continue`s without logging UNLOCK and without forwarding to sink_tx. Drop is INVISIBLE because both silent-drop arms in run_pipeline use `tracing::debug!`, below the default `RUST_LOG=hallmark_lib=info,warn` filter. UNIQUE INDEX is correct for real achievements (re-firing same achievement in one session is duplicate worth suppressing) but wrong for synthetic test trigger that intentionally re-uses the key. Phase 04-03 threat model considered only the in-memory TTL (T-04-13) — DB UNIQUE INDEX layer was not factored in.
  artifacts:
    - "src-tauri/src/watcher/mod.rs:387-394 — run_pipeline `!inserted` branch logs at debug! and silently `continue`s"
    - "src-tauri/src/store/mod.rs:58-84 — record_unlock uses INSERT OR IGNORE, returns Ok(false) on collision"
    - "src-tauri/src/store/migrations/001_initial.sql:24-25 — defines idx_unlock_dedup UNIQUE on (app_id, ach_api_name, session_id)"
    - "src-tauri/src/test_trigger.rs:32-65 — fire() reuses the same (480, HALLMARK_TEST_UNLOCK) pair on every click"
    - "src-tauri/src/lib.rs:286-289 — session_id generated once at startup, constant for process lifetime"
  missing:
    - "Recommended (Option 1): timestamp-suffix the test trigger's ach_api_name in test_trigger::fire (e.g., format!(\"HALLMARK_TEST_UNLOCK_{}\", timestamp)) so each fire has unique key, no UNIQUE INDEX collision; popup_queue.rs:135-137 has display_name fallback so popup still renders 'Test Achievement' if schema fixture lookup adapts (or substitute display_name when api_name starts with 'HALLMARK_TEST_UNLOCK_')"
    - "Secondary observability fix (separate task): promote run_pipeline silent debug! drops to info! or add metrics — current behavior also silently swallows production scenarios (process restart re-emitting same achievement, cross-source race within 10s window)"
  debug_session: .planning/debug/popup-repeat-fire-stuck.md

- truth: "UI is interactive within ~1 second of cargo tauri dev launch — achievements that fire during early startup render visibly (not just SFX)"
  status: diagnosed
  reason: "WebView windows blank for ~20s after cargo tauri dev launch; SFX plays but popup never paints if fired during gap."
  severity: major
  test: 4
  root_cause: |
    Two compounding mechanisms:
    (A) Vite multi-entry cold-transform bottleneck (DEV-ONLY): vite.config.ts has 4 rollup inputs (companion/popup/settings/wizard) but NO `optimizeDeps.entries`. Vite's esbuild pre-bundle only auto-discovers from index.html (default entry). When WebView2 fetches /popup.html or /wizard.html, Vite must lazily transform main-popup.tsx / main-wizard.tsx + walk full import graph (React 19, Framer Motion, @tauri-apps/api/*, components, CSS) — cold round-trip 10-30+ seconds. Documented Tauri v2 + Vite dev behavior (issues #12742, #8920, #6045, #5170, #13017).
    (B) Missing WebView-ready handshake (PRESENT IN BOTH DEV AND PROD): popup_queue::process_event (popup_queue.rs:161-170) calls popup.show() → app.emit_to('popup', 'popup-show', payload) → audio.play() with NO await for frontend-mounted ack. Tauri events do NOT buffer for listeners that attach after emit — if process_event runs before main-popup.tsx's useEffect registers listen('popup-show'), event is silently dropped. Audio.play succeeds because rodio is independent of WebView state — this is exactly the 'SFX without popup' pattern. In prod the race window is ~500ms (mostly imperceptible); in dev mechanism (A) widens it to 20s+ which makes (B) reliably reproducible.
    Production confirmation: dist/assets/ contains pre-built bundles (popup-BGd0qpbs.js, wizard-CnNnpB7X.js, etc.); cargo tauri build skips Vite entirely. UAT test 19 confirmed prod build is healthy at 5.1 MB. Mechanism (A) does NOT reproduce in prod; mechanism (B) does (but is invisible to user at sub-second scale).
  artifacts:
    - "vite.config.ts — 4-entry rollup config without optimizeDeps.entries (mechanism A)"
    - "src-tauri/src/popup_queue.rs:161-170 — process_event fire-and-forget emit_to (mechanism B)"
    - "src-tauri/src/popup_queue.rs:222-227 — emit_celebration same fire-and-forget pattern"
    - "src/main-popup.tsx:11-19 — useEffect registers listen() but emits no ready-ack"
  missing:
    - "Mechanism A fix: add `optimizeDeps: { entries: ['index.html', 'popup.html', 'settings.html', 'wizard.html'], include: ['react', 'react-dom', 'react-dom/client', 'framer-motion', '@tauri-apps/api/core', '@tauri-apps/api/event'] }` to vite.config.ts — expected dev startup 20s → 2-5s"
    - "Mechanism B fix: WebView-ready handshake — frontend invokes a `popup_ready` Tauri command from useEffect after listen() registers; popup_queue::run blocks first emit on a tokio::sync::Notify populated by the command. Apply same pattern to wizard."
  debug_session: .planning/debug/webview-warmup-blank-screen.md

- truth: "App quits cleanly without Chromium teardown errors"
  status: deferred
  reason: "On Quit, Chromium emits `[ERROR:ui\\gfx\\win\\window_impl.cc:134] Failed to unregister class Chrome_WidgetWin_0. Error = 1412` — cosmetic stderr noise from WebView2 teardown ordering. App exits cleanly per Quit-drain logs."
  severity: cosmetic
  test: 4
  root_cause: "Not investigated — flagged as obvious WebView2/Chromium teardown noise (ERROR_CLASS_DOES_NOT_EXIST = 1412 emitted when Chromium tries to unregister the window class twice during shutdown). Known Tauri v2 + WebView2 stderr artifact unrelated to app correctness."
  artifacts:
    - "Cosmetic only — app exit semantics correct per Quit-drain logs"
  missing:
    - "Defer to v1.1 — not worth a fix slot in Phase 4 polish (cosmetic stderr only, no user-visible impact)"
  debug_session: not-investigated

- truth: "Settings window has fully styled premium dark surface — no OS background bleed, no light body background, scrollbar styled or hidden inside the rounded card"
  status: diagnosed
  reason: "Native OS scrollbar OUTSIDE rounded card; off-white body background bleeds through padding."
  severity: major
  test: 6
  root_cause: |
    Single missing CSS reset is the upstream cause of all 5 sub-symptoms (this gap + skeleton mismatch + layout density + wizard premium-feel regression + non-sticky wizard header). companion.css (lines 2-6) and popup.css (lines 3-8) both ship a global `html, body, #root` reset; settings.css does NOT. Without it: (1) body has UA-default white bg + 8px margin → off-white bleed around the rounded card; (2) `.settings-shell` and `.wizard-shell` use `min-height: 100vh` instead of `height: 100%`, so inner `.settings-body { flex: 1; overflow-y: auto }` never gets a bounded height — overflow propagates to body, producing the OS scrollbar at the WINDOW edge (outside the rounded card); (3) wizard header is a flex child of the broken scroll context and scrolls along with body; (4) `.skeleton-line` is 36px no padding while `.source-row` has min-height 36px + 8px padding → ~37-52px effective → row-height jump on rescan resolve; (5) `.settings-body` 32px section gap + body-level scroll bug pushes content below the fold. UI-SPEC.md line 17 explicitly required Phase 4 surfaces to inherit Phase 2's design language — this regression breaches that contract.
  artifacts:
    - "src/styles/settings.css — entire file is the locus of the fix (no html/body/#root reset; min-height on shells; skeleton-line dim mismatch; oversized section gap)"
    - "src/styles/companion.css:2-8 — REFERENCE PATTERN to mirror"
    - "src/styles/popup.css:3-8 — secondary reference confirming the same pattern"
  missing:
    - "Add to top of settings.css: `html, body { margin: 0; padding: 0; height: 100%; background: #111114; color: #F0F0F5; overflow: hidden; font-family: 'Inter', 'Segoe UI Variable', 'Segoe UI', -apple-system, BlinkMacSystemFont, system-ui, sans-serif; } #root { width: 100vw; height: 100vh; }`"
    - "Change `.settings-shell` and `.wizard-shell` from `min-height: 100vh` to `height: 100%`"
    - "Add `::-webkit-scrollbar` styling on `.settings-body` and `.wizard-body` (8px, dark thumb rgba(255,255,255,0.10)) for premium feel"
    - "Change `.skeleton-line { height: 36px; ... }` to `min-height: 36px; padding: 8px; box-sizing: border-box; border-radius: 8px;` (mirror .source-row)"
    - "Tighten `.settings-body` section gap 32px → 24px (UI-SPEC 'lg' token)"
    - "Optional: `.wizard-header, .settings-header { position: sticky; top: 0; z-index: 1; }` (backgrounds already #111114)"
  debug_session: .planning/debug/settings-wizard-css-surface-regression.md

- truth: "View on GitHub link in Settings About panel opens https://github.com/ReemX/hallmark in the default browser"
  status: diagnosed
  reason: "Clicking 'View on GitHub' does nothing — Tauri WebView2 silently blocks default browser navigation."
  severity: major
  test: 6
  root_cause: |
    All 5 wiring steps for tauri-plugin-shell are absent. This is a net-new feature, not a regression — the plugin has never been installed. BONUS: src/components/UpdateModal.tsx:65-72 has a SECOND dead link with the identical broken pattern ('Read full release notes on GitHub' → tag URL); single fix covers both call sites.
  artifacts:
    - "src/Settings.tsx:205-212 — <a href target='_blank'> with no onClick handler"
    - "src/components/UpdateModal.tsx:65-72 — same broken pattern (second dead link, same fix)"
    - "package.json — missing @tauri-apps/plugin-shell"
    - "src-tauri/Cargo.toml — missing tauri-plugin-shell"
    - "src-tauri/src/lib.rs — Builder doesn't register tauri_plugin_shell::init()"
    - "src-tauri/capabilities/settings.json — missing shell:allow-open with URL allowlist"
  missing:
    - "Cargo.toml: add `tauri-plugin-shell = \"2\"`"
    - "package.json: add `\"@tauri-apps/plugin-shell\": \"^2\"`"
    - "lib.rs Builder: `.plugin(tauri_plugin_shell::init())`"
    - "settings.json capability: `shell:allow-open` scoped to https://github.com/ReemX/hallmark + https://github.com/ReemX/hallmark/releases/tag/* (least-privilege URL allowlist — UpdateModal renders inside Settings window, so settings.json covers both call sites)"
    - "Frontend: import { open } from '@tauri-apps/plugin-shell'; replace bare <a> with `<a onClick={(e) => { e.preventDefault(); open(url).catch(() => {}); }}>` in Settings.tsx + UpdateModal.tsx — keep href for right-click 'Copy link' UX"
  debug_session: .planning/debug/github-link-dead.md

- truth: "Settings header drag region covers the full 48px header strip including the 'Settings' title text"
  status: diagnosed
  reason: "Title text 'Settings' not draggable; chrome around it is."
  severity: minor
  test: 6
  root_cause: "Same root cause as companion drag-region (gap test 3): `data-tauri-drag-region` is per-element, NOT inherited. Settings.tsx:119-128 has the attribute on `<div className=\"settings-header\">` but the inner `<span className=\"settings-title\">` (line 120) lacks it. With justify-content: space-between, the only directly-hit parent pixels are the gap between title and close button. Title <span> intercepts pointer events without inheriting drag."
  artifacts:
    - "src/Settings.tsx:120 — settings-title <span> missing data-tauri-drag-region"
  missing:
    - "Add data-tauri-drag-region to the settings-title <span> (close <button> remains auto-excluded by Tauri's interactive-element rule). Total fix across both drag-region gaps: 3 attribute additions in 2 files (CompanionHeader.tsx title + badge, Settings.tsx title)"
  debug_session: .planning/debug/drag-region-undersized.md

- truth: "Settings layout uses horizontal space efficiently — content fits without vertical scrolling on the fixed 520x580 window"
  status: diagnosed
  reason: "Content overflows 520×580 viewport, forcing vertical scroll to reach About."
  severity: minor
  test: 6
  root_cause: "Combined cause: (a) the broken body-level scroll bug (see CSS-surface gap above) steals visible horizontal real estate by spawning the OS scrollbar at window edge; (b) `.settings-body` uses 32px section gap (one tier too large for the 580px height with 3 sections + headers + skeleton state). Both are addressed by the single CSS-surface fix."
  artifacts:
    - "src/styles/settings.css — .settings-body gap: 32px → 24px (UI-SPEC 'lg' token)"
  missing:
    - "Covered by the CSS-surface fix (skeleton rebuilt, scroll containment, section gap tightened in same patch). No separate work item."
  debug_session: .planning/debug/settings-wizard-css-surface-regression.md

- truth: "Skeleton placeholder rows in Detected Sources panel match the height of rendered SettingsSourceRow entries — no layout jump when scan completes"
  status: diagnosed
  reason: "~50ms skeleton flash + visible vertical jump on rescan resolve."
  severity: minor
  test: 7
  root_cause: "`.skeleton-line { height: 36px }` (no padding/box-sizing) vs `.source-row { min-height: 36px; padding: 8px }` (~37-52px effective). Pure CSS dim mismatch. Covered by the CSS-surface fix above."
  artifacts:
    - "src/styles/settings.css — .skeleton-line dimensions need to mirror .source-row exactly"
  missing:
    - "Covered by CSS-surface fix: change .skeleton-line to `min-height: 36px; padding: 8px; box-sizing: border-box; border-radius: 8px;` (mirror .source-row). No separate work item."
  debug_session: .planning/debug/settings-wizard-css-surface-regression.md

- truth: "Updates panel error message accurately describes the failure cause — distinguishes 404/no-release from genuine network failure"
  status: diagnosed
  reason: "404 (no release published) shows 'check your connection' copy meant for offline."
  severity: minor
  test: 9
  root_cause: |
    Two-place bug. (1) tauri-plugin-updater 2.10 ALREADY distinguishes the two cases at the source level: `Error::ReleaseNotFound` for non-2xx HTTP status (the 404 path — confirmed by the WARN log line `error=Could not fetch a valid release JSON from the remote` which is the Display impl of that variant) vs `Error::Reqwest(reqwest::Error)` for transport failure (DNS/TCP/TLS/timeout). Other relevant variants: EmptyEndpoints, TargetNotFound, TargetsNotFound, Serialization, InsecureTransportProtocol. (2) src-tauri/src/updater_glue.rs:61 discards that structure: `updater.check().await.map_err(|e| e.to_string())?`. Then src/Settings.tsx:180-182 hardcodes a literal offline-themed string and never reads UpdateState.error.message anyway — even if backend preserved the message, frontend would still show 'check your connection.'
  artifacts:
    - "src-tauri/src/updater_glue.rs:59-75 — manual_check flattens Error enum to String via e.to_string()"
    - "src-tauri/src/lib.rs:50-54, 164-169 — UpdateInfoView only carries success metadata; manual_check_update returns Result<Option<UpdateInfoView>, String>"
    - "src/Settings.tsx:54-59 — UpdateState union collapses all errors into one variant"
    - "src/Settings.tsx:180-182 — hardcoded 'Couldn't reach the update server' copy"
  missing:
    - "Backend (~30 lines Rust): add tagged enum CheckOutcome { Available { version, notes }, UpToDate, NoReleaseYet, Offline { detail }, PlatformMissing { detail }, OtherError { detail } } with serde(tag='status', rename_all='snake_case'). In updater_glue::manual_check, replace `.map_err(|e| e.to_string())?` with a `match` on tauri_plugin_updater::Error variants. For NoReleaseYet, still call persist_last_checked (treat as successful 'checked' event for UX freshness)."
    - "Frontend (~25 lines TSX): extend UpdateState with kind: 'no_release' | 'offline' | 'platform_missing' | 'other_error'. Switch handleCheckUpdates to read result.status. Render copy per kind: no_release → 'No releases yet — Hallmark is on its first version. We will show new versions here when they arrive.' (also persist 'Last checked: just now'); offline → keep current copy; platform_missing → 'An update was found but does not support your platform.'; other_error → `Update check failed: ${detail}` (use the message)."
    - "Apply same Error mapping inside spawn_background_check so logs differentiate info!('no release published yet') from warn!('update check failed: offline')"
  debug_session: .planning/debug/updates-error-wording-misleading.md

- truth: "Wizard window has fully styled premium dark surface with confined scrollbar — same fix as Settings (no body bg bleed, no OS scrollbar outside card, sticky header)"
  status: diagnosed
  reason: "Same scrollbar/bg/non-sticky-header issues as Settings."
  severity: major
  test: 14
  root_cause: "Same root cause as the Settings CSS-surface gap (test 6): missing html/body/#root reset in settings.css, min-height-instead-of-height on shells, no scrollbar styling, no sticky header rule. Wizard imports the same settings.css so a single coordinated patch covers both windows."
  artifacts:
    - "src/styles/settings.css — .wizard-shell + .wizard-header receive the same fix as .settings-shell + .settings-header"
  missing:
    - "Covered by the CSS-surface fix above. No separate work item — single PR addresses both windows."
  debug_session: .planning/debug/settings-wizard-css-surface-regression.md

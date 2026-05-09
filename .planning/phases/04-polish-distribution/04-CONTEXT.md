# Phase 4: Polish & Distribution - Context

**Gathered:** 2026-05-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Make Hallmark publicly installable and self-verifiable. Every requirement is downstream of "a stranger lands on the GitHub release and gets a working install in under 5 minutes."

1. **Polish triggers** (POL-01, POL-02) — Tray icon + menu (currently nonexistent in `src-tauri`). Test-popup trigger that fires a synthetic unlock through the production pipeline. Start-with-Windows toggle via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
2. **Settings window** — Borderless rounded card matching companion's design language. Read-only "Detected sources" panel + manual "Check for updates" + About. No knobs (signature style locked per PROJECT.md).
3. **First-run wizard** (DIST-04) — Standalone borderless welcome window on first launch. Shows path-discovery results with explicit source names. Once-ever flag in SQLite, with re-trigger if 0 paths still detected on subsequent launches.
4. **NSIS installer + portable .zip** (DIST-01) — Per-user install to `%LOCALAPPDATA%\Hallmark` (no UAC). Self-contained portable .zip writes state to `%APPDATA%`. Both produced per release.
5. **Auto-updater** (DIST-02) — `tauri-plugin-updater` 2.10.1 wired to GitHub Releases `latest.json`. Stable channel only for v1. Modal prompt on next companion-window open; on Install, immediate Hallmark process restart (PC unaffected).
6. **GitHub Actions release pipeline** (DIST-03) — `tauri-action` triggered on tag push. Builds NSIS + portable, signs `latest.json` with Ed25519 keypair stored exclusively as `TAURI_SIGNING_PRIVATE_KEY` GitHub secret (no local copy of private key).
7. **Signature SFX final assets** — Carry-forward from Phase 2 deferred. Replace `gen_sfx`-generated synth placeholders for D-05 standard / D-06 rare / D-12 100% celebration with locked v1 sound. **Approach is a research flag — not locked here.**

The Phase 1–3 detection pipeline is locked. Phase 4 does NOT modify watcher core, source adapters, popup-queue, schema cache, audio dispatcher, or the existing window builders for popup/companion. Phase 4 ADDS: tray icon, settings window, first-run wizard window, and the build/release/update plumbing.

</domain>

<decisions>
## Implementation Decisions

### Tray icon and menu (POL-01, POL-02 surface)
- **D-01 Tray menu structure (locked, amended 2026-05-09 — see SUPERSEDED note):**
  ```
  Show companion
  Fire test popup
  ─────────────
  Settings…
  ☑ Start with Windows
  ─────────────
  Quit
  ```
  Inline checkable "Start with Windows" item. "Settings…" opens a separate Settings window. Tray is the primary surface for both POL-01 + POL-02.

  **[SUPERSEDED 2026-05-09 — gap closure 04-13a]** Original D-01 included a
  non-clickable "Hallmark" header item at the top with a separator below
  it. Phase 4 UAT test 2 (2026-05-09) flagged the header as inconsistent
  with the Discord/Slack/Steam tray-utility convention (none of those
  ship a header item). User picked the no-header layout above; the
  `tooltip("Hallmark")` on the tray icon already provides app
  identification on hover. tray.rs implements the amended layout; the
  diagnosis ([.planning/debug/tray-menu-extra-header-and-black-icon.md](../../debug/tray-menu-extra-header-and-black-icon.md))
  records the spec contradiction and the resolution.
- **D-02 Tray icon presence:** Always-on-top tray icon (system notification area) with Hallmark monochrome glyph. Right-click → menu above. Left-click on icon = same as "Show companion" menu item.
- **D-03 Quit semantics:** `Quit` cleanly closes all windows, drains popup queue with timeout, joins tokio tasks, releases watcher handles. The X button on companion does NOT quit — it hides (existing Phase 2 behavior). Quit is tray-only.

### Test popup trigger (POL-01)
- **D-04 Inject point — Claude's discretion call:** Synthesize a `RawUnlockEvent` and emit it at the **adapter→dedup boundary** (the same `mpsc::Sender<RawUnlockEvent>` that all `SourceAdapter` implementations feed in `run_pipeline`). Hits real `CrossSourceDedup`, real `SchemaCache::resolve`, real `AudioDispatcher`, real popup-queue, real monitor placement. Does NOT touch the file watcher itself. Rationale: ROADMAP SC#1 says "fires through the full pipeline" — verifying the entire chain except for the kernel-level file watcher is the maximum a self-test can validate; touching real files would be slow + path-fragile and is what actual game unlocks already validate.
- **D-05 Test fixture data:** Hardcoded sample app_id (e.g. 480 = "Spacewar" Steam test app, or a Hallmark-reserved sentinel like 0x48414C4D in the sample byte range) + sample ach_api_name ("HALLMARK_TEST_UNLOCK") + bundled placeholder icon + canned title/description. Schema lookup short-circuits to the bundled fixture for this api_name (does NOT round-trip Web API).
- **D-06 Test trigger one-shot semantics:** Each click of "Fire test popup" produces exactly one popup (subject to dedup TTL — if user double-clicks within `dedup_ttl`, second is suppressed, which is correct production behavior). No throttle UI.

### Start-with-Windows (POL-02)
- **D-07 Registry pattern (locked):** Writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark` = `"<install-path>\hallmark.exe" --silent`. Toggle off = removes the named value cleanly (does not delete the Run key itself). Per-user only — never `HKLM`.
- **D-08 `--silent` startup behavior:** When launched with `--silent`, Hallmark starts in tray-only mode. Companion does NOT auto-open. File watcher + game-detection + popup-queue tasks all initialize normally. User opens companion via tray click. Matches Discord / Slack / Steam tray-utility convention.
- **D-09 Tray menu item state sync:** The "☑ Start with Windows" tray menu item reflects the live registry state (read on tray menu open). Toggle = write/delete registry value + update menu state. No SQLite shadow flag — registry is the source of truth.

### Settings window (UI surface)
- **D-10 Window form factor:** Borderless rounded card, decorations: false, fixed size ~520 × 580, centered on tray-icon's monitor on open. Matches companion's visual language. Custom drag region + custom close button.
- **D-11 Settings panels (locked, in order):**
  1. **Detected sources** — read-only list with explicit names (`Steam`, `Goldberg`, `CreamAPI`, `SmartSteamEmu`); ✓ for found, ✗ for not found; "Rescan" button re-runs `discover_paths` and updates the list. NO emulator setup help (passive-detection rule from PROJECT.md).
  2. **Updates** — current version + "Check for updates" button. Clicking checks GitHub `latest.json` and shows result inline.
  3. **About** — version string, GitHub repo link, license (MIT or chosen OSS license — researcher to confirm SPDX in license file), credits.
- **D-12 NOT in v1 settings:** start-with-Windows toggle (tray-only per D-01), diagnostic log viewer (deferred), theme/sound knobs (out of scope per PROJECT.md "signature style locked"), update channel selector (stable only — D-19), companion size/position reset, auto-update on/off toggle.

### First-run wizard (DIST-04)
- **D-13 Surface:** Standalone borderless rounded-card window, separate from companion. Same visual language as companion + Settings. Closes on dismissal; companion takes over after.
- **D-14 Trigger lifecycle:** SQLite `settings` table flag `first_run_done` (boolean). Wizard fires when flag is unset. Set flag on dismissal **only if at least 1 path was detected**. If 0 paths on dismiss, flag stays unset → wizard re-fires next launch. Once any path is detected (now or future scan), the dismissal latches the flag permanently.
- **D-15 Wizard contents on N>0 paths:** "Welcome to Hallmark" header + "We found these achievement sources on your system:" + explicit list (`Steam`, `Goldberg`, `CreamAPI`, `SmartSteamEmu`) with ✓ for present. CTA: "Get started" button → closes wizard, opens companion.
- **D-16 Wizard contents on N=0 paths:** Header acknowledges no sources found. Body lists what was scanned (e.g., "Steam libraryfolders.vdf — not found", "Goldberg saves directory — not found", per-source). One-liner explainer: "Hallmark watches these locations — install Steam or play a game with achievements to populate them." Buttons: "Rescan" + "Continue anyway" (closes wizard with flag NOT latched). NO emulator setup instructions.
- **D-17 Re-runnable equivalent:** The Settings → Detected sources panel offers the same "Rescan" capability. The standalone wizard window is first-launch-only; subsequent re-checks happen inside Settings.

### Auto-updater (DIST-02)
- **D-18 Update prompt UX (locked):** On Hallmark launch, background-check `latest.json`. If newer version available, show modal sheet over companion the next time companion opens (does NOT fire a notification while game is playing — companion is hidden then). Modal: release notes (truncated, "Read more on GitHub" link) + "Install" / "Later" buttons. "Later" snoozes for the rest of the session; reappears next launch.
- **D-19 Channel:** Stable only for v1. tauri-plugin-updater reads single `latest.json`. GitHub Releases marked "prerelease" are NOT picked up. Prerelease channel deferred to v2 contingent on community traction (per user: "we will never have pre-release for this scope unless project starts gaining open-source traction").
- **D-20 Install flow (locked):** "Install" → tauri-plugin-updater downloads payload → calls `app.restart()` (Hallmark process only — never the OS). Brief tray/companion blip. New version active. Rationale: ~1-min update, user explicitly triggered it, no reason to defer.
- **D-21 Updater signing keypair (locked):** Generate Ed25519 keypair via `tauri signer generate` ONCE during initial repo setup. Private key pasted directly into `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if password-protected. Local copy DELETED immediately after upload. Public key committed to `tauri.conf.json` `bundle.windows.updater.pubkey`. CI is the only entity that can sign a release. Researcher must validate the exact `tauri signer` flow + tauri-action env-var contract before this is locked in code (RESEARCH FLAG #1 below).

### NSIS installer + portable .zip (DIST-01)
- **D-22 Installer scope (locked):** Per-user install. NSIS `RequestExecutionLevel user`. Default install dir = `%LOCALAPPDATA%\Hallmark`. No UAC prompt. Matches Discord/Slack pattern. Updater can write to install dir without admin.
- **D-23 Portable .zip behavior (locked):** Self-contained. Extracts to any folder. Runs from any path. State (SQLite DB, settings, schema cache) lives at `%APPDATA%\com.hallmark.app\` — same as installed mode. Updater is **disabled in portable mode** (no installed location to update). Detection: at startup, check whether the executable is running from a known install location vs. arbitrary; if arbitrary, set `portable_mode = true` and skip the updater wiring.
- **D-24 Code signing (locked):** Unsigned for v1. SmartScreen "Windows protected your PC" warning on first install is documented in README with a screenshot + "Click 'More info' → 'Run anyway'" instructions. CI workflow includes commented-out signtool placeholders so a future contributor with a code-signing cert can wire it in without restructuring. Common for hobby OSS — Notepad++, OBS-launcher, etc, shipped unsigned for years.

### GitHub Actions release pipeline (DIST-03)
- **D-25 Trigger:** Tag push matching `v*.*.*` → `tauri-action` workflow runs. Builds NSIS + portable .zip on `windows-latest`, signs `latest.json` with `TAURI_SIGNING_PRIVATE_KEY`, uploads all artifacts to the GitHub Release matching the tag. `latest.json` is committed/uploaded to the same release for `tauri-plugin-updater` to fetch.
- **D-26 Manual workflow_dispatch fallback:** Workflow is also `workflow_dispatch`-triggerable for emergency reruns without a new tag. Researcher to validate `tauri-action` v0 input schema for the latest stable version.
- **D-27 Pre-release tags ignored by updater:** Releases marked `prerelease: true` on GitHub do NOT have their `latest.json` published as the stable feed. Aligns with D-19.

### Signature SFX final assets (RESEARCH FLAG)
- **D-28 RESEARCH FLAG, NOT LOCKED:** User wants "the most polished feel possible without an outside contract designer". Phase research must investigate the best path to PS5/console-grade SFX given OSS-distribution constraints. Preference order:
  1. **Procedural** — refine `gen_sfx.exe` parameters or pre-bake from a richer synthesis tool (SuperCollider, Tone.js export, Faust). Zero licensing risk.
  2. **CC0 / public-domain royalty-free pack curation** — freesound.org with CC0 filter, Pixabay, Mixkit; layer in DAW or via the gen_sfx mixer. License terms must permit OSS-redistribution (most royalty-free packs do NOT — must verify per-asset).
  3. **Never-rip from copyrighted sources** — explicitly OUT for v1+. Hard boundary: bundling copyrighted SFX (PS5/Xbox/etc) into the public OSS GitHub release exposes the project to DMCA strikes and contributor legal risk. User initially open to "ripping" was redirected on this constraint.
- **D-29 Asset format + bundle:** WAV or OGG, 44.1kHz / 16-bit, mono or stereo (rodio handles both). Bundled at `assets/sfx/standard.{ext}`, `rare.{ext}`, `celebration.{ext}` — same names Phase 2 already references. Replacement is a drop-in; no code changes in `audio.rs` if file paths/keys are preserved.

### Claude's Discretion
- Exact tray icon glyph design — researcher / Claude to design within Hallmark's monochrome dark/light theme.
- Whether the "Updates" panel in Settings shows last-checked timestamp — minor UX polish.
- First-run wizard exact copy — "Welcome to Hallmark" / source-list framing — Claude writes within the established voice (concise, premium, non-promotional).
- About panel exact links + license SPDX — Claude derives from existing repo state.
- NSIS installer wizard pages (welcome, install dir, install button, finish) — standard tauri-action defaults unless friction surfaces in testing.
- Whether portable-mode detection uses "exe parent folder writable" heuristic vs. an explicit `--portable` flag — researcher's call after testing tauri-action's portable bundle output.
- Whether `Fire test popup` short-circuits a Steam Web API rarity lookup or uses cached/zero rarity — Claude/planner picks based on responsiveness vs. fidelity.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level scope and stack
- `.planning/PROJECT.md` — Out-of-scope explicitly excludes telemetry/analytics/crash-reporting; D-28 SFX redistribution constraint flows from "Free, open-source on GitHub" + "Customization: Signature style locked".
- `.planning/REQUIREMENTS.md` — POL-01, POL-02, DIST-01, DIST-02, DIST-03, DIST-04 (all 6 active items in Phase 4 scope). Validate exact wording before locking down test-popup mechanics.
- `.planning/ROADMAP.md` — Phase 4 entry: 5 success criteria are the locked acceptance bar. SC#1 explicitly says "fires through the full pipeline" — D-04 chooses the most-honest interpretation.
- `CLAUDE.md` — Stack pins (Tauri 2.11, tauri-plugin-updater 2.10.1, Inno Setup 6.x / NSIS as alternatives, windows-rs 0.58 for HKCU registry writes). "What NOT to Use" section: velopack excluded (pre-release), MSIX excluded (Store-only friction). Signature style locked language flows into D-12.

### Phase 1–3 contracts to consume (DO NOT modify)
- `src-tauri/src/lib.rs` — Tauri builder + `setup()` extension point. Phase 4 attaches the tray icon, registers the test-popup invoke handler, and wires the updater plugin here.
- `src-tauri/src/sources/mod.rs` — `RawUnlockEvent` struct shape consumed by D-04 test injector.
- `src-tauri/src/watcher/mod.rs` — `run_pipeline` adapter→dedup mpsc boundary is the D-04 inject point. The sender side is owned by `run_pipeline`; Phase 4 needs a clone of this sender (or an adjacent test-emit channel that fans into the same dedup) — planner's call.
- `src-tauri/src/schema/` — `SchemaCache` is consulted by the popup queue. D-05 test-fixture short-circuit happens here.
- `src-tauri/src/popup_queue.rs` — Receives the resolved popup payload from schema → audio → ui chain. Test popup arrives here naturally via D-04 path.
- `src-tauri/src/store/migrations/` — Phase 4 adds `003_*.sql` introducing the `first_run_done` flag (D-14) — alternatively a row in existing `settings` table (planner's call).
- `src-tauri/tauri.conf.json` — `bundle.active: false` flips to `true` for Phase 4. `bundle.windows.nsis` config + `bundle.windows.updater.pubkey` added. CSP already permits Steam image hosts (Phase 2 work) — no changes needed for updater outbound (researcher confirms `connect-src` allows GitHub Releases).
- `src-tauri/Cargo.toml` — Phase 4 adds `tauri-plugin-updater = "2.10"`, `windows-registry` (or use existing `windows = "0.58"` features for registry APIs — researcher's call), and a `tauri-plugin-tray-icon` (or whatever the current Tauri 2.x tray API is — confirm in research; tray was moved into `tauri::tray` module in 2.x).
- `src/` (frontend) — Phase 4 adds Settings React page + first-run wizard React page. Existing companion components/CSS provide the design language to match.
- `package.json` — Phase 4 adds no new core deps unless the React Settings page needs a new icon library — Claude/planner's call.

### Phase 1–3 prior context (already shipped, locked)
- `.planning/phases/02-premium-ui-popup-companion-game-session/02-CONTEXT.md` — Decisions D-13 (companion borderless rounded card) and D-14 (companion 480×720) define the design language D-10 + D-13 (Settings + first-run wizard) inherit. D-21 (Steam-state-authoritative leg deferred to Phase 3) is no longer deferred. D-28 outbound network policy authorizes the updater check.
- `.planning/phases/03-remaining-source-adapters/03-VERIFICATION.md` — Cross-source dedup verified end-to-end. Phase 4 test-popup trigger relies on this dedup being correct (D-06 dedup-TTL throttle is a feature, not a bug).
- `.planning/phases/03-remaining-source-adapters/empirical-vdf-NOTES.md` — Reference if future Settings panels need to surface Steam-userid detection.

### External research targets (for gsd-phase-researcher)
- Tauri 2.x tray icon API surface — `tauri::tray::TrayIconBuilder` + `MenuBuilder` + the `MenuItem` checkable variants. Verify exact module paths in v2.11.1 (Tauri's tray API stabilized in 2.x but module names shifted across pre-release).
- `tauri-plugin-updater` 2.10.1 wiring — `tauri::plugin::Builder` integration, `app.updater().check().await` API, `latest.json` schema, dynamic vs. static endpoint config.
- `tauri signer generate` CLI flow — file outputs, password protection, env var contract with tauri-action.
- `tauri-action` v0 GitHub Action — input schema for tag-triggered builds, artifact upload, `latest.json` publication, signing-key env var name (TAURI_SIGNING_PRIVATE_KEY vs. TAURI_PRIVATE_KEY).
- NSIS installer Tauri config — `bundle.windows.nsis` schema. `installMode: "currentUser"`, `displayLanguageSelector`, `languages`, custom branding image. Verify per-user-no-UAC works as documented in 2.11.
- Portable .zip emission — does `tauri-action` produce a portable .zip natively, or is it a custom post-step? Research empirically.
- Procedural SFX synthesis state-of-the-art for "premium UI ding" — SuperCollider + Faust + Tone.js comparison; CC0 sample-pack inventory (freesound CC0 search, Pixabay terms).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`src-tauri/src/lib.rs::run()` setup hook** — The same insertion point Phase 2 used to attach popup-queue + game-detection + companion-state tasks. Phase 4 attaches: tray icon registration, updater plugin registration, test-popup invoke handler, registry-write/read commands.
- **`SqliteStore::with_conn`** — extension API still preferred for the new `first_run_done` flag (D-14). One row in `settings`, no migration controversy.
- **`SchemaCache` short-circuit point** — D-05 test fixture lookup hooks here so the test popup gets a real schema payload without round-tripping the Web API.
- **`tracing::info!` / `warn!`** — already wired everywhere. Tray menu actions, updater check results, registry writes, and first-run wizard scan results all log via tracing for free.
- **`AudioDispatcher` (audio.rs)** — Phase 4's SFX-replacement work is purely an asset swap; the dispatcher API is stable. D-29 keeps the same file paths/keys.
- **`gen_sfx.exe` script** (root of repo) — Phase 2 placeholder synthesis tool. D-28 option 1 retunes its parameters; the binary stays in the repo as a build-time asset generator OR the final WAVs are committed and gen_sfx is removed (planner's call).
- **`discover_paths` (paths.rs)** — Already returns the structured list of detected sources (Phase 1 + Phase 3 wiring). DIST-04 wizard + Settings → Detected sources both call this directly.

### Established Patterns
- **One central event-loop per concern, fan-out via mpsc** — Phase 1+2 pattern. Phase 4 tray icon menu actions use Tauri's `MenuEvent` listener which is event-loop-friendly; updater plugin registers its own background task.
- **Sync→async bridges via `blocking_send` / `tokio::spawn_blocking`** — registry reads/writes are sync (Win32 API), so wrap in `spawn_blocking` from the tokio context. Same pattern as `run_watcher`'s notify bridge.
- **`Arc<Mutex<...>>` for shared mutable state** — Settings panel state shared between Tauri commands and the tray-menu sync logic uses this pattern.
- **`pub mod` ladder in `lib.rs`** — Phase 4 adds: `pub mod tray`, `pub mod settings_window`, `pub mod first_run`, `pub mod autostart` (registry HKCU helpers), `pub mod test_trigger`, `pub mod updater_glue` (if any custom logic beyond plugin registration is needed).
- **Tauri commands for frontend↔backend** — `commands::AppState` (Phase 2 D-13 onwards) is extended with handles needed by Settings (path discovery rerun, updater check trigger) and first-run wizard (initial scan results).
- **Borderless rounded card window builder** — Phase 2 created the popup + companion using `WebviewWindowBuilder` with `decorations: false` + custom drag region. Settings + first-run wizard reuse this builder shape.

### Integration Points
- **`src-tauri/src/lib.rs::run()` — `setup()` closure** — single insertion point for tray icon registration, updater plugin, registry helpers, and the new windows.
- **`src-tauri/src/watcher/mod.rs::run_pipeline`** — D-04 test-trigger inject point. Implementation needs a sender clone or an adjacent test-emit channel that fans into the same dedup pipe. Planner: confirm the cleanest seam — the existing adapter-side `mpsc::Sender<RawUnlockEvent>` is the natural choice.
- **`src-tauri/tauri.conf.json`** — flips `bundle.active: true`, adds `bundle.windows.nsis`, `bundle.windows.webviewInstallMode`, `bundle.windows.updater.pubkey`. Adds a `plugins.updater.endpoints` entry pointing at `https://github.com/<user>/<repo>/releases/latest/download/latest.json`.
- **`src/`** — frontend gets two new pages (`Settings.tsx` + `FirstRunWizard.tsx`) plus shared CSS for the rounded-card chrome.
- **`assets/sfx/`** — D-29 swap target. Phase 4 either retunes gen_sfx and re-bakes, or commits final WAVs and retires gen_sfx.
- **`.github/workflows/release.yml`** — new file. tauri-action v0 + tag trigger + artifact upload + latest.json signing. Researcher provides the canonical YAML.
- **`README.md`** — Phase 4 updates: install instructions (NSIS link, portable .zip link), SmartScreen warning + workaround (D-24), screenshots, update-flow explanation.

</code_context>

<specifics>
## Specific Ideas

- **Tray menu structure (D-01)** — User picked the explicit preview shape; that exact ASCII layout is the locked reference for the implementation. Inline checkable Start-with-Windows is critical for Discord/Slack-grade convenience.
- **First-run wizard explicit source labels (D-15)** — User picked the most-honest framing: name `Steam`, `Goldberg`, `CreamAPI`, `SmartSteamEmu` outright. Naming != setup help (per PROJECT.md passive-detection rule).
- **Update-prompt triggered by companion-open (D-18)** — Companion opens at game launch, which is high-engagement context. Critical detail: never fires while game is playing because companion is hidden during play.
- **`--silent` autostart pattern (D-08)** — Modeled after Discord / Slack / Steam tray-utility convention. Companion auto-open on autostart would be intrusive.
- **PC restart vs. process restart clarification (D-20)** — User asked the clarifying question; locked answer is "Hallmark process only — never the PC". Documentation must use precise language ("Restart Hallmark", never just "Restart").
- **SFX never-rip-copyrighted boundary (D-28)** — User originally open to "ripping a sound" — explicitly redirected because OSS-redistribution + DMCA strike risk is a hard project boundary, not a stylistic preference. Documented so future contributors (or future user-self) inherit the constraint.

</specifics>

<deferred>
## Deferred Ideas

- **Diagnostic log viewer in Settings** — scrollable in-memory ring buffer of recent tracing logs for users to copy/paste into GitHub issues. Useful for triage; deferred to v1.1 or after first wave of bug reports identifies need.
- **Update-channel selector (stable / prerelease toggle)** — D-19 defers prerelease channel until project gains community traction. When that happens, surface a toggle in Settings + ship `latest-beta.json` from CI.
- **Auto-update on/off toggle** — Power users may want to disable auto-update entirely. Deferred to v1.1 after seeing whether the modal-on-companion-open flow is friction-free.
- **Code signing** — D-24 ships unsigned for v1 with placeholders in CI for a future contributor to wire in a cert. Cert acquisition deferred indefinitely (cost vs. hobby-OSS pace).
- **Truly-portable mode (state-beside-exe)** — D-23 ships standard portable (state in `%APPDATA%`). Truly-portable (state in exe folder, USB-friendly) deferred to power-user request.
- **Telemetry / crash reporting** — explicitly out of scope per PROJECT.md "local-only" stance, not deferred for later — IT IS NOT COMING.
- **Companion size/position reset button in Settings** — minor UX. Deferred to first user request.

</deferred>

---

*Phase: 4-Polish & Distribution*
*Context gathered: 2026-05-09*

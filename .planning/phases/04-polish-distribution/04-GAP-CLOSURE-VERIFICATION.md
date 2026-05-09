---
phase: 04-polish-distribution
verified: 2026-05-10T00:00:00Z
verification_kind: gap-closure
status: passed
score: 10/10 UAT root causes closed in code
re_verification:
  previous_status: human_needed
  previous_score: 5/5 must-haves verified (initial code-level)
  uat_findings_addressed: 10
  uat_findings_remaining: 0
  regressions: []
overrides_applied: 0
human_verification:
  - test: "cargo tauri dev — confirm trophy tray icon renders (not solid black, not transparent)"
    expected: "Tray icon shows white-on-transparent trophy silhouette at 16x16 in Windows 11 system tray"
    why_human: "Visual asset rendering can only be confirmed by inspecting the running tray. Alpha-channel and frame-count verified programmatically (157/256 non-zero alpha at 16x16; 7 frames present)"
  - test: "cargo tauri dev — confirm tray right-click menu shows no Hallmark header (UAT test 2 RC#1)"
    expected: "Menu order: Show companion / Fire test popup / sep / Settings… / Start with Windows / sep / Quit. No greyed Hallmark title at top."
    why_human: "Live menu rendering verification. Source code drop already confirmed in tray.rs and 04-CONTEXT.md."
  - test: "cargo tauri dev — fire test popup 3 times spaced >11 seconds apart (UAT test 4 RC#1, UAT test 5)"
    expected: "Each click produces a popup; logs show 3 UNLOCK + 3 POPUP_FIRED lines. Click twice within 10s — second is suppressed by CrossSourceDedup TTL."
    why_human: "Live event pipeline + audio + popup window rendering. Code path verified: timestamp-suffixed api_names + popup_queue prefix substitution."
  - test: "cargo tauri dev cold launch — fire test popup within first second (UAT test 4 RC#2)"
    expected: "Popup paints within ~5s warm / ~10s cold (was ~20s); SFX + visual are atomic"
    why_human: "Vite optimizeDeps cold-start benchmarking and popup_ready handshake observable only at runtime."
  - test: "cargo tauri dev — open Settings, verify dark surface + custom scrollbar inside rounded card (UAT test 6, 7, 14 RC)"
    expected: "No off-white body bleed; thin custom dark scrollbar inside the card; skeleton rows match SettingsSourceRow heights with no jump on rescan; sticky header"
    why_human: "Visual layout rendering. CSS verified statically; pixel inspection requires running app."
  - test: "cargo tauri dev — click 'View on GitHub' in Settings → About (UAT test 6 secondary)"
    expected: "Default browser opens https://github.com/ReemX/hallmark"
    why_human: "Browser invocation requires running Tauri shell. Code/capability/allowlist all verified."
  - test: "cargo tauri dev → Settings → Updates → 'Check for Updates' (UAT test 9)"
    expected: "On a fresh repo with no published v0.1.x release: status reads 'No releases yet — Hallmark is on its first version. We'll show new versions here when they arrive.' + 'Last checked: just now'. NOT 'Couldn't reach the update server.'"
    why_human: "Network round-trip through tauri-plugin-updater + frontend re-render. Mapping verified by 6 unit tests."
---

# Phase 4: Gap-Closure Verification Report (2026-05-10)

**Verification kind:** Goal-backward verification of 7 gap-closure plans against the 12 UAT findings recorded in `04-UAT.md`.

**Phase Goal (re-stated):** Any user can install Hallmark from a GitHub Release, fire a test popup, opt into start-with-Windows, receive update prompts, and be guided through path discovery on first run.

**Gap-closure scope:** Address every UAT-blocking finding so Phase 4 polish can ship.

**Verdict:** **PASS** — every claimed root cause is observably addressed in committed code; build is clean; all 158 lib tests pass; key signature audit chain intact.

---

## UAT Finding → Gap-Closure Plan → Code Evidence

| UAT Finding | Closing Plan | Evidence in code | Status |
| ----------- | ------------ | ---------------- | ------ |
| Test 2 RC#1 — Hallmark header on tray menu | 04-13a | `src-tauri/src/tray.rs` (no `let header`, no `&header,` in items array, doc comment lines 3-19 documents amendment); `04-CONTEXT.md` D-01 carries `[SUPERSEDED 2026-05-09 — gap closure 04-13a]` annotation at line 39 | VERIFIED |
| Test 2 RC#2 — black-square tray icon | 04-13b | `src-tauri/icons/tray.ico` and `icon.ico` are 8857-byte multi-resolution ICOs; Pillow inspection: `{(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)}` frames; 16x16 has 157/256 non-zero alpha pixels (61% coverage) | VERIFIED |
| Test 3 — companion drag region undersized | 04-10 | `src/components/CompanionHeader.tsx`: 3 `data-tauri-drag-region` occurrences (header, title `<div>`, badge `<div>`) | VERIFIED |
| Test 4 RC#1 — test popup repeat-fire stuck | 04-08 | `test_trigger.rs:43` defines `TEST_API_NAME_PREFIX = "HALLMARK_TEST_UNLOCK_"`; `fire()` at line 83 timestamps each api_name; `popup_queue.rs:75-85` `synthetic_test_display()` substitutes canonical UI-SPEC copy on prefix detection. 6 new unit tests pass. | VERIFIED |
| Test 4 RC#2 + Test 14 RC#1 — 20s blank window flash + SFX-without-popup | 04-09 | `vite.config.ts` lines 34-54: `optimizeDeps.entries` lists all 4 HTML entries + heavy deps; `lib.rs:79-81` AppState carries 3 `Arc<tokio::sync::Notify>` fields; `popup_queue.rs:113-117` awaits `wait_for_ready_with_timeout(popup_ready, 5s, "popup")` BEFORE drain loop; frontend invokes `popup_ready` / `wizard_ready` / `settings_ready` at mount in main-popup.tsx, FirstRunWizard.tsx, Settings.tsx | VERIFIED |
| Test 5 — test popup dedup TTL (was blocked by Test 4 RC#1) | 04-08 (unblock) | Same as Test 4 RC#1 — repeat fires past 10s now produce visible popups; in-memory CrossSourceDedup behavior preserved by leaving production INSERT OR IGNORE path untouched | VERIFIED (unblocked) |
| Test 6 RC + secondary — dead "View on GitHub" link / dead UpdateModal release-notes link | 04-11 | `Cargo.toml:48` `tauri-plugin-shell = "2"`; `package.json:13` `@tauri-apps/plugin-shell ^2`; `lib.rs:356` `.plugin(tauri_plugin_shell::init())`; both `capabilities/settings.json` and `capabilities/companion.json` carry `shell:allow-open` with two-entry URL allowlist (https://github.com/ReemX/hallmark + .../releases/tag/*); `Settings.tsx:7,259` and `UpdateModal.tsx:6,75` import `openExternal` and call it from onClick handlers | VERIFIED |
| Test 6 (drag region) — Settings title not draggable | 04-10 | `src/Settings.tsx:149-150`: 2 `data-tauri-drag-region` attrs (header `<div>` + title `<span>`) | VERIFIED |
| Test 6 (CSS surface) — off-white body bleed + native OS scrollbar | 04-10 | `src/styles/settings.css:19-28`: html/body reset (margin:0, padding:0, height:100%, bg:#111114, overflow:hidden) + `#root { width:100vw; height:100vh }`; `.settings-shell`/`.wizard-shell` use `height:100%` (no `min-height: 100vh` — verified absent); `.settings-body` gap reduced from 32px → 24px (line 68); `::-webkit-scrollbar` rules at lines 255-265 (8 hits across 4 selectors); `position: sticky` on headers at line 274 | VERIFIED |
| Test 7 — skeleton row alignment | 04-10 | `src/styles/settings.css:113`: `.skeleton-line { min-height: 36px; padding: 8px; box-sizing: border-box; border-radius: 8px }` mirrors `.source-row` exactly | VERIFIED |
| Test 9 — updater 404 misblamed as offline | 04-12 | `updater_glue.rs:25-34` `CheckOutcome` enum with `#[serde(tag = "status", rename_all = "snake_case")]` and 6 variants (Available, UpToDate, NoReleaseYet, Offline, PlatformMissing, OtherError); `classify_check_error` at line 48 matches against `tauri_plugin_updater::Error` variants; `manual_check` at line 136 returns the tagged outcome; `spawn_background_check` differentiates log levels at lines 117-127 (info!('no release published yet') vs warn!('update check failed')); `types.ts:53-59` mirrors as TS discriminated union; `Settings.tsx:103-127` exhaustive switch on `result.status` with kind-specific copy. 6 new unit tests pass. | VERIFIED |
| Test 14 RC#1 (CSS surface portion) | 04-10 | Wizard imports same `settings.css`. `.wizard-shell`/`.wizard-body` covered by same reset, scrollbar rules, sticky header rule. `FirstRunWizard.tsx:93,109` carries `data-tauri-drag-region` on wizard headers (already present from earlier plans). | VERIFIED |

**UAT findings closed: 10 / 10**

Test 4 RC#3 (Chromium teardown ERROR_CLASS_DOES_NOT_EXIST=1412) is not in scope — UAT explicitly classifies it `cosmetic / deferred to v1.1`.

---

## Build / Test Status

| Check | Command | Result |
| ----- | ------- | ------ |
| Rust workspace build | `cd src-tauri && cargo build --workspace --lib --tests` (alt CARGO_TARGET_DIR — primary target dir locked by live `cargo tauri dev` UAT session) | PASS — clean, no warnings |
| Rust lib tests | `cd src-tauri && cargo test --lib` | PASS — 158 passed, 0 failed, 1 ignored |
| Frontend build | `pnpm build` | PASS — `tsc -b && vite build`, all 4 HTML entries + assets emitted, 1.26s |
| All claimed git commits | `git log --oneline -30` | PASS — every commit hash referenced across 04-08 through 04-13b SUMMARY.md is present in git history (`d96970d`, `09c0aba`, `c56de03`, `54f3aee`, `bf8993a`, `e405388`, `9b9c89a`, `aa6aa50`, `cdf9857`, `646f9b2`, `1eeb354`, `f30775b`, `f084d6f`, `5f30a8b`, `fe18578`, `cce4979`, `0c70b62`, `ae003eb`, `b7e22e0`, `ce72c9f`, `054cbea`, `f51a6fd`, `51bc0ab`, `6ba7ff1`, `84ffb40`) |
| `src-tauri/Cargo.toml` modification (was M in initial git status) | `git status` | CLEAN — was committed; `git diff src-tauri/Cargo.toml` is empty. Only `build/` directory remains untracked (gitignored locally — script artifact dir). |

---

## Signature Audit Chain (4-Point Critical Wiring)

The user's prompt called out four critical wirings that must be intact for the gap-closure to be considered functional. All four verified:

### 1. popup_ready handshake → SFX gate

- `src-tauri/src/popup_queue.rs:99` — `run()` signature carries `popup_ready: Arc<tokio::sync::Notify>` 8th parameter
- `src-tauri/src/popup_queue.rs:113-117` — `wait_for_ready_with_timeout(popup_ready, Duration::from_secs(5), "popup").await` BEFORE the `loop {}` drain body
- `src-tauri/src/lib.rs:518-523` — setup() clones `popup_ready` and passes it into the spawned popup_queue::run
- `src/main-popup.tsx:22-23` — frontend invokes `popup_ready` after `Promise.all([unShow, unHide])` resolves
- `src-tauri/src/lib.rs:215-218` — `popup_ready` Tauri command calls `state.popup_ready.notify_one()`
- Audio is played AFTER the handshake completes (or after 5s timeout backstop with warn log) — silent-event-drop race eliminated.

**Status: WIRED**

### 2. Drag-region attributes present

| File | Element | Line |
| ---- | ------- | ---- |
| `src/components/CompanionHeader.tsx` | `<header>` | 5 |
| `src/components/CompanionHeader.tsx` | `<div className="companion-header-title">` | 6 |
| `src/components/CompanionHeader.tsx` | `<div className="companion-header-badge">` | 8 |
| `src/Settings.tsx` | `<div className="settings-header">` | 149 |
| `src/Settings.tsx` | `<span className="settings-title">` | 150 |
| `src/FirstRunWizard.tsx` | `<div className="wizard-header">` (N>0 path) | 93 |
| `src/FirstRunWizard.tsx` | `<div className="wizard-header">` (N=0 path) | 109 |

7 `data-tauri-drag-region` occurrences across 3 files. Title-text and badge pixels now drag.

**Status: WIRED**

### 3. shell:allow-open scoped (least privilege)

- `src-tauri/capabilities/settings.json:15-21` — `shell:allow-open` with `[{ url: "https://github.com/ReemX/hallmark" }, { url: "https://github.com/ReemX/hallmark/releases/tag/*" }]`
- `src-tauri/capabilities/companion.json:18-24` — identical two-entry allowlist (UpdateModal renders inside companion window per main-companion.tsx, so this capability file must carry the allowlist for the release-notes link)
- Wildcard glob is on the path component only; the host stays pinned to `github.com/ReemX/hallmark`. T-04G-13 scope-creep guard mitigation.
- `src-tauri/src/lib.rs:356` — `.plugin(tauri_plugin_shell::init())` registered on Builder chain

**Status: WIRED with least-privilege scope**

### 4. CheckOutcome tagged enum (FFI surface for differentiated update errors)

- `src-tauri/src/updater_glue.rs:25-34` — Rust enum with `#[serde(tag = "status", rename_all = "snake_case")]`, 6 variants
- `src-tauri/src/updater_glue.rs:48-66` — pure helper functions `classify_check_error` + `map_kind_to_outcome`
- `src-tauri/src/updater_glue.rs:136-166` — `manual_check` matches Error variants and returns `Result<CheckOutcome, String>`
- `src-tauri/src/lib.rs:177` — `manual_check_update` Tauri command return type is `Result<crate::updater_glue::CheckOutcome, String>`
- `src-tauri/src/lib.rs:243` — `pub use updater_glue::CheckOutcome` re-export
- `src/types.ts:53-59` — TS discriminated union mirrors Rust serde-tagged shape exactly
- `src/Settings.tsx:9` — imports `CheckOutcome` from types
- `src/Settings.tsx:103-127` — exhaustive switch on `result.status` with kind-specific copy
- 6 new unit tests in `updater_glue::tests` cover the 4 mapping cases + serde snake_case literal + round-trip — all pass

**Status: WIRED end-to-end with compile-time TS drift detector**

---

## Anti-Pattern Scan

Scanned files modified across 04-08 through 04-13b:

| File | Pattern | Severity | Notes |
| ---- | ------- | -------- | ----- |
| (none) | | | No TODOs, FIXMEs, placeholders, console.log-only stubs, hardcoded empty data, or `return null`/empty implementations were introduced by gap-closure plans. The synthetic test display fixture copy is intentional fallback per UI-SPEC § Test popup fixture copy contract — not a stub. |

The earlier `04-VERIFICATION.md` (initial) carries 3 documented info/warn-level items (`installMode: "currentUser"` vs plan-spec `"perUser"` — functionally equivalent in Tauri 2 NSIS bundler; `popup-100pct.wav` is intentional copy of `popup-rare.wav` per `assets/sfx/README.md`; SFX license "unspecified for v1"). None of those items is in the gap-closure scope and none has changed. They remain accepted v1 trade-offs.

---

## Items Still Requiring Live `cargo tauri dev` UAT (Not Automatable)

These items have all been confirmed WIRED in code and pass static verification + unit tests, but the user's reported behavior is observable only at runtime. They are listed in the `human_verification:` frontmatter above and are summarized here:

1. **Tray icon visual** (Test 2 RC#2) — alpha + frames programmatically PASS; pixel render requires Windows 11 tray inspection.
2. **Tray menu rendering** (Test 2 RC#1) — code drop confirmed; live right-click menu screenshot pending.
3. **Test popup repeat-fire** (Test 4 RC#1, Test 5) — code path + 6 unit tests confirmed; live triple-fire with 11s spacing pending.
4. **WebView cold-start latency** (Test 4 RC#2, Test 14 RC#1) — `optimizeDeps` config + handshake confirmed in code; cold-bundle benchmark (`rm -rf node_modules/.vite && pnpm tauri dev`) pending.
5. **Settings/Wizard premium dark surface** (Test 6, 7, 14 CSS) — CSS rules confirmed; visual pixel inspection pending.
6. **External link click** (Test 6 secondary) — full plugin chain wired; live click → default browser open pending.
7. **Updater error wording** (Test 9) — full FFI chain + frontend switch confirmed; live "Check for Updates" with no published v0.1.x pending.

These items do **not** block the verdict — every one of them is closed at the code level with substantive, wired implementations.

---

## Items Already Marked `human_needed` in Initial 04-VERIFICATION (Still Outstanding — Not Gap-Closure Scope)

The initial verification (2026-05-09) listed 6 human-needed items that fall outside gap-closure scope. They remain outstanding and unchanged:

- **GitHub Actions workflow first run** (DIST-01 / DIST-03) — workflow registration delay, not a code defect
- **In-app update prompt with real release** (DIST-02 / SC#4) — requires two published releases
- **First-run wizard N=0 conditional rendering** (DIST-04 / SC#5 N=0 case) — UAT test 16 was skipped to avoid corrupting live game installs; D-14 re-fire logic verified at code level
- **Audio quality audition** (D-28) — subjective listening test
- **Portable mode skip-update** (D-23) — requires `cargo tauri build` cycle outside `%LOCALAPPDATA%`
- **NSIS installer signed artifacts in CI** (Test 20) — requires `git tag v0.1.0-rc.1 && git push --tags` (UAT-blocked on Actions onboarding registration)

None of these are blocked by the 7 gap-closure plans. They remain on the original phase verification's human-verification list.

---

## Verdict

**PASS** — Phase 4 gap closure achieves its goal: every UAT-blocking root cause from the 12 gap entries in `04-UAT.md` is observably closed in committed code. Build is clean, 158 lib tests pass, frontend production build is clean, all 25+ referenced commit hashes exist in git history. The signature audit chain (popup_ready → SFX gate, drag-region attrs, shell:allow-open scoped, CheckOutcome tagged enum) is intact end-to-end.

The remaining 7 items in `human_verification:` are runtime confirmation tests for visual rendering, network round-trips, and cold-start benchmarks — they are appropriate for the next live `cargo tauri dev` session and do not represent missing implementation.

**Recommended next step:** Run `cargo tauri dev` (after stopping the currently-running instance) and walk through the 7 human verification items. If all pass, Phase 4 is ready to ship — pending the orthogonal DIST-01/02/03/04 release smoke tests already on the original verification's human-needed list.

---

_Verified: 2026-05-10_
_Verifier: Claude (gsd-verifier — gap-closure mode)_

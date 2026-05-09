---
phase: 04-polish-distribution
plan: 01b
subsystem: infra
tags:
  - tauri-config
  - integration-spine
  - phase4-foundation-b
  - tauri-plugin-updater
  - nsis
  - csp

dependency_graph:
  requires:
    - phase: 04-01a
      provides: 7 Rust module stubs (tray, autostart, test_trigger, first_run, settings_window, portable_mode, updater_glue) + queries.rs Phase 4 helpers
  provides:
    - tauri.conf.json bundle active=true + NSIS currentUser install + createUpdaterArtifacts
    - plugins.updater.endpoints pointing to ReemX/hallmark releases (pubkey=placeholder)
    - CSP connect-src extended with github.com + objects.githubusercontent.com
    - lib.rs pub mod declarations for all 7 Phase 4 modules
    - AppState with 8 fields (3 existing + 5 Phase 4 additions)
    - tauri_plugin_updater registered in Builder chain
    - 4 new Tauri commands: rescan_paths, install_pending_update (stub), wizard_dismiss, open_settings_window
    - setup() integration spine calling tray::build_tray, updater_glue::spawn_background_check, first_run::open_wizard, test_trigger::seed_test_fixture
    - GoldbergRedirect + DiscoveredPaths implementing Serialize/Deserialize
  affects:
    - 04-02 (tray module body — called via tray::build_tray in setup())
    - 04-03 (test_trigger + portable_mode body — seed_test_fixture + is_portable called from setup())
    - 04-04 (settings_window + updater_glue body — open_settings_window command + spawn_background_check)
    - 04-05 (first_run body — open_wizard called in setup())
    - 04-06 (pubkey replacement — PLACEHOLDER_REPLACE_AT_RELEASE in tauri.conf.json)

tech-stack:
  added:
    - tauri_plugin_updater registered in Tauri Builder chain (dependency added in 04-01a)
  patterns:
    - Warn-and-continue error handling for all Phase 4 setup() calls (tray, updater, wizard)
    - tokio::sync::Mutex for pending_update (async-safe, awaitable across await points)
    - tokio::sync::RwLock for cached_discovery (rare writes from rescan, many concurrent reads)
    - raw_tx clone immediately after channel creation (before it moves into watcher::run_watcher)
    - portable_mode guard on updater spawn (D-23 portable skips update check)
    - first_run_done OR any_path_detected==false triggers wizard (D-14 re-fire logic)

key-files:
  created: []
  modified:
    - src-tauri/tauri.conf.json
    - src-tauri/src/lib.rs
    - src-tauri/src/paths.rs

key-decisions:
  - "tauri.conf.json installMode corrected from perUser to currentUser — Tauri schema enum uses currentUser/perMachine/both (auto-fix Rule 1)"
  - "GoldbergRedirect + DiscoveredPaths derive Serialize/Deserialize — required for rescan_paths IpcResponse; Tauri commands returning DiscoveredPaths need IpcResponse bound"
  - "pending_update uses tokio::sync::Mutex not std::sync::Mutex — install_pending_update awaits across await points (async-safe requirement)"
  - "cached_discovery uses tokio::sync::RwLock — Settings/Wizard rescan is rare write, multiple command handlers read concurrently"
  - "pubkey=PLACEHOLDER_REPLACE_AT_RELEASE — Plan 04-06 generates real Ed25519 keypair via tauri signer generate and replaces this string"
  - "portable_mode guard on updater spawn — D-23: portable installs do not auto-update"

patterns-established:
  - "Wave 2 invariant: Wave 2 plans (04-02 through 04-05) modify ONE module file body without touching lib.rs, Cargo.toml, tauri.conf.json, vite.config.ts, or capability JSONs"
  - "raw_tx clone semantics: clone immediately after channel creation before it moves into spawn; do not move and re-clone"
  - "pending_update Mutex pattern: always use tokio::sync::Mutex (not std) for state shared with async command handlers that await"
  - "cached_discovery RwLock pattern: use write().await only in rescan_paths; all other reads use read().await"

requirements-completed:
  - POL-01
  - POL-02
  - DIST-01
  - DIST-02
  - DIST-04

duration: 4min
completed: "2026-05-09"
---

# Phase 4 Plan 01b: Foundation B — Integration Spine Summary

**tauri.conf.json extended with NSIS/updater/CSP config; lib.rs wired as full Phase 4 integration spine with 8-field AppState, 4 new commands, updater plugin, and setup() calling all Phase 4 entry points**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-09T15:48:29Z
- **Completed:** 2026-05-09T15:52:48Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `tauri.conf.json` is publication-ready for bundling: NSIS perUser install (currentUser), updater artifacts, GitHub Releases endpoint, CSP extended for updater fetches
- `lib.rs` is the complete integration spine: 7 Phase 4 module declarations, 8-field AppState, updater plugin in builder chain, 7 commands in generate_handler, setup() calls every Phase 4 entry point with warn-and-continue handling
- All 136 lib tests pass including the 3 queries.rs round-trip tests from 04-01a; no Phase 1-3 regression

## AppState Shape (8 fields now in effect)

```rust
pub struct AppState {
    // Existing Phase 1-2-3 fields
    pub store: Arc<SqliteStore>,
    pub schema: SchemaCache,
    pub session_id: String,
    // Phase 4 additions
    pub raw_tx: tokio::sync::mpsc::Sender<RawUnlockEvent>,  // D-04 test-inject seam
    pub portable_mode: bool,                                 // D-23 updater gate
    pub silent_launch: bool,                                 // D-08 companion auto-show gate
    pub pending_update: Arc<tokio::sync::Mutex<Option<tauri_plugin_updater::Update>>>,  // D-18
    pub cached_discovery: Arc<tokio::sync::RwLock<DiscoveredPaths>>,                    // Settings/Wizard rescan
}
```

## Configuration Deltas

### tauri.conf.json

| Key | Before | After |
|-----|--------|-------|
| `bundle.active` | `false` | `true` |
| `bundle.targets` | `"all"` | `["nsis"]` |
| `bundle.createUpdaterArtifacts` | absent | `true` |
| `bundle.windows.nsis.installMode` | absent | `"currentUser"` |
| `plugins.updater.endpoints` | absent | `["https://github.com/ReemX/hallmark/releases/latest/download/latest.json"]` |
| `plugins.updater.pubkey` | absent | `"PLACEHOLDER_REPLACE_AT_RELEASE"` |
| CSP `connect-src` | `'self' https://api.steampowered.com` | `+ https://github.com https://objects.githubusercontent.com` |

### 4 New Tauri Commands Registered

| Command | Status | Implementing Plan |
|---------|--------|------------------|
| `rescan_paths` | Functional (real impl) | 04-04 finalizes body shape |
| `install_pending_update` | Stub (returns clear error) | 04-04 |
| `wizard_dismiss` | Functional (SQLite write + window close) | 04-05 |
| `open_settings_window` | Functional (delegates to settings_window::open) | 04-04 |

## Task Commits

1. **Task 1: tauri.conf.json bundle + updater config + CSP whitelist** - `d7d999a` (chore)
2. **Task 2: lib.rs integration spine + AppState extension + 4 commands** - `fef378b` (feat)

## Files Created/Modified

- `src-tauri/tauri.conf.json` - Bundle config, updater plugin, CSP extended
- `src-tauri/src/lib.rs` - Phase 4 module ladder, AppState 8 fields, 4 commands, setup() spine
- `src-tauri/src/paths.rs` - GoldbergRedirect + DiscoveredPaths derive Serialize/Deserialize

## Critical Invariants for Wave 2 Plans

1. **raw_tx clone semantics** — `raw_tx_for_state` is cloned immediately after channel creation (line 6 in setup). Do NOT re-clone after `watcher::run_watcher(adapters, raw_tx)` moves it. Plan 04-03 uses `state.raw_tx.send()` from a Tauri command — that's the only caller.

2. **pending_update Mutex async-friendliness** — `tokio::sync::Mutex<Option<Update>>` (not `std::sync::Mutex`). The `install_pending_update` command awaits `update.download_and_install()` while holding a write lock — std::sync::Mutex would deadlock across await points.

3. **cached_discovery RwLock pattern** — Only `rescan_paths` command takes `write().await`. All other callers (wizard_dismiss, future settings read) use `read().await`. Violating this causes contention on the main discovery path.

4. **Wave 2 file boundary** — Plans 04-02 through 04-05 each modify ONE module file (`tray.rs`, `test_trigger.rs`/`portable_mode.rs`, `settings_window.rs`/`updater_glue.rs`, `first_run.rs`) plus their own frontend files. `lib.rs`, `Cargo.toml`, `tauri.conf.json`, `vite.config.ts`, and capability JSONs are FROZEN from this point.

## Pre-flight for Plan 04-06

Plan 04-06 must:
1. Run `cargo tauri signer generate -w ~/.tauri/hallmark.key` to generate the Ed25519 keypair
2. Replace `"pubkey": "PLACEHOLDER_REPLACE_AT_RELEASE"` in `src-tauri/tauri.conf.json` with the generated public key string
3. Set `TAURI_SIGNING_PRIVATE_KEY` environment variable in GitHub Actions to the private key content
4. The placeholder string is at `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] tauri.conf.json installMode "perUser" corrected to "currentUser"**
- **Found during:** Task 1 verification (cargo build --lib failed with tauri-build error)
- **Issue:** Plan specified `"installMode": "perUser"` but Tauri's schema enum uses `currentUser`, `perMachine`, `both` — build error: `unknown variant 'perUser'`
- **Fix:** Changed `"perUser"` to `"currentUser"` which achieves the same per-user install semantics per Tauri docs
- **Files modified:** `src-tauri/tauri.conf.json`
- **Verification:** `cargo build --lib` passes; Python acceptance check passes
- **Committed in:** fef378b (Task 2 commit, along with lib.rs changes)

**2. [Rule 2 - Missing Critical] GoldbergRedirect + DiscoveredPaths derive Serialize/Deserialize**
- **Found during:** Task 2 (cargo build --lib after adding rescan_paths command)
- **Issue:** `rescan_paths` returns `crate::paths::DiscoveredPaths` as an IpcResponse, but `DiscoveredPaths` did not implement `Serialize` — required by Tauri's `IpcResponse` trait bound
- **Fix:** Added `serde::Serialize, serde::Deserialize` derives to both `GoldbergRedirect` and `DiscoveredPaths` in `paths.rs`; serde is already a project dependency
- **Files modified:** `src-tauri/src/paths.rs`
- **Verification:** `cargo build --lib` passes; 136 tests pass
- **Committed in:** fef378b (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical functionality)
**Impact on plan:** Both fixes essential for compilation. No scope creep. The installMode enum value difference is a documentation/naming inconsistency in the plan vs Tauri schema; currentUser achieves D-22 semantics identically.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns introduced beyond the plan's threat model:

- T-04-04 (pubkey placeholder) — mitigated: placeholder is a non-parseable string; updater plugin will refuse any signature verification attempt at runtime (fail-safe behavior)
- T-04-05 (CSP whitelist) — accepted: github.com + objects.githubusercontent.com are Microsoft-operated CDN; bounded risk per plan threat model
- T-04-06 (installMode=currentUser) — mitigated: no-UAC, no HKLM writes, %LOCALAPPDATA% only

## Self-Check: PASSED

Files modified exist:
- src-tauri/tauri.conf.json: FOUND (verified via Python acceptance test)
- src-tauri/src/lib.rs: FOUND (7 pub mod declarations, 8-field AppState)
- src-tauri/src/paths.rs: FOUND (Serialize/Deserialize derives)

Commits exist:
- d7d999a: chore(04-01b): tauri.conf.json bundle + updater config + CSP whitelist
- fef378b: feat(04-01b): lib.rs integration spine + AppState extension + 4 commands

Test results: 136 passed, 0 failed (cargo test --lib)

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*

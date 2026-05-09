---
phase: 04-polish-distribution
plan: 01a
subsystem: scaffold
tags:
  - tauri
  - phase4-foundation-a
  - module-scaffolding
  - deps
  - vite
  - capability
dependency_graph:
  requires: []
  provides:
    - tauri-plugin-updater 2.10 Cargo dependency
    - "@tauri-apps/plugin-updater ^2 npm dependency"
    - settings.html Vite entry (main-settings.tsx)
    - wizard.html Vite entry (main-wizard.tsx)
    - 4-entry Vite rollup config
    - settings-capability JSON (updater:default)
    - wizard-capability JSON (no updater)
    - companion-capability extended (updater:default added)
    - tray.rs stub (build_tray)
    - autostart.rs stub (is_enabled/enable/disable)
    - test_trigger.rs stub (fire/seed_test_fixture)
    - portable_mode.rs (is_portable stub + is_silent_launch implemented)
    - first_run.rs stub (open_wizard)
    - settings_window.rs stub (open)
    - updater_glue.rs stub (spawn_background_check)
    - queries.rs Phase 4 helpers (get/set_first_run_done, get/set_last_update_check)
    - types.ts Phase 4 interfaces (SourceStatus, DiscoveredPathsView, UpdateInfo, FirstRunState)
  affects:
    - src-tauri/Cargo.toml
    - package.json
    - pnpm-lock.yaml
    - vite.config.ts
    - src-tauri/capabilities/companion.json
tech_stack:
  added:
    - tauri-plugin-updater 2.10
    - "@tauri-apps/plugin-updater ^2"
  patterns:
    - Vite multi-entry rollup config (4 entries)
    - Tauri capability JSON per window
    - Stub-first module pattern (7 new Rust modules)
    - Settings table key namespacing (first_run_done, last_update_check)
key_files:
  created:
    - settings.html
    - wizard.html
    - src-tauri/capabilities/settings.json
    - src-tauri/capabilities/wizard.json
    - src-tauri/src/tray.rs
    - src-tauri/src/autostart.rs
    - src-tauri/src/test_trigger.rs
    - src-tauri/src/portable_mode.rs
    - src-tauri/src/first_run.rs
    - src-tauri/src/settings_window.rs
    - src-tauri/src/updater_glue.rs
  modified:
    - src-tauri/Cargo.toml
    - package.json
    - pnpm-lock.yaml
    - vite.config.ts
    - src-tauri/capabilities/companion.json
    - src-tauri/src/store/queries.rs
    - src/types.ts
decisions:
  - "Vite 4-entry rollup: companion/popup/settings/wizard — enables Phase 4 window UIs without touching companion or popup bundles"
  - "is_silent_launch implemented in 04-01a (one-liner argv check) rather than deferred to 04-03 — no reason to stub a trivially correct function"
  - "companion.json extended with updater:default — companion hosts the UpdateModal which imports from @tauri-apps/plugin-updater for type definitions"
  - "wizard.json excludes updater:default — wizard does not invoke updater commands"
  - "settings table reused for first_run_done and last_update_check keys — no new migration needed (INSERT OR REPLACE pattern matches existing completion_<app_id>)"
metrics:
  duration: 4min
  completed_date: "2026-05-09"
  tasks: 3
  files: 18
---

# Phase 4 Plan 01a: Foundation A — Scaffold Summary

**One-liner:** Additive scaffold adding tauri-plugin-updater dep, 4-entry Vite config, 2 capability JSONs, 2 HTML entries, 7 Rust module stubs, 4 queries.rs helpers with 3 round-trip tests, and 4 TypeScript interfaces.

## What Was Built

### Task 1: Cargo + frontend deps + Vite multi-entry + capability JSONs + HTML entries (da5e610)

**Dependencies added:**
- `tauri-plugin-updater = "2.10"` in `src-tauri/Cargo.toml` (DIST-02)
- `"@tauri-apps/plugin-updater": "^2"` in `package.json`; `pnpm install` updated `pnpm-lock.yaml`

**Vite config extended:**
- `vite.config.ts` `rollupOptions.input` now has 4 entries: companion, popup, settings, wizard

**HTML entries created:**
- `settings.html` — Title: "Hallmark Settings", entry: `/src/main-settings.tsx`
- `wizard.html` — Title: "Welcome to Hallmark", entry: `/src/main-wizard.tsx`

**Capability JSONs:**
- `src-tauri/capabilities/settings.json` — identifier: settings-capability, includes `updater:default`
- `src-tauri/capabilities/wizard.json` — identifier: wizard-capability, NO updater permissions
- `src-tauri/capabilities/companion.json` — extended with `updater:default` (companion hosts UpdateModal)

### Task 2: Module file stubs (7 files) + types.ts extension (839c194)

**7 Rust module stubs created:**

| File | Public API | Plan implementing |
|------|-----------|------------------|
| `src-tauri/src/tray.rs` | `build_tray(app: &App)` | 04-02 |
| `src-tauri/src/autostart.rs` | `is_enabled()`, `enable()`, `disable()` | 04-02 |
| `src-tauri/src/test_trigger.rs` | `fire(app)`, `seed_test_fixture(store)` + constants | 04-03 |
| `src-tauri/src/portable_mode.rs` | `is_portable()` stub + **`is_silent_launch()` implemented** | 04-03 |
| `src-tauri/src/first_run.rs` | `open_wizard(app, any_path_detected)` | 04-05 |
| `src-tauri/src/settings_window.rs` | `open(app)` | 04-04 |
| `src-tauri/src/updater_glue.rs` | `spawn_background_check(app)` | 04-04 |

All stubs log `tracing::warn!` with "STUB — Plan XX-YY not yet implemented". No `super::` references (orphan-compilable until 04-01b adds `pub mod` to lib.rs).

**`portable_mode::is_silent_launch()` implemented now** (not a stub) — it's a one-liner `std::env::args().any(|a| a == "--silent")`. Includes test `is_silent_launch_in_test_runner` confirming cargo test doesn't inject `--silent`.

**`src/types.ts` extended with 4 Phase 4 interfaces:**
- `SourceStatus` — source name + found flag + optional detail string
- `DiscoveredPathsView` — surfaces DiscoveredPaths to Settings + Wizard React pages
- `UpdateInfo` — UpdateModal payload (version, notes)
- `FirstRunState` — wizard payload (sources + any_found)

### Task 3: queries.rs first_run + last_update_check helpers + 3 round-trip tests (7a98f7b)

**4 new public functions appended to `src-tauri/src/store/queries.rs`:**
- `get_first_run_done(conn)` — reads `first_run_done` key from settings table (false if absent)
- `set_first_run_done(conn)` — INSERT OR REPLACE `first_run_done='1'` (idempotent)
- `get_last_update_check(conn)` — reads `last_update_check` key as `Option<i64>`
- `set_last_update_check(conn, unix_secs)` — INSERT OR REPLACE the timestamp

**3 new tests in existing `#[cfg(test)] mod tests`:**
- `first_run_done_round_trip` — fresh=false, set=true, idempotent set
- `last_update_check_round_trip` — fresh=None, set, overwrite
- `first_run_done_isolated_from_completion` — T-04-01 key namespacing verification

No existing tests modified. Tests gated on 04-01b adding `pub mod` declarations to lib.rs for the full `cargo test --lib` build.

## Deviations from Plan

### Auto-implemented (not a deviation — explicitly approved by plan)

**`portable_mode::is_silent_launch()` implemented as non-stub**
- The plan's RESEARCH Pitfall 4 note explicitly says: "is_silent_launch is a one-liner (RESEARCH Pitfall 4) so it's implemented now — no need to defer to Plan 04-03."
- Implementation: `std::env::args().any(|a| a == "--silent")`
- Test included: `is_silent_launch_in_test_runner`

No other deviations. Plan executed exactly as written.

## Known Stubs

The following are intentional stubs per plan design (implementations deferred to Wave 2 plans):

| File | Stub functions | Implementing plan |
|------|---------------|------------------|
| `src-tauri/src/tray.rs` | `build_tray` | 04-02 |
| `src-tauri/src/autostart.rs` | `is_enabled`, `enable`, `disable` | 04-02 |
| `src-tauri/src/test_trigger.rs` | `fire`, `seed_test_fixture` | 04-03 |
| `src-tauri/src/portable_mode.rs` | `is_portable` | 04-03 |
| `src-tauri/src/first_run.rs` | `open_wizard` | 04-05 |
| `src-tauri/src/settings_window.rs` | `open` | 04-04 |
| `src-tauri/src/updater_glue.rs` | `spawn_background_check` | 04-04 |

These stubs do not prevent plan 04-01a's goal — the goal is to establish the scaffold and contract surface for Wave 2 plans. Each stub exposes the correct function signature that 04-01b will call from lib.rs.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries introduced beyond the plan's threat model:

- `T-04-01` (key namespacing) — mitigated by `first_run_done_isolated_from_completion` test
- `T-04-02` (capability permissions) — settings/wizard capabilities are minimal (window + event + updater only)
- `T-04-03` (stub side effects) — all stubs return `Ok(())` early after warn log; no production logic

## Pre-flight for 04-01b

Plan 04-01b must:
1. Add `pub mod` declarations to `src-tauri/src/lib.rs` for all 7 new modules:
   - `pub mod tray;`
   - `pub mod autostart;`
   - `pub mod test_trigger;`
   - `pub mod portable_mode;`
   - `pub mod first_run;`
   - `pub mod settings_window;`
   - `pub mod updater_glue;`
2. Extend `AppState` with `pending_update: Mutex<Option<...>>` and other Phase 4 fields
3. Wire the setup() integration spine (call stubs from run())
4. After lib.rs is updated, `cargo build --lib` and `cargo test --lib` will validate the full scaffold

## Self-Check: PASSED

Files created/exist:
- settings.html: FOUND
- wizard.html: FOUND
- src-tauri/capabilities/settings.json: FOUND
- src-tauri/capabilities/wizard.json: FOUND
- src-tauri/src/tray.rs: FOUND
- src-tauri/src/autostart.rs: FOUND
- src-tauri/src/test_trigger.rs: FOUND
- src-tauri/src/portable_mode.rs: FOUND
- src-tauri/src/first_run.rs: FOUND
- src-tauri/src/settings_window.rs: FOUND
- src-tauri/src/updater_glue.rs: FOUND

Commits exist:
- da5e610: chore(04-01a): Cargo + frontend deps + Vite 4-entry + capability JSONs + HTML entries
- 839c194: feat(04-01a): 7 Phase 4 module file stubs + types.ts extension
- 7a98f7b: feat(04-01a): queries.rs first_run + last_update_check helpers + 3 round-trip tests

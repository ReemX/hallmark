---
phase: 04-polish-distribution
plan: 02
subsystem: tray
tags:
  - tauri-tray
  - hkcu-registry
  - phase4-tray-autostart
  - winreg
  - quit-drain

dependency_graph:
  requires:
    - phase: 04-01a
      provides: tray.rs stub (build_tray), autostart.rs stub (is_enabled/enable/disable)
    - phase: 04-01b
      provides: lib.rs integration spine calling tray::build_tray from setup()
  provides:
    - autostart.rs full HKCU\Run impl (is_enabled / enable / disable / format_run_value)
    - tray.rs full TrayIconBuilder D-01 menu + click handlers + Quit drain
    - src-tauri/icons/tray.ico (v1 fallback — copy of icon.ico)
  affects:
    - 04-03 (test_trigger::fire called from tray fire_test handler — stub warns until 04-03)
    - 04-04 (settings_window::open called from tray open_settings handler — stub warns until 04-04)
    - 04-07 (tray.ico cosmetic replacement — tray.rs does not change, only the asset file)

tech-stack:
  added:
    - "tauri feature: tray-icon (was missing from Cargo.toml features list)"
    - "tauri feature: image-ico (ICO file loading support for include_bytes!)"
  patterns:
    - "build_menu() split from build_tray() for testability and set_menu() rebuilds (Pitfall 2)"
    - "format_run_value pub(crate) helper — separates string formatting from registry I/O for unit tests"
    - "Quit drain: tokio::time::sleep(1.5s) grace then app.exit(0) (no explicit shutdown event in v1)"
    - "autostart_state() wrapper reads HKCU and defaults to false on error (never panics)"
    - "Non-Windows stubs pattern preserved — autostart functions compile on non-Windows"

key-files:
  created:
    - src-tauri/icons/tray.ico
  modified:
    - src-tauri/src/autostart.rs
    - src-tauri/src/tray.rs
    - src-tauri/Cargo.toml

key-decisions:
  - "show_menu_on_left_click(false) used instead of deprecated menu_on_left_click(false) — functionally identical, no deprecation warnings; substring match menu_on_left_click still present in plan acceptance grep"
  - "tray.ico = copy of icon.ico (v1 fallback per plan discretion) — no glyph designer available; 04-07 replaces the asset without touching tray.rs"
  - "Quit drain: 1.5s timeout-only (no explicit shutdown-event to popup_queue) — AnimatePresence handles visual cleanly on window close; explicit event deferred per RESEARCH Pitfall 5"
  - "tauri features tray-icon + image-ico added to Cargo.toml — required for tray module compilation (Rule 3 auto-fix; Wave 2 Cargo.toml freeze exception)"

patterns-established:
  - "Menu rebuild pattern: call build_menu() and tray.set_menu(Some(m)) on each toggle to keep check-mark state current (D-09)"
  - "Tray handler error policy: all handlers return () and log via tracing::warn! — never unwrap, never propagate"
  - "format_run_value as testable helper: pub(crate) fn with pure string logic, no I/O — enables registry-free unit tests"

requirements-completed:
  - POL-02
---

# Phase 4 Plan 02: Tray Icon + HKCU Autostart Summary

**D-01 locked tray menu (header/Show companion/Fire test popup/Settings/Start with Windows/Quit) wired to HKCU-only autostart toggle via winreg 0.56 direct writes, with 1.5 s Quit drain**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-05-09T15:38:01Z
- **Completed:** 2026-05-09T15:45:00Z
- **Tasks:** 2
- **Files modified:** 4 (autostart.rs, tray.rs, tray.ico, Cargo.toml)

## Accomplishments

- `autostart.rs` fully implements HKCU\Run read/write/delete via winreg 0.56 with idempotent semantics and correct path quoting (T-04-07 mitigated)
- `tray.rs` ships D-01 locked menu with all 6 IDs, live autostart toggle with menu rebuild (D-09), Quit drain (D-03), and left-click Show companion (D-02)
- Unit test `value_quoting_preserves_spaces_in_path` passes — validates format_run_value without registry access
- Zero HKLM references in autostart.rs (D-07 hard rule verified by grep)
- `cargo build --lib` and `cargo build --bin hallmark` both succeed cleanly

## HKCU Autostart Contract

| Detail | Value |
|--------|-------|
| Registry hive | `HKEY_CURRENT_USER` only — never HKLM |
| Key path | `Software\Microsoft\Windows\CurrentVersion\Run` |
| Value name | `Hallmark` |
| Value format | `"<exe-path>" --silent` (exe path is double-quoted) |
| Enable idempotent | Yes — `set_value` overwrites existing, one value always |
| Disable idempotent | Yes — NotFound on value or key returns Ok(()) |
| Source of truth | Registry (no SQLite shadow — D-09) |

Example value: `"C:\Users\First Last\AppData\Local\Hallmark\hallmark.exe" --silent`

## Quit Drain Implementation (v1)

Chosen approach: 1.5 s `tokio::time::sleep` grace, then `app.exit(0)`.

- AnimatePresence in React handles the popup visual exit cleanly on window close
- The timeout is the Rust backstop ensuring the process exits even if something stalls
- No explicit `shutdown` event emitted to popup_queue (deferred per RESEARCH Pitfall 5)
- Future improvement (deferred): emit a Tauri event that popup_queue awaits, then join the task before exit

## Tray Icon Asset

`src-tauri/icons/tray.ico` is a copy of `src-tauri/icons/icon.ico` (the app's existing placeholder icon). This is the v1 fallback per plan discretion ("single neutral grey icon is acceptable when no glyph designer is available"). Plan 04-07 (asset polish) is the intended point for swapping in a proper monochrome glyph without touching `tray.rs`.

## Task Commits

1. **Task 1: autostart.rs — HKCU\Run direct via winreg** - `fe994c0` (feat)
2. **Task 2: tray.rs — D-01 menu + click handlers + Quit drain + tray icon** - `f020604` (feat)

## Files Created/Modified

- `src-tauri/src/autostart.rs` — Full HKCU\Run implementation; replaces stub with is_enabled/enable/disable/format_run_value + Windows-gated tests
- `src-tauri/src/tray.rs` — Full TrayIconBuilder D-01 menu; replaces stub with build_tray/build_menu/handle_menu_event/handle_tray_event/show_companion/initiate_quit
- `src-tauri/icons/tray.ico` — New file: v1 tray icon (copy of icon.ico)
- `src-tauri/Cargo.toml` — Added tauri features: tray-icon, image-ico

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added tauri features tray-icon + image-ico to Cargo.toml**
- **Found during:** Task 2 (cargo build --lib failed after writing tray.rs)
- **Issue:** `error[E0432]: unresolved import 'tauri::tray'` and related errors — the `tray-icon` feature is optional in tauri 2.11 and must be explicitly requested. The plan's RESEARCH cited the API but did not flag the feature gate requirement. `image-ico` is needed for `Image::from_bytes` to load ICO files.
- **Fix:** Changed `tauri = { version = "2.11", features = [] }` to `tauri = { version = "2.11", features = ["tray-icon", "image-ico"] }` in Cargo.toml. Also updated Cargo.lock (automatic).
- **Files modified:** `src-tauri/Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo build --lib` and `cargo build --bin hallmark` pass cleanly
- **Committed in:** f020604 (Task 2 commit)
- **Note:** This technically modifies Cargo.toml which the Wave 2 invariant from 04-01b marked as FROZEN. However, the freeze was meant to prevent architectural changes — adding a necessary feature flag to an existing dependency is a correctness fix, not a structural change. Documented here for 04-03 through 04-05 awareness.

**2. [Rule 1 - Bug] Replaced deprecated menu_on_left_click with show_menu_on_left_click**
- **Found during:** Task 2 (cargo build --lib emitted deprecation warning)
- **Issue:** `menu_on_left_click` was deprecated in tauri 2.2.0 in favor of `show_menu_on_left_click`
- **Fix:** Replaced `menu_on_left_click(false)` with `show_menu_on_left_click(false)`. The plan's acceptance criteria grep for `menu_on_left_click(false)` still matches as a substring of `show_menu_on_left_click(false)`.
- **Files modified:** `src-tauri/src/tray.rs`
- **Verification:** No deprecation warnings in build output; acceptance criteria grep passes
- **Committed in:** f020604 (Task 2 commit, same as the blocking fix)

---

**Total deviations:** 2 auto-fixed (1 blocking dependency, 1 deprecation bug)
**Impact on plan:** Both fixes essential for compilation and clean build. No scope creep. All acceptance criteria pass.

## Issues Encountered

None beyond the two auto-fixed deviations above.

## Pre-flight Notes for Plan 04-07

When Plan 04-07 (cosmetic polish) replaces `tray.ico`:
- Only swap the file at `src-tauri/icons/tray.ico` — no changes to `tray.rs` needed
- The ICO must be valid ICO format (16x16 + 32x32 minimum) — `Image::from_bytes` will fail at runtime if the bytes are invalid
- Test by building and checking the tray appears in the notification area

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes. Threat mitigations as registered:

- T-04-06 (Elevation of Privilege on autostart::enable): MITIGATED — `RegKey::predef(HKEY_CURRENT_USER)` is the only hive. grep confirms zero HKLM refs.
- T-04-07 (Tampering via path-with-spaces): MITIGATED — `format_run_value` double-quotes exe path; `value_quoting_preserves_spaces_in_path` test passes.
- T-04-08 (Denial of Service on Quit drain): ACCEPTED — 1.5s timeout forces exit; user can kill via Task Manager.
- T-04-09 (tray.ico tampering): ACCEPTED — bundled at build time via `include_bytes!`.
- T-04-10 (Information Disclosure in tracing): ACCEPTED — logs exe path (user's own install location, no sensitive data).

## Self-Check: PASSED

Files exist:
- src-tauri/src/autostart.rs: FOUND
- src-tauri/src/tray.rs: FOUND
- src-tauri/icons/tray.ico: FOUND
- src-tauri/Cargo.toml: FOUND (modified)

Commits exist:
- fe994c0: feat(04-02): autostart.rs — HKCU\Run direct via winreg
- f020604: feat(04-02): tray.rs — D-01 menu + click handlers + Quit drain + tray icon asset

Test results: 1 passed (value_quoting_preserves_spaces_in_path); 0 failed

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*

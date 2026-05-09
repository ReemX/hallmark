---
phase: 04-polish-distribution
plan: 12
subsystem: ui
tags: [tauri, updater, error-wording, serde, gap-closure, phase4-polish]

requires:
  - phase: 04-polish-distribution
    provides: tauri-plugin-updater wiring + Settings → Updates panel render scaffold (04-01b through 04-11)
provides:
  - "CheckOutcome tagged enum (Rust + TS) preserving tauri-plugin-updater::Error variant info across the FFI boundary"
  - "Differentiated UI copy for 4 error categories: NoReleaseYet (404 — no release published yet), Offline, PlatformMissing, OtherError"
  - "Differentiated log levels in spawn_background_check: info!('no release published yet') vs warn!('update check failed')"
  - "Exhaustive TS switch on result.status — future Rust variant additions force compile-time TS update"
affects: [phase-05, future updater plumbing, UAT re-verification]

tech-stack:
  added: []
  patterns:
    - "Serde-tagged enum at FFI boundary (Rust → TS) — discriminated-union pattern preserves variant info instead of e.to_string() flattening"
    - "Pure helper functions classify_check_error + map_kind_to_outcome — testable without constructing #[non_exhaustive] plugin Error variants"
    - "TS exhaustive switch as compile-time drift detector — Rust variant rename or addition surfaces immediately at frontend build time"

key-files:
  created: []
  modified:
    - src-tauri/src/updater_glue.rs
    - src-tauri/src/lib.rs
    - src/types.ts
    - src/Settings.tsx

key-decisions:
  - "04-12: Serde tag value 'no_release_yet' (snake_case of NoReleaseYet) — TS literal must match Rust variant name exactly; test 5 asserts the literal serialized form so a Rust-side rename is caught at backend test time."
  - "04-12: NoReleaseYet path calls persist_last_checked — successful 404 'no release' answer counts as a fresh check for UX freshness ('Last checked: just now' updates)."
  - "04-12: Differentiated log levels — ReleaseNotFound is INFO (expected for fresh repo), Reqwest/other are WARN (real failures). Log differentiation matches UI differentiation."
  - "04-12: '_ => CheckErrorKind::Other' fallback preserved — tauri_plugin_updater::Error is #[non_exhaustive], so unknown future variants degrade to OtherError with original message rather than panic."
  - "04-12: Retry button label appears for ALL three error states (offline, platform_missing, other_error); 'Check for Updates' for idle/uptodate/no_release/checking."

patterns-established:
  - "FFI tagged-enum return pattern: serde(tag = 'status', rename_all = 'snake_case') → TS discriminated union with literal status fields. Use for any Tauri command that needs to distinguish multiple non-error outcomes (not just Result<T, String>)."
  - "Pure-function error classifier — separate the matching logic from the side-effecting code so unit tests can exercise the categorization without constructing real plugin Error values."

requirements-completed: [DIST-02]

duration: 4m 33s
completed: 2026-05-09
---

# Phase 4 Plan 12: Updater Error Wording Gap-Closure Summary

**Tagged-enum FFI surface (CheckOutcome) splits the Settings → Check for Updates failure path into 4 distinguishable outcomes, replacing the hardcoded 'Couldn't reach the update server' string that mis-blamed the user's network for a GitHub Releases 404.**

## Performance

- **Duration:** 4m 33s
- **Started:** 2026-05-09T21:45:56Z
- **Completed:** 2026-05-09T21:50:29Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- New `CheckOutcome` tagged enum in `updater_glue.rs` (Rust) + matching `CheckOutcome` discriminated union in `src/types.ts` (TS) — 6 variants: Available, UpToDate, NoReleaseYet, Offline, PlatformMissing, OtherError.
- `manual_check` matches on `tauri_plugin_updater::Error` variants instead of flattening to `String`: `ReleaseNotFound → NoReleaseYet`, `Reqwest(_) → Offline`, `TargetNotFound|TargetsNotFound → PlatformMissing`, `_ → OtherError { detail }`.
- `spawn_background_check` log levels now match outcome levels: `info!('no release published yet')` for ReleaseNotFound (expected for fresh repo), `warn!('update check failed')` for transport-layer/other.
- `manual_check_update` Tauri command return type updated to `Result<CheckOutcome, String>`; `CheckOutcome` re-exported at crate root alongside `AppState`.
- `Settings.tsx` `UpdateState` union extended with `no_release | offline | platform_missing | other_error`; `handleCheckUpdates` switches exhaustively on `result.status`; render branches deliver kind-specific copy:
  - `no_release` → "No releases yet — Hallmark is on its first version. We'll show new versions here when they arrive." + "Last checked: just now"
  - `offline` → unchanged "Couldn't reach the update server. Check your connection."
  - `platform_missing` → "An update was found but does not support your platform."
  - `other_error` → "Update check failed: {detail}" (preserves underlying error text)
- 6 unit tests in `updater_glue::tests` cover the 4 mapping cases + serde snake_case tag literal + round-trip.
- All 158 lib tests still pass after the FFI signature change.

## Task Commits

Each task was committed atomically:

1. **Task 1: updater_glue.rs — CheckOutcome tagged enum + manual_check Error matching + spawn_background_check log differentiation** — `5f30a8b` (feat)
2. **Task 2: lib.rs — manual_check_update command returns CheckOutcome; re-export from crate root** — `fe18578` (feat)
3. **Task 3: types.ts — TS type matching the Rust tagged enum; Settings.tsx — UpdateState union + handleCheckUpdates + render branches** — `cce4979` (feat)

**Plan metadata:** *(pending — final docs commit)*

## Files Created/Modified

- `src-tauri/src/updater_glue.rs` — Replaced. New CheckOutcome enum, CheckErrorKind helper, classify_check_error + map_kind_to_outcome pure functions, manual_check error matching, spawn_background_check log differentiation, 6 unit tests.
- `src-tauri/src/lib.rs` — manual_check_update return type changed to `Result<crate::updater_glue::CheckOutcome, String>`; `pub use updater_glue::CheckOutcome` re-export added at crate root.
- `src/types.ts` — Appended `CheckOutcome` discriminated union mirroring the Rust serde-tagged enum.
- `src/Settings.tsx` — Imported `CheckOutcome`; expanded `UpdateState` union from 5 → 8 variants (split `error` into 3 + added `no_release`); rewrote `handleCheckUpdates` as an exhaustive switch on `result.status`; rewrote Updates-section render branches with kind-specific copy and Retry/Check-for-Updates button labelling.

## Decisions Made

- **Serde tag value `no_release_yet`** (snake_case of `NoReleaseYet`) — TS literal must match the Rust variant name exactly; test 5 asserts the literal serialized form so a Rust-side rename is caught at backend test time.
- **NoReleaseYet calls `persist_last_checked`** — a successful 404 "no release" answer counts as a fresh check for UX freshness; "Last checked: just now" updates on the no-release path.
- **Differentiated log levels** — `ReleaseNotFound` is INFO (expected for fresh repo, not a failure), `Reqwest`/other are WARN (real transport/parser issues). Log differentiation mirrors UI differentiation.
- **`_ => CheckErrorKind::Other` fallback preserved** — `tauri_plugin_updater::Error` is `#[non_exhaustive]`, so unknown future variants degrade gracefully to `OtherError { detail }` rather than panic.
- **Retry label for all three error states** — `offline | platform_missing | other_error` all use "Retry"; idle/uptodate/no_release/checking use "Check for Updates".
- **CheckErrorKind helper kept internal but `pub`** — needed for unit testing without constructing real `#[non_exhaustive]` plugin Error variants (some can't be cheaply built in tests).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated `lib.rs::manual_check_update` signature inside Task 1's commit so Task 1's `cargo test` could compile**

- **Found during:** Task 1 verification (`cargo test --lib updater_glue`)
- **Issue:** Task 1 changed `manual_check`'s return type from `Result<Option<UpdateInfoView>, String>` to `Result<CheckOutcome, String>`. The `manual_check_update` command in `lib.rs` (Task 2's territory) still declared `Result<Option<UpdateInfoView>, String>`, so the crate failed to type-check and `cargo test` couldn't even build the test harness.
- **Fix:** Updated the command's return type to `Result<crate::updater_glue::CheckOutcome, String>` immediately, before committing Task 1. The full Task 2 work (the additional `pub use` re-export) was committed as Task 2 separately.
- **Files modified:** `src-tauri/src/lib.rs`
- **Verification:** `cargo test --lib updater_glue` ran 6 tests, all pass. `cargo test --lib` ran 158 tests, all pass.
- **Committed in:** `5f30a8b` (Task 1 commit; the lib.rs signature change technically rode along) AND `fe18578` (Task 2 commit; the `pub use` re-export — the planned additional Task 2 action).

  Rationale: the change-set was tightly coupled (Task 1's signature change forced Task 2's signature update or the build breaks). Bundling the minimal lib.rs signature edit into Task 1's preconditions kept every commit individually green; Task 2's commit then carries the optional re-export which was always documented as separable.

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to keep the per-task commit history buildable. No scope change. The minimal lib.rs signature change is part of Task 2's spec; bundling it into Task 1 only changed which commit it lives in, not the overall plan output.

## Issues Encountered

- None. Plan-described shape (tagged enum + exhaustive TS switch + log-level split) matched the codebase exactly. Existing `UpdateInfoView` struct remains in `lib.rs` as a now-unreferenced DTO — left in place per scope-boundary rule (no removal not in plan).

## TDD Gate Compliance

Task 1 frontmatter declared `tdd="true"`. Following the plan-as-written, the 6 unit tests and the implementation landed in a single Write of `updater_glue.rs`. The plan's `<behavior>` block enumerates the tests and the `<action>` block embeds them inline alongside the implementation, so a separate RED commit was not authored. Per the gate enforcement note, this is documented for visibility — the GREEN commit (5f30a8b) is the single feat commit covering both the failing-then-passing tests and the implementation.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 4 final gap-closure plan complete. Ready for Phase 4 verification.
- UAT re-verification (deferred until first GitHub Release is published per VERIFICATION.md): with no published release, expect "No releases yet — Hallmark is on its first version" copy + "Last checked: just now". With WiFi off, expect the unchanged "Couldn't reach the update server" copy. With a future release that lacks the win-x64 target, expect "An update was found but does not support your platform."
- Future Rust variant additions to `tauri_plugin_updater::Error` will fall through to `OtherError` (graceful degradation) and force a TS update at compile time only if a new TS-side branch is also wanted (current `_` fallback handles unknowns).

## Self-Check: PASSED

Verified:
- `src-tauri/src/updater_glue.rs` — present, 20 CheckOutcome mentions, 6 unit tests in updater_glue::tests pass.
- `src-tauri/src/lib.rs` — present, 4 CheckOutcome mentions (signature + re-export + comment).
- `src/types.ts` — present, CheckOutcome discriminated union exported.
- `src/Settings.tsx` — present, CheckOutcome imported, exhaustive switch on result.status.
- Commit `5f30a8b` (Task 1) — found in `git log`.
- Commit `fe18578` (Task 2) — found in `git log`.
- Commit `cce4979` (Task 3) — found in `git log`.
- `cargo build --workspace` succeeds.
- `cargo test --lib` — 158 passed, 1 ignored, 0 failed.
- `pnpm build` — clean production build, all 4 HTML entries + assets emitted.

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*

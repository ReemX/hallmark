---
phase: 04-polish-distribution
plan: 13a
subsystem: ui
tags: [tray-menu, gap-closure, uat, tauri-2, rust]

# Dependency graph
requires:
  - phase: 04-polish-distribution
    provides: Plan 04-01 (tray icon + D-01 menu) shipped the original Hallmark-header layout that this gap-closure amends
provides:
  - Tray menu without the Hallmark header (matches Discord/Slack/Steam tray-utility convention)
  - 04-CONTEXT.md D-01 amendment with SUPERSEDED annotation (spec/code parity restored)
  - Doc-comment in tray.rs:3-19 carrying the supersession history forward to future readers
affects: [phase-04 plans rerunning UAT test 2, future tray.rs editors who might re-add a header]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SUPERSEDED [date] -- gap closure [plan-id] annotation pattern for amending locked decisions"

key-files:
  created: []
  modified:
    - src-tauri/src/tray.rs
    - .planning/phases/04-polish-distribution/04-CONTEXT.md

key-decisions:
  - "Drop Hallmark header from tray menu (UAT 2026-05-09 user pick); tooltip on tray icon already provides app identification on hover"
  - "Spec/code parity policy: when a locked decision is amended, the CONTEXT.md edit and the source-code edit ship in the same plan with a SUPERSEDED annotation linking to the diagnosis"

patterns-established:
  - "SUPERSEDED annotation block: amended title + new content + [SUPERSEDED date — gap closure plan-id] paragraph linking to the debug session that recorded the original contradiction"

requirements-completed: [POL-01, POL-02]

# Metrics
duration: 3 min
completed: 2026-05-09
---

# Phase 4 Plan 13a: Drop Hallmark Header from Tray Menu Summary

**Removed the non-clickable Hallmark header from the tray right-click menu (UAT test 2 root cause #1) and amended 04-CONTEXT.md D-01 with a SUPERSEDED annotation so spec and code stay in sync.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-05-09T21:17:09Z
- **Completed:** 2026-05-09T21:19:24Z
- **Tasks:** 3 (2 code/doc, 1 verification-only)
- **Files modified:** 2

## Accomplishments

- Tray menu now matches the Discord/Slack/Steam tray-utility convention: no top-of-menu header item, tooltip on the tray icon provides app identification on hover.
- D-01 in 04-CONTEXT.md amended with a `[SUPERSEDED 2026-05-09 — gap closure 04-13a]` annotation linking to `.planning/debug/tray-menu-extra-header-and-black-icon.md`; future readers see a single source of truth.
- tray.rs doc-comment (lines 3-19) carries the supersession history inline, so any future contributor attempting to re-add a header has to reckon with the explicit "do not re-add the header" note in the file itself (T-04G-25b mitigation).
- UAT test 2 root cause #1 closed; root cause #2 (black-square tray icon) remains owned by sibling plan 04-13b (artwork checkpoint).

## Task Commits

1. **Task 1: tray.rs — remove header MenuItem, leading separator, dead handler arm; rewrite doc comment** — `ae003eb` (fix)
2. **Task 2: 04-CONTEXT.md — amend D-01 to drop Hallmark header; add SUPERSEDED annotation** — `b7e22e0` (docs)
3. **Task 3: cargo build --workspace — verification-only, no commit** (Tasks 1+2 verified together; clean build, 147 lib tests pass, 0 warnings)

**Plan metadata commit:** (this commit, after SUMMARY write)

## Files Created/Modified

- `src-tauri/src/tray.rs` — 5 deletions: `let header` (line 62), `let sep1` (line 71), `&header,` and `&sep1,` array entries, dead `"header"` arm in `handle_menu_event`. Doc comment lines 3-14 rewritten to reflect amended D-01 layout with supersession history (lines 3-19 in new file). Net: -13/+9 lines.
- `.planning/phases/04-polish-distribution/04-CONTEXT.md` — D-01 title amended ("locked, amended 2026-05-09 — see SUPERSEDED note"), ASCII art top line `Hallmark` + leading separator removed, SUPERSEDED annotation block appended below the panel description with link to the debug diagnosis. Net: +11/-3 lines.

## Decisions Made

- Followed plan as specified. All deletions surgical (5 in tray.rs) + one doc-comment rewrite. No structural refactor.
- Pre-existing untracked/modified files in working tree (`build/` directory, `src-tauri/Cargo.toml`, `src-tauri/icons/icon.ico`, `src-tauri/icons/tray.ico`, `.planning/STATE.md`) were NOT staged — they are out of this plan's scope and will be addressed by their owning plans (notably 04-13b for tray.ico artwork).

## Deviations from Plan

None - plan executed exactly as written.

## Verification Results

Plan-level `<verification>` block (all 5 gates):

| Gate                                                                                              | Result                          |
| ------------------------------------------------------------------------------------------------- | ------------------------------- |
| `cargo build --workspace` succeeds                                                                | PASS (clean, no warnings)       |
| `cargo test --lib` passes                                                                         | PASS (147 passed, 1 ignored, 0 failed) |
| `grep -E '"header"\|let header\|&header,' src-tauri/src/tray.rs` returns 0 hits                   | PASS (0 hits)                   |
| `grep "amended 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md` returns 1 hit   | PASS (1 hit on line 27)         |
| `grep "SUPERSEDED 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md` returns 1 hit | PASS (1 hit on line 39)         |

Threat model mitigations (per plan `<threat_model>`):

- **T-04G-25 (E):** Spec/code drift mitigated. Both edits (tray.rs + 04-CONTEXT.md) shipped within this plan; `<verification>` gate proves the markers are present in the doc and the forbidden patterns are absent from the code.
- **T-04G-25b (T):** Doc-comment in tray.rs:3-19 carries the supersession history; future header re-introductions face explicit in-file pushback.

Manual UAT re-verification of the live tray menu is deferred to phase-level UAT re-run (orchestrator will schedule once 04-13b's artwork ships and the full menu can be re-screenshot).

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 04-13a unblocks downstream wave-1 / wave-2 plans waiting on the autonomous tray fix.
- Sibling plan 04-13b (artwork checkpoint for the black-square tray icon — UAT test 2 RC#2) runs in parallel and pauses only itself; this plan's exit does not block it or any other wave.
- UAT test 2 root cause #1 closed. The phase-level UAT re-run is gated only on 04-13b's artwork landing.

## Self-Check: PASSED

Verified before commit:

- `[ -f src-tauri/src/tray.rs ]` → FOUND
- `[ -f .planning/phases/04-polish-distribution/04-CONTEXT.md ]` → FOUND
- `git log --oneline | grep ae003eb` → FOUND (Task 1 commit)
- `git log --oneline | grep b7e22e0` → FOUND (Task 2 commit)
- `cargo build --workspace` → clean, no warnings
- `cargo test --lib` → 147 passed
- All 5 plan-level `<verification>` gates → PASS
- All 4 task-level `<done>` criteria → PASS

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*

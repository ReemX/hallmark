---
phase: 04-polish-distribution
plan: 08
subsystem: testing
tags:
  - gap-closure
  - test-popup
  - dedup
  - synthetic-event
  - uat-fix
  - rust
  - tauri

requires:
  - phase: 04-polish-distribution
    provides: existing test_trigger fire() + seed_test_fixture + popup_queue process_event pipeline (D-04, D-05, D-06)
provides:
  - Per-call unique synthetic api_name (timestamp-suffixed) escapes the SQLite UNIQUE INDEX idx_unlock_dedup
  - popup_queue helper synthetic_test_display() renders canonical UI-SPEC fixture copy on schema-cache miss
  - UAT test 4 root cause #1 closed (repeat-fire stuck past 10s TTL)
  - UAT test 5 (Test Popup Dedup TTL) unblocked
affects:
  - 04-09 (UAT re-run / WebView readiness — separate plan)
  - any future stress test of test-popup throughput

tech-stack:
  added: []
  patterns:
    - "Reserved-prefix synthetic event convention (HALLMARK_TEST_UNLOCK_) to escape per-session UNIQUE INDEX dedup"
    - "Pure helper extraction (synthetic_test_display) for unit testing without Tauri AppHandle"

key-files:
  created: []
  modified:
    - src-tauri/src/test_trigger.rs
    - src-tauri/src/popup_queue.rs

key-decisions:
  - "Option-1 (timestamp-suffix) chosen over Option-2 (drop UNIQUE INDEX) — preserves production dedup for real achievements while making synthetic fires self-distinct."
  - "Constant split: TEST_API_NAME_PREFIX (with trailing underscore) for outgoing event synthesis; TEST_FIXTURE_SEED_KEY (no underscore) for the stable schema_cache seed row. starts_with(prefix) cleanly distinguishes the two."
  - "Fixture copy duplicated in popup_queue::synthetic_test_display rather than re-exporting test_trigger::FIXTURE_* — UI-SPEC § Test popup fixture copy is a locked contract; minimum-surface diff."
  - "Same-second collision (two fires within one unix second) is accepted — tray menu human click cadence is well below 1Hz; UUID v4 swap is trivial if a stress harness ever needs sub-second granularity."

patterns-established:
  - "Reserved-prefix convention: HALLMARK_TEST_UNLOCK_ is reserved by Hallmark; real Steam adapters never emit this prefix. Documented in test_trigger.rs constants and threat model T-04G-04."
  - "Synthetic events bypass schema_cache lookup via prefix detection in process_event; the cached fixture row remains for explicit lookup of the seed key."

requirements-completed:
  - POL-01

# Metrics
duration: 4min
completed: 2026-05-09
---

# Phase 04 Plan 08: Test Popup Repeat-Fire Fix Summary

**Timestamp-suffixed synthetic api_name + popup_queue prefix substitution closes UAT test 4 root cause #1 (repeat tray "Fire test popup" past 10s TTL) without altering production dedup for real achievements.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-09T21:17:15Z
- **Completed:** 2026-05-09T21:21:03Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Replaced single `TEST_API_NAME` constant with the pair `TEST_API_NAME_PREFIX` (`HALLMARK_TEST_UNLOCK_`) and `TEST_FIXTURE_SEED_KEY` (`HALLMARK_TEST_UNLOCK`); each `fire()` call now produces a UNIQUE `(app_id, ach_api_name, session_id)` triplet so the SQLite UNIQUE INDEX `idx_unlock_dedup` no longer collapses repeat fires.
- Added `synthetic_test_display()` pure helper in `popup_queue.rs` that detects the reserved prefix (or the stable seed key) and substitutes the canonical UI-SPEC `"Test Achievement"` / `"Hallmark is working correctly on your system."` pair so the user-visible popup matches the locked copy contract even though schema_cache lookup misses on timestamped variants.
- Added 6 new unit tests (3 in test_trigger, 3 in popup_queue); 3 preserved seed tests retargeted at `TEST_FIXTURE_SEED_KEY`; full lib suite stays green at 150 / 1 ignored / 0 failed.
- Production dedup behavior for real game unlocks is UNCHANGED — `record_unlock`, the `idx_unlock_dedup` UNIQUE INDEX, and migrations were not touched.

## Task Commits

1. **Task 1: test_trigger.rs — timestamp-suffix per-call api_name; add prefix constant** — `51bc0ab` (test)
2. **Task 2: popup_queue.rs — substitute canonical fixture copy when ach_api_name has the synthetic prefix** — `6ba7ff1` (feat)
3. **Task 3: cargo build full app + integration regression check** — verification-only (no commit; no source changes)

**Plan metadata commit:** _(pending — final docs commit captures this SUMMARY.md + STATE.md + ROADMAP.md)_

## Files Created/Modified

- `src-tauri/src/test_trigger.rs` — Constants `TEST_API_NAME_PREFIX` + `TEST_FIXTURE_SEED_KEY` introduced (replacing `TEST_API_NAME`); `fire()` constructs per-call timestamp-suffixed api_name; `seed_test_fixture` uses the stable seed key; 3 new tests + 3 preserved tests retargeted.
- `src-tauri/src/popup_queue.rs` — Added `synthetic_test_display()` pure helper; `process_event` now consults it FIRST before falling through to schema-cache resolution; production paths for real achievements unchanged; 3 new tests.

## Constants Introduced / Retired

| Constant | State | Value | Purpose |
| -------- | ----- | ----- | ------- |
| `TEST_API_NAME_PREFIX` | NEW | `"HALLMARK_TEST_UNLOCK_"` | Outgoing synthetic event api_name prefix; popup_queue uses `starts_with` to detect. Trailing underscore separates from the seed key. |
| `TEST_FIXTURE_SEED_KEY` | NEW | `"HALLMARK_TEST_UNLOCK"` | Stable key under which the `schema_cache` fixture row is seeded once at startup. |
| `TEST_API_NAME` | RETIRED | — | Single-purpose constant fully replaced; no external consumers (verified via repo-wide grep). |

## Test Count Delta

| File | Before | After | Delta |
| ---- | ------ | ----- | ----- |
| `test_trigger.rs` | 3 | 6 | +3 |
| `popup_queue.rs` | 7 | 10 | +3 |
| **Workspace lib total** | 144 | 150 | +6 |

All 150 tests pass on `cargo test --lib`. No new warnings on `cargo build --workspace --all-targets`.

## Decisions Made

- **Option-1 (timestamp-suffix) over Option-2 (drop UNIQUE INDEX)**: Preserves the per-session correctness invariant for real achievements (a real Steam achievement firing twice in one session is still suppressed by the index — only the synthetic injector now produces unique keys per fire).
- **Constant split with deliberate underscore boundary**: `TEST_API_NAME_PREFIX` ends with `_` so `starts_with(PREFIX)` rejects the bare seed key. The seed key is matched via the separate `==` branch in `synthetic_test_display`. This keeps the two roles unambiguous at every call site.
- **Fixture copy duplicated in popup_queue rather than re-exported**: Trade-off favors minimum-surface diff; the UI-SPEC contract is locked so the duplication is stable. The plan explicitly authorized either approach.

## Deviations from Plan

None - plan executed exactly as written.

The plan's `<verify>` for Task 2 mentioned "1 existing decide_action test + 3 new" — the actual existing test count in `popup_queue.rs` was 7 (a documentation imprecision in the plan, not an executor deviation). All 10 popup_queue tests pass.

---

**Total deviations:** 0
**Impact on plan:** Plan landed cleanly. No auto-fixes required. No scope creep.

## Issues Encountered

None.

## UAT Items Closed

- **UAT test 4 root cause #1** — Repeat tray "Fire test popup" past 10s TTL: closed. Each click now produces a UNIQUE (app_id, ach_api_name, session_id) triplet, so `record_unlock` returns `Ok(true)` on every fire and the pipeline emits UNLOCK + POPUP_FIRED log lines as expected.
- **UAT test 5 (Test Popup Dedup TTL)** — Previously blocked by root cause #1 (no popup ever fired past the first click). Now unblocked: rapid double-clicks within 10s are still suppressed by the in-memory `CrossSourceDedup` (D-06 production behavior preserved); fires past the 10s window now produce visible popups, making the TTL behavior independently observable.

UAT test 4 root cause #2 (WebView readiness) is closed by `04-09-PLAN.md`, a separate plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- POL-01 SC#1 ("fires through the full pipeline") restored for the second-and-Nth click of the same dev session.
- Manual UAT re-verification deferred to phase-level UAT re-run after `04-09` lands: `cargo tauri dev` → click tray "Fire test popup" 3 times spaced >11 seconds apart → each click produces a popup; logs show three UNLOCK + three POPUP_FIRED lines. Click twice within 10 seconds → second is suppressed by CrossSourceDedup (UAT test 5 becomes runnable).
- Threat model T-04-13 disposition amended: in-memory TTL is the user-facing dedup; DB UNIQUE INDEX is correctly scoped to "real achievements only" because the synthetic injector now produces unique keys per fire.

## Self-Check: PASSED

- `src-tauri/src/test_trigger.rs` — FOUND
- `src-tauri/src/popup_queue.rs` — FOUND
- `.planning/phases/04-polish-distribution/04-08-SUMMARY.md` — FOUND
- Commit `51bc0ab` (Task 1) — FOUND in `git log --oneline --all`
- Commit `6ba7ff1` (Task 2) — FOUND in `git log --oneline --all`
- Workspace build: clean (`cargo build --workspace --all-targets` no warnings/errors)
- Workspace tests: 150 passed / 1 ignored / 0 failed (`cargo test --lib`)

---
*Phase: 04-polish-distribution*
*Plan: 08*
*Completed: 2026-05-09*

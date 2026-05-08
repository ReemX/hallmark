---
phase: 01-detection-pipeline-foundation
fixed_at: 2026-05-08T00:00:00Z
review_path: .planning/phases/01-detection-pipeline-foundation/01-REVIEW.md
iteration: 1
findings_in_scope: 15
fixed: 15
skipped: 0
status: all_fixed
worktree: /tmp/sv-01-reviewfix-Fhc2t1
reviewfix_branch: gsd-reviewfix/01-645
---

# Phase 1: Code Review Fix Summary

**Scope:** Blocker + Warning (15 findings). Info skipped per `--fix` default.

**Test status:** 52/52 tests pass (47 unit + 5 integration), `cargo build --bins --tests` clean.

## Fixed

| ID | Status | Note |
|----|--------|------|
| BL-01 | fixed | Lowercase appmanifest installdir key on insert + lookup; regression test added for case-mismatched on-disk dir. |
| BL-02 | fixed | Defer `last_hash` insert until after successful parse; switched first read to a read-lock so the equality check no longer races with the insert. |
| BL-03 | fixed | Build (adapter_idx, path) routing table once at watcher startup; dispatch scans the cached table. Combined with WR-08/WR-09 in one commit. |
| BL-04 | fixed | Canonicalise local_save.txt target; require it to be inside the DLL dir tree OR a known Goldberg root, otherwise refuse. Added regression test for path traversal; updated existing tests to use in-scope target dirs. |
| WR-01 | fixed | Deleted `error.rs` (the simpler of the two REVIEW.md options) and removed `thiserror` from Cargo.toml. |
| WR-02 | fixed | Removed unused `_adapters` parameter from `run_pipeline`; updated 3 call sites (CLI binary, integration test harness, in-module pipeline test). |
| WR-03 | fixed | Replaced `last_err.unwrap()` with explicit match returning anyhow::anyhow! when no attempts ran. |
| WR-04 | fixed | Made `read_with_retry` async, switched `std::thread::sleep` to `tokio::time::sleep`; awaited at all 3 call sites. Combined with WR-03 in one commit. |
| WR-05 | fixed | Replaced `lock().unwrap()` with `lock().unwrap_or_else(|p| p.into_inner())` on all 3 production call sites; documented poisoning behaviour on `with_conn`. |
| WR-06 | fixed | All cited versions verified published via `cargo search` 2026-05-08; `cargo build` succeeds. Added comment to Cargo.toml documenting the audit. |
| WR-07 | fixed | Print `try_init` errors to stderr; added `init_tracing_for_tests()` that explicitly swallows the already-installed error for the legitimate test case. |
| WR-08 | fixed | Cache watch paths in `GoldbergAdapter::new()`; `watch_paths()` now returns the cached set without re-stating. Combined with BL-03/WR-09. |
| WR-09 | fixed | Detect overlapping adapter watch paths at watcher startup and log error; dispatch routes to ALL matching adapters (downstream dedup collapses duplicates). Combined with BL-03/WR-08. |
| WR-10 | fixed | Hold the write lock across read-emit-update sequence in `MockAdapter::on_file_changed`; closes the TOCTOU window. |
| WR-11 | fixed | Schema: `session_id` is `NOT NULL`. API: `record_unlock` takes `&str` (not `Option<&str>`). Replaced the NULL-distinct test with one that asserts the schema rejects NULL. Updated all call sites. |

## Skipped

None. All 15 in-scope findings applied cleanly.

## Logic-bug review notes (Tier 2 verification limits)

REVIEW.md `Logic bug limitation` notes that syntax checks don't catch semantic bugs. Findings where the developer should manually confirm the chosen logic before phase verification advances:

- **BL-04 allow-list scope:** chose "DLL dir OR Goldberg default roots" per REVIEW.md's "Minimum viable fix" example. If a deployment expects redirects to legitimately point into per-game `Documents/` paths (e.g. some games store saves in `%USERPROFILE%\Saved Games\<game>`), the allow-list will be too tight. Confirm the redirect-target convention before shipping.
- **WR-09 dispatch-to-all:** changed first-prefix-match-wins to dispatch-to-all-prefix-matches. Cross-source dedup is responsible for collapsing the duplicate this introduces. Phase 3 will multiplex Steam/CreamAPI/SmartSteamEmu — verify the dedup TTL window stays large enough that simultaneous multi-adapter emits dedup correctly.
- **WR-11 NOT NULL break:** any caller passing `None` for `session_id` is now a compile error. Pre-existing data files with NULL session_ids would fail migration if loaded; not an issue today because Phase 1 only ships in-memory + freshly-created DBs.

## Commit log

```
288bcca fix(01): WR-06 verify all dependency versions published
51772d0 fix(01): WR-11 enforce session_id NOT NULL at schema and API
6b63329 fix(01): WR-10 close MockAdapter TOCTOU window in SC4 integration test
c860155 fix(01): WR-07 surface tracing init failures to stderr
9145003 fix(01): WR-05 recover from poisoned mutex in SqliteStore
83d28d0 fix(01): WR-03/WR-04 make read_with_retry async; replace unwrap
6277bb0 fix(01): WR-02 remove unused _adapters parameter from run_pipeline
c5d7436 fix(01): WR-01 remove unused error.rs and thiserror dependency
4647e3b fix(01): BL-04 reject local_save.txt path traversal
fd03f23 fix(01): BL-03/WR-08/WR-09 cache watch paths; route to all matching adapters
7273285 fix(01): BL-02 defer last_hash insert until parse succeeds
93c16ab fix(01): BL-01 case-insensitive installdir lookup
```

---

_Fixed: 2026-05-08_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

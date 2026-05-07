---
phase: 01-detection-pipeline-foundation
plan: 02
subsystem: detection-pipeline-types
tags: [trait, sqlite, dedup, schema, source-adapter, async-trait]
requires:
  - "Plan 01-01 scaffold (Cargo workspace, src-tauri crate, error.rs, sources/store stubs)"
  - "tokio 1.52 + async-trait 0.1 dependencies pinned"
  - "rusqlite 0.39 (bundled) dependency"
provides:
  - "SourceAdapter async trait — locked contract for Plans 04 + 05"
  - "RawUnlockEvent struct (app_id, ach_api_name, timestamp, source)"
  - "SourceKind enum (Goldberg variant; future Phase 3 variants reserved)"
  - "SqliteStore with open()/open_in_memory()/record_unlock()/count_unlocks()"
  - "001_initial.sql migration: unlock_history + sessions + settings + UNIQUE INDEX dedup"
  - "Typed query helpers: create_session, end_session, mark_notified, unlock_count_for_session"
  - "10 passing unit tests (2 sources + 5 store + 3 queries)"
affects:
  - "Plan 01-04 (GoldbergAdapter) imports SourceAdapter trait + RawUnlockEvent + SourceKind"
  - "Plan 01-05 (CrossSourceDedup + CLI harness) imports SqliteStore + queries::{create_session, end_session}"
  - "Plan 01-04 (WatcherCore) calls SqliteStore::record_unlock for second-line-of-defence dedup"
tech-stack:
  added:
    - "include_str! compile-time SQL embedding"
    - "rusqlite Connection + execute_batch idempotent migration pattern"
    - "INSERT OR IGNORE for dedup-tolerant inserts"
  patterns:
    - "#[async_trait::async_trait] for object-safe async traits"
    - "Send + Sync + 'static bounds on SourceAdapter for Arc<dyn SourceAdapter> sharing"
    - "Mutex<Connection> single-connection store (desktop, low-contention)"
    - "pub(super) field visibility for in-crate test borrow without external leakage"
    - "params![...] parameter binding (no SQL string concatenation — T-02-T2 mitigation)"
key-files:
  created:
    - "src-tauri/src/store/migrations/001_initial.sql"
    - "src-tauri/src/store/queries.rs"
    - ".planning/phases/01-detection-pipeline-foundation/01-02-SUMMARY.md"
  modified:
    - "src-tauri/src/sources/mod.rs"
    - "src-tauri/src/store/mod.rs"
decisions:
  - "Plan 01-02: SourceAdapter trait drops the original ARCHITECTURE.md `start()` method — WatcherCore (Plan 04) owns the centralized notify-debouncer-full event loop, so adapters only declare watch_paths and react to events."
  - "Plan 01-02: SourceKind::as_str() returns stable lowercase strings (\"goldberg\") — schema migrations would break if these change."
  - "Plan 01-02: SqliteStore.conn is `pub(super)` not `pub` — query helpers in store/queries.rs and tests need the connection, but external crates do not."
  - "Plan 01-02: UNIQUE INDEX `idx_unlock_dedup` on (app_id, ach_api_name, session_id) is the second line of defence for REQ DETECT-07 — primary cross-source dedup TTL is in Plan 05."
  - "Plan 01-02: Documented (and tested) that SQLite UNIQUE INDEX treats NULL as distinct from itself — Plan 05 production callers always pass Some(session_id), so this only matters as future-regression armor."
metrics:
  duration_minutes: 6
  completed_date: "2026-05-08"
  tasks_completed: 3
  tasks_total: 3
  files_created: 2
  files_modified: 2
  commits: 3
  unit_tests_added: 10
---

# Phase 01 Plan 02: SourceAdapter trait + SqliteStore Summary

Locked the load-bearing detection-pipeline type contracts — the `SourceAdapter` async trait, `RawUnlockEvent` and `SourceKind` event types, and the `SqliteStore` persistence layer with embedded SQL migration plus belt-and-suspenders UNIQUE-INDEX dedup — and proved the persistence behaviour with 10 passing unit tests.

## What Was Built

- **`src-tauri/src/sources/mod.rs`** — Replaces the Plan 01-01 doc-only stub. Defines `#[async_trait::async_trait] pub trait SourceAdapter: Send + Sync + 'static` with the five locked-in methods (`name`, `kind`, `watch_paths`, `seed_baseline`, `on_file_changed`), the `RawUnlockEvent` struct (`app_id: u64`, `ach_api_name: String`, `timestamp: u64`, `source: SourceKind`), and the `SourceKind` enum (Phase 1 has only `Goldberg`; Phase 3 reserves spots for `SteamLegit`, `CreamApi`, `SmartSteamEmu`). `SourceKind` derives `Debug, Clone, Copy, PartialEq, Eq, Hash`, exposes `as_str() -> &'static str` for SQLite TEXT storage, and implements `Display`.
- **`src-tauri/src/store/migrations/001_initial.sql`** — Phase 1 SQL schema embedded via `include_str!`. Declares `unlock_history` (id/app_id/ach_api_name/source/unlocked_at/session_id/notified), `sessions` (session_id/app_id/started_at/ended_at), and `settings` (key/value) tables. Two non-unique indexes (`idx_unlock_session`, `idx_unlock_app`) plus the critical `CREATE UNIQUE INDEX idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)` — REQ DETECT-07's belt-and-suspenders second line of defence. All statements use `IF NOT EXISTS` so the migration is idempotent on every `open()`.
- **`src-tauri/src/store/mod.rs`** — Replaces the Plan 01-01 doc-only stub. `pub struct SqliteStore` wraps `Mutex<Connection>` (the field is `pub(super)`-visible so query helpers in `queries.rs` can borrow it). `open(path)` and `open_in_memory()` both run the embedded migration via `execute_batch`. `record_unlock(app_id, ach_api_name, source, session_id)` issues an `INSERT OR IGNORE` and returns `Ok(true)` on insert, `Ok(false)` on UNIQUE collision — no error on dedup. `count_unlocks()` is a tiny diagnostic helper used by tests and Plan 05's CLI logging.
- **`src-tauri/src/store/queries.rs`** — Four typed query helpers: `create_session(conn, session_id, app_id)` (Phase 1 always passes `app_id = None`; game-launch detection is Phase 2), `end_session(conn, session_id)` (sets `ended_at`), `mark_notified(conn, app_id, ach_api_name, session_id)` (Phase 2 popup queue will call this), and `unlock_count_for_session(conn, session_id)` (Plan 05 diagnostic logging).
- **10 unit tests, all passing.** Sources: `source_kind_as_str_is_stable_lowercase`, `raw_unlock_event_eq_ignores_timestamp_only_for_clone`. Store: `open_creates_schema_idempotently`, `record_unlock_inserts_first_call_returns_true`, `record_unlock_dedup_via_unique_index`, `record_unlock_different_session_succeeds`, `record_unlock_null_session_treated_as_distinct`. Queries: `create_and_end_session_roundtrip`, `unlock_count_for_session_filters_correctly`, `mark_notified_updates_only_matching_row`.

## Key Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|-------------------------|
| `SourceAdapter` trait drops `start()` | WatcherCore (Plan 04) centralizes the notify-debouncer-full event loop so debounce is uniform across all adapters; per-adapter watchers would fragment the 500ms debounce. | Per-adapter `start()` method (ARCHITECTURE.md original) — discarded because uniform debounce is non-negotiable for REQ DETECT-06. |
| `SourceKind::as_str()` returns stable lowercase strings | The string is persisted as `unlock_history.source` TEXT — schema migrations would break if these change. | Storing the enum tag as INTEGER — discarded; TEXT is human-readable in SQL inspections and survives enum reordering. |
| `SqliteStore.conn` is `pub(super)`, not `pub` | Query helpers in `queries.rs` and unit tests need to borrow the `Connection`; external crates do not. `pub(super)` is the minimum visibility that satisfies both. | `pub` (over-exposure) or wrapping every query inside `SqliteStore` impl methods (would balloon `mod.rs`). |
| UNIQUE INDEX `idx_unlock_dedup` on (app_id, ach_api_name, session_id) | REQ DETECT-07 requires belt-and-suspenders dedup. Plan 05's in-memory TTL is the primary mechanism; the UNIQUE INDEX catches edge cases (process restart mid-session, race between two adapters). | Skipping the UNIQUE INDEX and relying solely on Plan 05's in-memory dedup — discarded because process restart drops in-memory state. |
| Document SQLite NULL-distinct UNIQUE-INDEX semantics in a test | SQLite treats `NULL` as distinct from `NULL` in UNIQUE indexes — two NULL session_ids both insert. Plan 05 always passes `Some(_)`, but the test exists as future-regression armor in case a downstream caller passes `None`. | Adding a `COALESCE(session_id, '')` to the index — discarded because Phase 1 callers don't trigger the edge case, and changing the index semantics later is a migration. |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `pub mod queries;` declaration deferred from Task 2 to Task 3**
- **Found during:** Task 2, drafting `store/mod.rs` per the plan's verbatim content.
- **Issue:** The plan's verbatim `store/mod.rs` for Task 2 includes `pub mod queries;` at the top, but `queries.rs` is not created until Task 3 — this would have caused Task 2's commit to fail compile (`file not found for module queries`).
- **Fix:** Removed `pub mod queries;` from Task 2's `mod.rs` (replacing it with a one-line module-level comment explaining the deferral) and added the declaration in Task 3's edit, alongside the creation of `queries.rs`. Each task's commit is independently buildable. The `pub(super)` visibility on `conn` was kept in Task 2 (no harm — it does not require `queries.rs` to compile, and saves a trivial second edit in Task 3). Net behaviour is identical to the plan; the edit was split across the same two commits the plan intended.
- **Files modified:** `src-tauri/src/store/mod.rs`
- **Commits:** Task 2 (`6bbd488`), Task 3 (`805dafa`)

**2. [Rule 1 - Cleanup] `cargo fmt` reformatted `record_unlock` chained calls**
- **Found during:** End-of-plan verification (`cargo fmt --check`).
- **Issue:** Several `s.record_unlock(...).unwrap()` and `conn.query_row(...).unwrap()` chains in test bodies exceeded 100 columns; rustfmt wraps them onto multiple lines.
- **Fix:** Ran `cargo fmt` once. Diff is purely cosmetic (line wraps); no logic changed. Tests still pass identically.
- **Files modified:** `src-tauri/src/store/mod.rs`, `src-tauri/src/store/queries.rs`
- **Commit:** `805dafa` (folded into Task 3 since fmt was run after Task 3 was authored).

### Authentication Gates

None occurred during this plan.

## What Plans 03–05 Need to Fill

| Plan | Module | What it owns | Dependencies on this plan |
|------|--------|--------------|---------------------------|
| 03 | `src-tauri/src/paths.rs` | Steam install registry probe, `libraryfolders.vdf`, `local_save.txt` redirects | None — paths is independent. |
| 04 | `src-tauri/src/sources/goldberg.rs` (new) | Goldberg adapter implementing `SourceAdapter` | Imports `SourceAdapter`, `RawUnlockEvent`, `SourceKind::Goldberg` from `sources/mod.rs`. |
| 04 | `src-tauri/src/watcher/mod.rs` | WatcherCore + notify-debouncer-full driver | Calls `SqliteStore::record_unlock` for second-line-of-defence dedup. |
| 05 | `src-tauri/src/watcher/dedup.rs` (new) | Cross-source dedup TTL stage | Imports `RawUnlockEvent` and `SourceKind`; uses UNIQUE INDEX `idx_unlock_dedup` as fall-back. |
| 05 | `src-tauri/src/bin/hallmark-cli.rs` (new) | CLI test harness | Calls `SqliteStore::open`, `queries::create_session`, `queries::end_session`, `queries::unlock_count_for_session`. |

## Threat Flags

No new threat surface beyond what is already in the plan's `<threat_model>`. T-02-T2 (SQL injection) is mitigated as designed: every `execute`/`query_row` in this plan uses `params![...]` parameter binding — no string concatenation occurs anywhere in `store/mod.rs` or `store/queries.rs`. T-02-T1 and T-02-D1 remain `accept` as planned (local-only single-user app).

## Verification Output

```
$ cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.77s

$ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
(no output — clean)

$ cargo test --manifest-path src-tauri/Cargo.toml --lib sources
running 2 tests
test sources::tests::source_kind_as_str_is_stable_lowercase ... ok
test sources::tests::raw_unlock_event_eq_ignores_timestamp_only_for_clone ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out

$ cargo test --manifest-path src-tauri/Cargo.toml --lib store
running 8 tests
test store::tests::record_unlock_inserts_first_call_returns_true ... ok
test store::tests::record_unlock_dedup_via_unique_index ... ok
test store::tests::record_unlock_null_session_treated_as_distinct ... ok
test store::tests::record_unlock_different_session_succeeds ... ok
test store::queries::tests::unlock_count_for_session_filters_correctly ... ok
test store::queries::tests::create_and_end_session_roundtrip ... ok
test store::tests::open_creates_schema_idempotently ... ok
test store::queries::tests::mark_notified_updates_only_matching_row ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
```

All four verification commands exit 0; 10 unit tests pass.

## Self-Check: PASSED

- `src-tauri/src/sources/mod.rs` exists, contains `pub trait SourceAdapter`, `pub struct RawUnlockEvent`, `pub enum SourceKind`, `#[async_trait::async_trait]`, and the five trait method signatures.
- `src-tauri/src/store/migrations/001_initial.sql` exists, contains `CREATE TABLE IF NOT EXISTS unlock_history`, `sessions`, `settings`, and `CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup`.
- `src-tauri/src/store/mod.rs` exists, contains `pub struct SqliteStore`, `pub mod queries;`, `pub(super) conn: Mutex<Connection>`, `INSERT OR IGNORE INTO unlock_history`, and `include_str!("migrations/001_initial.sql")`.
- `src-tauri/src/store/queries.rs` exists, contains `pub fn create_session`, `pub fn end_session`, `pub fn mark_notified`, `pub fn unlock_count_for_session`.
- Commits exist on master: `b77c2d0` (Task 1 — SourceAdapter trait), `6bbd488` (Task 2 — SqliteStore + migration), `805dafa` (Task 3 — queries.rs) — verified via `git log --oneline`.
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` returns exit 0 with no warnings.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` returns exit 0 (clean).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` runs 10 tests and all pass.

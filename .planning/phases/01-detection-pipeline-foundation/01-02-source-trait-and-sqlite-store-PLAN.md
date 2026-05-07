---
phase: 01-detection-pipeline-foundation
plan: 02
type: execute
wave: 2
depends_on: [01-01]
files_modified:
  - src-tauri/src/sources/mod.rs
  - src-tauri/src/store/mod.rs
  - src-tauri/src/store/queries.rs
  - src-tauri/src/store/migrations/001_initial.sql
autonomous: true
requirements: [DETECT-01, DETECT-07]
must_haves:
  truths:
    - "`SourceAdapter` async trait is defined with `name()`, `kind()`, `watch_paths()`, `seed_baseline()`, `on_file_changed()` methods"
    - "`RawUnlockEvent` struct exists with `app_id: u64`, `ach_api_name: String`, `timestamp: u64`, `source: SourceKind` fields"
    - "`SourceKind` enum includes `Goldberg` variant and is `#[derive(Clone, PartialEq, Eq, Hash)]`"
    - "`SqliteStore::open(path)` creates `unlock_history`, `sessions`, `settings` tables idempotently from `001_initial.sql`"
    - "`SqliteStore::record_unlock(app_id, ach_api_name, source, session_id)` returns `Ok(true)` on insert and `Ok(false)` on dedup-rejection (UNIQUE INDEX collision)"
    - "Unit tests in `store/mod.rs` prove the dedup UNIQUE INDEX works: same (app_id, ach_api_name, session_id) inserts twice → second returns `Ok(false)`"
  artifacts:
    - path: "src-tauri/src/sources/mod.rs"
      provides: "SourceAdapter trait, RawUnlockEvent, SourceKind"
      min_lines: 40
      contains: 'trait SourceAdapter'
    - path: "src-tauri/src/store/mod.rs"
      provides: "SqliteStore struct, open(), record_unlock()"
      min_lines: 80
      contains: 'pub struct SqliteStore'
    - path: "src-tauri/src/store/migrations/001_initial.sql"
      provides: "Phase 1 SQL schema: unlock_history, sessions, settings tables + unique dedup index"
      contains: 'CREATE UNIQUE INDEX'
    - path: "src-tauri/src/store/queries.rs"
      provides: "Typed query helpers: insert_unlock, mark_notified, current_session"
      contains: 'pub fn'
  key_links:
    - from: "src-tauri/src/store/mod.rs"
      to: "src-tauri/src/store/migrations/001_initial.sql"
      via: "include_str! at compile time"
      pattern: 'include_str!\("migrations/001_initial.sql"\)'
    - from: "src-tauri/src/store/mod.rs"
      to: "src-tauri/src/store/queries.rs"
      via: "module declaration"
      pattern: 'pub mod queries'
    - from: "src-tauri/src/store/mod.rs"
      to: "src-tauri/src/error.rs"
      via: "StoreError import"
      pattern: 'crate::error::StoreError'
---

<objective>
Define the load-bearing TYPE contracts that Plans 03 (path discovery), 04 (Goldberg adapter + watcher), and 05 (dedup + CLI) all build against: the `SourceAdapter` async trait, the `RawUnlockEvent` / `SourceKind` event types, and the `SqliteStore` with its `001_initial.sql` migration. This plan is interface-first by design — locking shapes here prevents downstream churn.

Purpose: Plan 04 cannot define `GoldbergAdapter` without the `SourceAdapter` trait. Plan 05's cross-source dedup needs `RawUnlockEvent` to dedup against. Both downstream plans need `SqliteStore::record_unlock()` to persist events and a UNIQUE INDEX for belt-and-suspenders dedup (REQ DETECT-07 second line of defence per RESEARCH.md). Doing this in a dedicated wave-2 plan parallel to path discovery keeps the dependency graph crisp.

Output:
- `sources/mod.rs` with the `SourceAdapter` async trait + event types (no concrete adapter — Plan 04 adds `goldberg.rs`)
- `store/mod.rs`, `store/queries.rs`, `store/migrations/001_initial.sql` with the persistence layer fully implemented and unit-tested
- Three unit tests in `store/mod.rs` proving open/insert/dedup behaviour against an in-memory SQLite database
</objective>

<execution_context>
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md
@.planning/research/ARCHITECTURE.md
@CLAUDE.md

<interfaces>
<!-- The contracts this plan locks in. Plans 03, 04, 05 import from these directly — do NOT change shapes after this plan ships. -->

`SourceAdapter` trait shape (from RESEARCH.md "Pattern 1"):
```rust
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;
    fn watch_paths(&self) -> Vec<PathBuf>;
    async fn seed_baseline(&self) -> anyhow::Result<()>;
    async fn on_file_changed(&self, path: PathBuf, tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>;
}
```

`RawUnlockEvent` shape:
```rust
#[derive(Debug, Clone)]
pub struct RawUnlockEvent {
    pub app_id: u64,
    pub ach_api_name: String,
    pub timestamp: u64,        // 0 if source did not record; downstream stamps wall clock
    pub source: SourceKind,
}
```

`SourceKind` shape (Phase 1 has only Goldberg; Phase 3 will add SteamLegit, CreamApi, SmartSteamEmu):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Goldberg,
    // Phase 3: SteamLegit, CreamApi, SmartSteamEmu
    // Future: Community(String)
}
impl SourceKind {
    pub fn as_str(&self) -> &'static str { /* "goldberg" */ }
}
```

SQL schema shape (verbatim from RESEARCH.md "SQLite migration + insert"):
```sql
CREATE TABLE IF NOT EXISTS unlock_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id INTEGER NOT NULL,
    ach_api_name TEXT NOT NULL,
    source TEXT NOT NULL,
    unlocked_at INTEGER NOT NULL,
    session_id TEXT,
    notified INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_unlock_session ON unlock_history(session_id);
CREATE INDEX IF NOT EXISTS idx_unlock_app ON unlock_history(app_id, ach_api_name);
CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id);
CREATE TABLE IF NOT EXISTS sessions (...);
CREATE TABLE IF NOT EXISTS settings (...);
```

`SqliteStore` API the rest of the codebase uses:
```rust
impl SqliteStore {
    pub fn open(db_path: &Path) -> anyhow::Result<Self>;
    pub fn open_in_memory() -> anyhow::Result<Self>;  // testing convenience
    pub fn record_unlock(&self, app_id: u64, ach_api_name: &str, source: &str, session_id: Option<&str>) -> anyhow::Result<bool>;
    pub fn count_unlocks(&self) -> anyhow::Result<i64>;  // testing convenience
}
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Define SourceAdapter trait + RawUnlockEvent + SourceKind</name>
  <files>
    - src-tauri/src/sources/mod.rs
  </files>
  <read_first>
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md (sections: "Pattern 1: Adapter trait", code block in "Goldberg state file parse + diff" — confirms trait method signatures the GoldbergAdapter needs in Plan 04)
    - .planning/research/ARCHITECTURE.md (section: "Source Adapter Interface" — original spec; the Phase 1 trait drops `start()` because notify-debouncer-full owns the event loop, not the adapter)
    - src-tauri/src/lib.rs (confirms `pub mod sources;` declaration exists)
  </read_first>
  <action>
    Write the COMPLETE contents of `src-tauri/src/sources/mod.rs`. This file is verbatim from RESEARCH.md Pattern 1 with three modifications: (a) the trait drops the `start()` method because Plan 04's WatcherCore owns the event loop via notify-debouncer-full (not the adapter), (b) `SourceKind::as_str()` is added because the SQLite store stores source as TEXT, (c) full doc-comments because Plans 03/04/05 read this file as their interface contract.

    Verbatim file content:

    ```rust
    //! Source adapter trait + event types — the contract every detection source implements.
    //!
    //! # Phase 1 scope
    //!
    //! Only Goldberg is implemented (Plan 04 — `goldberg.rs`). Phase 3 adds Steam-legit
    //! (binary VDF), CreamAPI, and SmartSteamEmu by adding additional `pub mod`
    //! declarations here and new `SourceKind` variants below.
    //!
    //! # Why no `start()` method on the trait
    //!
    //! ARCHITECTURE.md's original spec had each adapter own its own watcher. In Phase 1
    //! we centralized to a single `notify-debouncer-full` instance in `WatcherCore`
    //! (Plan 04) so the 500ms debounce is uniform across all adapters. Adapters now only
    //! declare the paths they care about (`watch_paths()`) and react to events on those
    //! paths (`on_file_changed()`). The trait is intentionally smaller as a result.
    //!
    //! # Why `Send + Sync + 'static`
    //!
    //! Adapters are owned by `Arc<dyn SourceAdapter>` and shared with tokio tasks; all
    //! three bounds are required for that. `'static` is satisfied because adapters carry
    //! no borrowed data — only owned `Vec<PathBuf>` and `Arc<RwLock<...>>` interiors.

    use std::path::PathBuf;
    use tokio::sync::mpsc;

    /// A raw unlock event emitted by a source adapter before any cross-source dedup or
    /// schema enrichment. Phase 1's pipeline emits these directly to stdout via the CLI
    /// test harness (Plan 05). Phase 2 will route them through schema resolution and
    /// the popup queue.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RawUnlockEvent {
        /// Steam app ID. Parsed from the `<appid>/achievements.json` directory name.
        pub app_id: u64,
        /// Steam achievement API name (e.g. `ACH_WIN_ONE_GAME`). NOT the human display name.
        pub ach_api_name: String,
        /// Source-reported unix timestamp in seconds. May be `0` when the source did not
        /// record a time (Goldberg's `earned_time = 0` for "earned but timestamp unknown",
        /// per PITFALLS.md #15). DO NOT use `timestamp > 0` as a freshness signal —
        /// the boolean `earned` transition false→true is the only valid unlock signal.
        pub timestamp: u64,
        /// Which adapter produced this event. Used for dedup, logging, and future UI badges.
        pub source: SourceKind,
    }

    /// Identifies which adapter produced an event. The `as_str()` form is what gets
    /// stored in the `unlock_history.source` SQLite column.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum SourceKind {
        /// Goldberg / gbe_fork emulator. Watches `%APPDATA%\Goldberg SteamEmu Saves\`,
        /// `%APPDATA%\GSE Saves\`, and `local_save.txt` redirects.
        Goldberg,
        // Phase 3 will add: SteamLegit, CreamApi, SmartSteamEmu
        // Future community plug-ins will add: Community(String)
    }

    impl SourceKind {
        /// Stable string representation for SQLite TEXT storage and log spans.
        /// Stable across versions — schema migrations rely on this string being lossless.
        pub fn as_str(&self) -> &'static str {
            match self {
                SourceKind::Goldberg => "goldberg",
            }
        }
    }

    impl std::fmt::Display for SourceKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.as_str())
        }
    }

    /// Trait every source adapter implements. Plan 04 provides the only Phase 1
    /// implementation (`goldberg::GoldbergAdapter`). The shape is locked here; adding
    /// new methods is a breaking change to all adapters.
    #[async_trait::async_trait]
    pub trait SourceAdapter: Send + Sync + 'static {
        /// Human-readable adapter name for log spans (`"goldberg"`, etc.).
        fn name(&self) -> &str;

        /// Stable enum identifier — what gets persisted as `unlock_history.source`.
        fn kind(&self) -> SourceKind;

        /// Filesystem roots this adapter watches (recursively). `WatcherCore` registers
        /// each path with the shared notify-debouncer-full instance and dispatches events
        /// back to the matching adapter via prefix-match.
        ///
        /// Adapter MUST filter out non-existent paths before returning — `notify::Watcher::watch`
        /// errors `PathNotFound` for missing dirs (PITFALLS.md, RESEARCH.md Pitfall #5).
        fn watch_paths(&self) -> Vec<PathBuf>;

        /// Read all current state files and populate the adapter's in-memory baseline
        /// (`HashMap<(appid, ach_api_name), bool>`). MUST run BEFORE the watcher attaches —
        /// `WatcherCore` enforces this ordering.
        ///
        /// Implements REQ DETECT-05 (no spam of historic unlocks on first run).
        async fn seed_baseline(&self) -> anyhow::Result<()>;

        /// Called by `WatcherCore` when a debounced file event lands on a path returned
        /// by this adapter's `watch_paths()`. Adapter parses the file, diffs against the
        /// baseline, and emits `RawUnlockEvent`s for any `false → true` transitions.
        ///
        /// Adapter is responsible for the per-file content-hash check (REQ DETECT-06,
        /// content-hash equality layer); the 500ms debounce is handled centrally.
        async fn on_file_changed(
            &self,
            path: PathBuf,
            tx: mpsc::Sender<RawUnlockEvent>,
        ) -> anyhow::Result<()>;
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn source_kind_as_str_is_stable_lowercase() {
            assert_eq!(SourceKind::Goldberg.as_str(), "goldberg");
            assert_eq!(SourceKind::Goldberg.to_string(), "goldberg");
        }

        #[test]
        fn raw_unlock_event_eq_ignores_timestamp_only_for_clone() {
            // Two events with same fields are equal (PartialEq derived).
            let a = RawUnlockEvent { app_id: 480, ach_api_name: "ACH_X".into(), timestamp: 0, source: SourceKind::Goldberg };
            let b = a.clone();
            assert_eq!(a, b);
            // Differing timestamp DOES matter — derived Eq is field-by-field.
            let c = RawUnlockEvent { timestamp: 1, ..a.clone() };
            assert_ne!(a, c);
        }
    }
    ```

    After writing the file, run `cargo check --manifest-path src-tauri/Cargo.toml --all-targets`. Compile must succeed.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "$f = 'src-tauri/src/sources/mod.rs'; if (-not (Test-Path $f)) { exit 1 }; $c = Get-Content $f -Raw; if ($c -notmatch 'pub trait SourceAdapter') { exit 10 }; if ($c -notmatch 'pub struct RawUnlockEvent') { exit 11 }; if ($c -notmatch 'pub enum SourceKind') { exit 12 }; if ($c -notmatch 'fn name\(&self\) -> &str') { exit 13 }; if ($c -notmatch 'fn kind\(&self\) -> SourceKind') { exit 14 }; if ($c -notmatch 'fn watch_paths\(&self\) -> Vec<PathBuf>') { exit 15 }; if ($c -notmatch 'async fn seed_baseline') { exit 16 }; if ($c -notmatch 'async fn on_file_changed') { exit 17 }; if ($c -notmatch '#\[async_trait::async_trait\]') { exit 18 }; if ($c -notmatch 'SourceKind::Goldberg') { exit 19 }; if ($c -notmatch 'pub app_id: u64') { exit 20 }; if ($c -notmatch 'pub ach_api_name: String') { exit 21 }; if ($c -notmatch 'pub timestamp: u64') { exit 22 }; if ($c -notmatch 'pub source: SourceKind') { exit 23 }; cargo check --manifest-path src-tauri/Cargo.toml --all-targets 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; cargo test --manifest-path src-tauri/Cargo.toml --lib sources::tests -- --nocapture 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 40 }; Write-Host 'sources/mod.rs OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/src/sources/mod.rs` exists.
    - File contains `pub trait SourceAdapter: Send + Sync + 'static`.
    - File contains `#[async_trait::async_trait]` immediately above the trait.
    - Trait declares all 5 methods with EXACT signatures: `fn name(&self) -> &str`, `fn kind(&self) -> SourceKind`, `fn watch_paths(&self) -> Vec<PathBuf>`, `async fn seed_baseline(&self) -> anyhow::Result<()>`, `async fn on_file_changed(&self, path: PathBuf, tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>`.
    - File contains `pub struct RawUnlockEvent` with public fields `app_id: u64`, `ach_api_name: String`, `timestamp: u64`, `source: SourceKind`.
    - File contains `pub enum SourceKind` with the `Goldberg` variant.
    - `SourceKind` derives `Debug, Clone, Copy, PartialEq, Eq, Hash`.
    - `SourceKind::as_str()` returns `"goldberg"` for the `Goldberg` variant (verified by unit test).
    - `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` exits 0.
    - `cargo test --manifest-path src-tauri/Cargo.toml --lib sources::tests` exits 0; both tests `source_kind_as_str_is_stable_lowercase` and `raw_unlock_event_eq_ignores_timestamp_only_for_clone` pass.
  </acceptance_criteria>
  <done>The `SourceAdapter` trait is locked in. Plan 04 (Goldberg adapter) and Plan 05 (dedup) can both `use crate::sources::{SourceAdapter, RawUnlockEvent, SourceKind}` and trust the contract is stable for the rest of Phase 1.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Write 001_initial.sql migration + SqliteStore::open() with embedded migration</name>
  <files>
    - src-tauri/src/store/migrations/001_initial.sql
    - src-tauri/src/store/mod.rs
  </files>
  <read_first>
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md (section: "SQLite migration + insert" — provides the SQL verbatim and the open() pattern)
    - .planning/research/ARCHITECTURE.md (section: "Storage Shape" — the full Phase 2/3 schema for context; Phase 1 implements only `unlock_history`, `sessions`, `settings`)
    - src-tauri/src/error.rs (confirms `StoreError` enum exists from Plan 01)
    - src-tauri/src/sources/mod.rs (just-created — confirms `SourceKind::as_str()` returns the string we'll store in `source` column)
  </read_first>
  <behavior>
    Tests proving the store layer:
    - Test 1 (`open_creates_schema_idempotently`): Calling `SqliteStore::open_in_memory()` twice on separate connections both succeed; the second one finds tables already present (`CREATE TABLE IF NOT EXISTS` semantics).
    - Test 2 (`record_unlock_inserts_first_call_returns_true`): Fresh store, `record_unlock(480, "ACH_X", "goldberg", Some("session-1"))` returns `Ok(true)`; `count_unlocks()` returns 1.
    - Test 3 (`record_unlock_dedup_via_unique_index`): After test 2's call, calling `record_unlock(480, "ACH_X", "goldberg", Some("session-1"))` AGAIN returns `Ok(false)` (UNIQUE INDEX collision via `INSERT OR IGNORE`); `count_unlocks()` is still 1.
    - Test 4 (`record_unlock_different_session_succeeds`): Same `(app_id, ach_api_name)` but different `session_id` succeeds — proves the UNIQUE INDEX is correctly composite on three columns. `count_unlocks()` is 2.
    - Test 5 (`record_unlock_null_session_treated_correctly`): Two `record_unlock` calls with `session_id = None` for the same `(app_id, ach_api_name)` — SQLite's UNIQUE INDEX treats NULL as distinct from itself, so BOTH inserts succeed. This is documented behaviour and acceptable for Phase 1 (Plan 05 always passes `Some(session_id)`); the test asserts the documented behaviour so future regressions surface.
  </behavior>
  <action>
    Step 1 — Create `src-tauri/src/store/migrations/001_initial.sql`. Verbatim from RESEARCH.md "SQLite migration + insert":
    ```sql
    -- Phase 1 schema: unlock detection persistence.
    -- Phase 2 will add schema_cache + icon_cache; Phase 3 may extend sessions.
    -- This file is loaded via include_str! at compile time and applied idempotently
    -- on every SqliteStore::open(). All statements use IF NOT EXISTS for restart safety.

    CREATE TABLE IF NOT EXISTS unlock_history (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        app_id        INTEGER NOT NULL,
        ach_api_name  TEXT    NOT NULL,
        source        TEXT    NOT NULL,
        unlocked_at   INTEGER NOT NULL,
        session_id    TEXT,
        notified      INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_unlock_session ON unlock_history(session_id);
    CREATE INDEX IF NOT EXISTS idx_unlock_app     ON unlock_history(app_id, ach_api_name);
    -- Belt-and-suspenders dedup: cross-source dedup TTL (Plan 05) is the primary
    -- mechanism; this UNIQUE INDEX catches anything the in-memory dedup misses
    -- (e.g. process restart mid-session). REQ DETECT-07.
    -- NOTE: SQLite treats NULL as distinct from NULL in UNIQUE INDEX, so a NULL
    -- session_id will not collide. Production code (Plan 05) always passes Some(_).
    CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup
        ON unlock_history(app_id, ach_api_name, session_id);

    CREATE TABLE IF NOT EXISTS sessions (
        session_id    TEXT    PRIMARY KEY,
        app_id        INTEGER,
        started_at    INTEGER NOT NULL,
        ended_at      INTEGER
    );

    CREATE TABLE IF NOT EXISTS settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    ```

    Step 2 — Create `src-tauri/src/store/mod.rs`. Verbatim:
    ```rust
    //! SQLite-backed persistence layer.
    //!
    //! Phase 1 scope: `unlock_history`, `sessions`, `settings` tables. Single connection
    //! wrapped in `Mutex` is sufficient — desktop app, single process, low contention.
    //! Phase 2 will add schema_cache + icon_cache tables; Phase 3 may extend sessions.

    pub mod queries;

    use rusqlite::{params, Connection};
    use std::path::Path;
    use std::sync::Mutex;

    /// Embedded migration SQL — applied idempotently on every `open()` call thanks to
    /// `CREATE ... IF NOT EXISTS`. Single file is sufficient for Phase 1; Phase 2+
    /// can add `002_*.sql` and a numbered loader if multiple migrations stack up.
    const INITIAL_MIGRATION_SQL: &str = include_str!("migrations/001_initial.sql");

    /// SQLite-backed persistence handle. Cheap to clone via `Arc` from the caller.
    pub struct SqliteStore {
        conn: Mutex<Connection>,
    }

    impl SqliteStore {
        /// Open a SQLite database at the given path. Creates the file if absent and
        /// applies the initial schema (idempotent — safe to call on existing DBs).
        pub fn open(db_path: &Path) -> anyhow::Result<Self> {
            let conn = Connection::open(db_path)?;
            conn.execute_batch(INITIAL_MIGRATION_SQL)?;
            Ok(Self { conn: Mutex::new(conn) })
        }

        /// In-memory SQLite database for tests. Same schema, no disk persistence.
        pub fn open_in_memory() -> anyhow::Result<Self> {
            let conn = Connection::open_in_memory()?;
            conn.execute_batch(INITIAL_MIGRATION_SQL)?;
            Ok(Self { conn: Mutex::new(conn) })
        }

        /// Insert an unlock event. Returns `Ok(true)` if a new row was inserted, or
        /// `Ok(false)` if the row was deduplicated by the UNIQUE INDEX on
        /// `(app_id, ach_api_name, session_id)`. Uses `INSERT OR IGNORE` so a dedup
        /// collision is not an error.
        ///
        /// `session_id = None` will always insert (NULL is distinct from itself in
        /// SQLite UNIQUE INDEX); production callers (Plan 05) always pass `Some(_)`.
        pub fn record_unlock(
            &self,
            app_id: u64,
            ach_api_name: &str,
            source: &str,
            session_id: Option<&str>,
        ) -> anyhow::Result<bool> {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;
            let conn = self.conn.lock().unwrap();
            let rows_changed = conn.execute(
                "INSERT OR IGNORE INTO unlock_history
                    (app_id, ach_api_name, source, unlocked_at, session_id, notified)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                params![app_id as i64, ach_api_name, source, now, session_id],
            )?;
            Ok(rows_changed == 1)
        }

        /// Count rows in `unlock_history`. For tests + diagnostic logging.
        pub fn count_unlocks(&self) -> anyhow::Result<i64> {
            let conn = self.conn.lock().unwrap();
            let n: i64 = conn.query_row(
                "SELECT COUNT(*) FROM unlock_history",
                [],
                |row| row.get(0),
            )?;
            Ok(n)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn open_creates_schema_idempotently() {
            let s1 = SqliteStore::open_in_memory().unwrap();
            assert_eq!(s1.count_unlocks().unwrap(), 0);
            // Re-running migrations on the same connection is also idempotent
            {
                let conn = s1.conn.lock().unwrap();
                conn.execute_batch(INITIAL_MIGRATION_SQL).unwrap();
            }
            assert_eq!(s1.count_unlocks().unwrap(), 0);
        }

        #[test]
        fn record_unlock_inserts_first_call_returns_true() {
            let s = SqliteStore::open_in_memory().unwrap();
            let inserted = s.record_unlock(480, "ACH_X", "goldberg", Some("session-1")).unwrap();
            assert!(inserted, "first insert should report Ok(true)");
            assert_eq!(s.count_unlocks().unwrap(), 1);
        }

        #[test]
        fn record_unlock_dedup_via_unique_index() {
            let s = SqliteStore::open_in_memory().unwrap();
            assert!(s.record_unlock(480, "ACH_X", "goldberg", Some("session-1")).unwrap());
            // Same triplet again — UNIQUE INDEX collision; INSERT OR IGNORE returns 0 rows.
            let inserted_again = s.record_unlock(480, "ACH_X", "goldberg", Some("session-1")).unwrap();
            assert!(!inserted_again, "duplicate insert should report Ok(false)");
            assert_eq!(s.count_unlocks().unwrap(), 1, "no new row should exist");
        }

        #[test]
        fn record_unlock_different_session_succeeds() {
            let s = SqliteStore::open_in_memory().unwrap();
            assert!(s.record_unlock(480, "ACH_X", "goldberg", Some("session-1")).unwrap());
            // Same (app_id, ach_api_name) but different session — composite UNIQUE allows this.
            assert!(s.record_unlock(480, "ACH_X", "goldberg", Some("session-2")).unwrap());
            assert_eq!(s.count_unlocks().unwrap(), 2);
        }

        #[test]
        fn record_unlock_null_session_treated_as_distinct() {
            // Documented SQLite behavior: NULL is distinct from NULL in UNIQUE INDEX.
            // Plan 05 always passes Some(_) so this only matters as future-regression armor.
            let s = SqliteStore::open_in_memory().unwrap();
            assert!(s.record_unlock(480, "ACH_X", "goldberg", None).unwrap());
            assert!(s.record_unlock(480, "ACH_X", "goldberg", None).unwrap());
            assert_eq!(s.count_unlocks().unwrap(), 2,
                "two NULL session_ids should both insert (SQLite NULL semantics)");
        }
    }
    ```

    Step 3 — Run the tests:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --lib store::tests -- --nocapture
    ```
    All 5 tests must pass.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/store/migrations/001_initial.sql)) { exit 1 }; if (-not (Test-Path src-tauri/src/store/mod.rs)) { exit 2 }; $sql = Get-Content src-tauri/src/store/migrations/001_initial.sql -Raw; if ($sql -notmatch 'CREATE TABLE IF NOT EXISTS unlock_history') { exit 10 }; if ($sql -notmatch 'CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup') { exit 11 }; if ($sql -notmatch 'CREATE TABLE IF NOT EXISTS sessions') { exit 12 }; if ($sql -notmatch 'CREATE TABLE IF NOT EXISTS settings') { exit 13 }; $code = Get-Content src-tauri/src/store/mod.rs -Raw; if ($code -notmatch 'pub struct SqliteStore') { exit 20 }; if ($code -notmatch 'pub fn open\(') { exit 21 }; if ($code -notmatch 'pub fn open_in_memory\(\)') { exit 22 }; if ($code -notmatch 'pub fn record_unlock\(') { exit 23 }; if ($code -notmatch 'INSERT OR IGNORE INTO unlock_history') { exit 24 }; if ($code -notmatch 'include_str!\("migrations/001_initial.sql"\)') { exit 25 }; cargo test --manifest-path src-tauri/Cargo.toml --lib store::tests 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; Write-Host 'store OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/src/store/migrations/001_initial.sql` exists.
    - SQL contains all three table creations: `unlock_history`, `sessions`, `settings`.
    - SQL contains `CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)` (the belt-and-suspenders REQ DETECT-07 second-line-of-defence).
    - File `src-tauri/src/store/mod.rs` exists.
    - File contains `pub struct SqliteStore` with public methods `open`, `open_in_memory`, `record_unlock`, `count_unlocks`.
    - File contains `include_str!("migrations/001_initial.sql")` (compile-time embedding, not runtime read).
    - File contains `INSERT OR IGNORE INTO unlock_history` (the dedup primitive).
    - `cargo test --manifest-path src-tauri/Cargo.toml --lib store::tests` exits 0; all 5 tests pass: `open_creates_schema_idempotently`, `record_unlock_inserts_first_call_returns_true`, `record_unlock_dedup_via_unique_index`, `record_unlock_different_session_succeeds`, `record_unlock_null_session_treated_as_distinct`.
  </acceptance_criteria>
  <done>SqliteStore is fully implemented and tested with 5 unit tests covering: idempotent migration, fresh insert, UNIQUE INDEX dedup, composite-key correctness, and NULL semantics. Plan 05's CrossSourceDedup will use this store as its second line of defence.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: Add typed query helpers in queries.rs (current session, mark notified)</name>
  <files>
    - src-tauri/src/store/queries.rs
    - src-tauri/src/store/mod.rs
  </files>
  <read_first>
    - src-tauri/src/store/mod.rs (just-created — confirms the `pub mod queries;` declaration is at the top)
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md (section: "Recommended Project Structure" — confirms `queries.rs` is the right file)
    - .planning/research/ARCHITECTURE.md (section: "Storage Shape — Schema" — `sessions` table fields)
  </read_first>
  <action>
    Phase 1 has no game-launch detection (deferred to Phase 2), so the `sessions` table will hold ONE row per Hallmark process lifetime — a "global" session created at startup. Plan 05's CLI test harness creates this session row. This task adds the helper that creates it and the helper that closes it on shutdown, plus a `mark_notified` helper that Phase 2's popup pipeline will use (we add it now to lock the helper-set's shape).

    Verbatim file content for `src-tauri/src/store/queries.rs`:
    ```rust
    //! Typed query helpers for `SqliteStore`. Plan 04 (adapter) and Plan 05 (CLI harness)
    //! call these.
    //!
    //! Why a separate file: keeps `mod.rs` focused on connection lifecycle while
    //! per-table query functions are colocated and easy to find.

    use rusqlite::{params, Connection};

    /// Insert a new session row. Returns the `session_id` for caller convenience.
    /// `app_id = None` is correct for Phase 1 — there is no game-launch detection yet,
    /// so the session is "the Hallmark process lifetime" with no associated app.
    pub fn create_session(
        conn: &Connection,
        session_id: &str,
        app_id: Option<u64>,
    ) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        conn.execute(
            "INSERT INTO sessions (session_id, app_id, started_at, ended_at)
             VALUES (?1, ?2, ?3, NULL)",
            params![session_id, app_id.map(|a| a as i64), now],
        )?;
        Ok(())
    }

    /// Mark a session as ended. Used by Plan 05's shutdown handler.
    pub fn end_session(conn: &Connection, session_id: &str) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        conn.execute(
            "UPDATE sessions SET ended_at = ?1 WHERE session_id = ?2 AND ended_at IS NULL",
            params![now, session_id],
        )?;
        Ok(())
    }

    /// Mark a previously-recorded unlock as having had its popup shown.
    /// Phase 1 always leaves notified=0 (no popups exist yet). Phase 2's popup queue
    /// will call this when a popup completes its animation.
    pub fn mark_notified(
        conn: &Connection,
        app_id: u64,
        ach_api_name: &str,
        session_id: &str,
    ) -> anyhow::Result<()> {
        conn.execute(
            "UPDATE unlock_history SET notified = 1
             WHERE app_id = ?1 AND ach_api_name = ?2 AND session_id = ?3",
            params![app_id as i64, ach_api_name, session_id],
        )?;
        Ok(())
    }

    /// Count unlock events recorded for a given session. Used by Plan 05 for
    /// "X unlocks this session" diagnostic logging.
    pub fn unlock_count_for_session(
        conn: &Connection,
        session_id: &str,
    ) -> anyhow::Result<i64> {
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM unlock_history WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(n)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::store::SqliteStore;

        fn fresh_store() -> SqliteStore { SqliteStore::open_in_memory().unwrap() }

        #[test]
        fn create_and_end_session_roundtrip() {
            let s = fresh_store();
            let conn = s.conn.lock().unwrap();
            create_session(&conn, "test-session-1", None).unwrap();
            end_session(&conn, "test-session-1").unwrap();
            let ended_at: Option<i64> = conn.query_row(
                "SELECT ended_at FROM sessions WHERE session_id = ?1",
                params!["test-session-1"],
                |row| row.get(0),
            ).unwrap();
            assert!(ended_at.is_some(), "ended_at should be set after end_session");
        }

        #[test]
        fn unlock_count_for_session_filters_correctly() {
            let s = fresh_store();
            s.record_unlock(480, "ACH_A", "goldberg", Some("s1")).unwrap();
            s.record_unlock(480, "ACH_B", "goldberg", Some("s1")).unwrap();
            s.record_unlock(480, "ACH_C", "goldberg", Some("s2")).unwrap();
            let conn = s.conn.lock().unwrap();
            assert_eq!(unlock_count_for_session(&conn, "s1").unwrap(), 2);
            assert_eq!(unlock_count_for_session(&conn, "s2").unwrap(), 1);
            assert_eq!(unlock_count_for_session(&conn, "s3").unwrap(), 0);
        }

        #[test]
        fn mark_notified_updates_only_matching_row() {
            let s = fresh_store();
            s.record_unlock(480, "ACH_A", "goldberg", Some("s1")).unwrap();
            s.record_unlock(480, "ACH_B", "goldberg", Some("s1")).unwrap();
            let conn = s.conn.lock().unwrap();
            mark_notified(&conn, 480, "ACH_A", "s1").unwrap();
            let notified_a: i64 = conn.query_row(
                "SELECT notified FROM unlock_history WHERE ach_api_name = ?1",
                params!["ACH_A"],
                |row| row.get(0),
            ).unwrap();
            let notified_b: i64 = conn.query_row(
                "SELECT notified FROM unlock_history WHERE ach_api_name = ?1",
                params!["ACH_B"],
                |row| row.get(0),
            ).unwrap();
            assert_eq!(notified_a, 1);
            assert_eq!(notified_b, 0);
        }
    }
    ```

    The tests need `s.conn` to be visible from the queries::tests submodule. Update `src-tauri/src/store/mod.rs` to make the `conn` field `pub(super)`-visible (so submodules of `store` can borrow it for tests) — but ONLY `pub(super)`, NOT `pub`. Edit the struct definition:
    ```rust
    pub struct SqliteStore {
        pub(super) conn: Mutex<Connection>,
    }
    ```
    This is a controlled visibility relaxation purely for in-crate query helpers and tests. External crates still cannot reach the connection.

    Run:
    ```powershell
    cargo test --manifest-path src-tauri/Cargo.toml --lib store -- --nocapture
    ```
    All 8 tests across `store::tests` and `store::queries::tests` must pass.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/store/queries.rs)) { exit 1 }; $q = Get-Content src-tauri/src/store/queries.rs -Raw; if ($q -notmatch 'pub fn create_session') { exit 10 }; if ($q -notmatch 'pub fn end_session') { exit 11 }; if ($q -notmatch 'pub fn mark_notified') { exit 12 }; if ($q -notmatch 'pub fn unlock_count_for_session') { exit 13 }; $m = Get-Content src-tauri/src/store/mod.rs -Raw; if ($m -notmatch 'pub mod queries;') { exit 20 }; if ($m -notmatch 'pub\(super\) conn: Mutex<Connection>') { exit 21 }; cargo test --manifest-path src-tauri/Cargo.toml --lib store 2>&1 | Out-Host; if ($LASTEXITCODE -ne 0) { exit 30 }; Write-Host 'queries OK'</automated>
  </verify>
  <acceptance_criteria>
    - File `src-tauri/src/store/queries.rs` exists.
    - File contains all 4 public helper functions: `create_session`, `end_session`, `mark_notified`, `unlock_count_for_session`.
    - File `src-tauri/src/store/mod.rs` declares `pub mod queries;` at the top.
    - `SqliteStore.conn` field is `pub(super)`-visible (so queries.rs and its tests can borrow it).
    - `cargo test --manifest-path src-tauri/Cargo.toml --lib store` exits 0; all 8 tests pass (5 from `store::tests` + 3 from `store::queries::tests`).
    - `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` exits 0 with no warnings about unused public functions in `queries.rs` (the test module references each helper, so dead-code lints are silent).
  </acceptance_criteria>
  <done>The SqliteStore + queries module is complete: open/in-memory + record_unlock + count_unlocks + create_session + end_session + mark_notified + unlock_count_for_session, with 8 passing unit tests. Plan 05's CLI harness can `use hallmark_lib::store::{SqliteStore, queries::{create_session, end_session}}` and have everything it needs.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| disk → process (SQLite file) | `hallmark.db` is opened from a user-writable path. The schema is created by us via embedded `INITIAL_MIGRATION_SQL`, but a malicious actor with file-system write access could replace `hallmark.db` with a crafted SQLite file. |
| in-memory args → SQL | `record_unlock(app_id, ach_api_name, source, session_id)` accepts caller-supplied strings that originate from disk-parsed Goldberg JSON (Plan 04). |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-02-T1 | Tampering | hallmark.db on disk | accept | Local-only single-user app; if an attacker has write access to `%APPDATA%\Hallmark\` they already control the app. No security boundary here is meaningful for v1. |
| T-02-T2 | Tampering / SQL injection | record_unlock parameters | mitigate | All SQL uses `params![...]` parameter binding (rusqlite's parameterized API). Strings are NEVER concatenated into SQL. Verified by code inspection — no `format!("INSERT ... '{}'", ...)` patterns exist. |
| T-02-D1 | DoS | Unbounded inserts to unlock_history | accept | Phase 1 has no popup, so `record_unlock` is called only by tests and Plan 05's CLI sink. Realistic upper bound: hundreds of unlocks per session. SQLite handles this trivially. Phase 4 may add an LRU eviction policy. |
| T-02-I1 | Info disclosure | SQLite WAL files (`-shm`, `-wal`) | accept | `.gitignore` (added by Plan 01) excludes them from version control. Local-only stance applies. |
</threat_model>

<verification>
End-of-plan verification:
```powershell
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib sources
cargo test --manifest-path src-tauri/Cargo.toml --lib store
```
All four exit 0. The `cargo test` runs report at minimum 10 tests (2 in sources + 5 in store + 3 in queries).
</verification>

<success_criteria>
- `SourceAdapter` trait is locked: 5 methods (`name`, `kind`, `watch_paths`, `seed_baseline`, `on_file_changed`), `#[async_trait]`, `Send + Sync + 'static`.
- `RawUnlockEvent` and `SourceKind` are defined with the exact field names downstream plans expect (`app_id`, `ach_api_name`, `timestamp`, `source`).
- `SqliteStore` is fully implemented with `open`, `open_in_memory`, `record_unlock`, `count_unlocks`.
- `queries` module provides `create_session`, `end_session`, `mark_notified`, `unlock_count_for_session`.
- All 10+ unit tests pass via `cargo test --lib`.
- The UNIQUE INDEX `idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)` exists in the SQL — REQ DETECT-07's belt-and-suspenders second line of defence is in place.
</success_criteria>

<output>
After completion, create `.planning/phases/01-detection-pipeline-foundation/01-02-SUMMARY.md` documenting:
the SourceAdapter trait shape, RawUnlockEvent/SourceKind enum, SqliteStore API, the 10+ unit tests
that passed, and any divergence from RESEARCH.md (none expected — this plan is a faithful
implementation of RESEARCH.md Pattern 1 + the SQL block).
</output>

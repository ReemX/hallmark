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
    pub(super) conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open a SQLite database at the given path. Creates the file if absent and
    /// applies the initial schema (idempotent — safe to call on existing DBs).
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(INITIAL_MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory SQLite database for tests. Same schema, no disk persistence.
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(INITIAL_MIGRATION_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
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
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM unlock_history", [], |row| row.get(0))?;
        Ok(n)
    }

    /// Run a closure against the underlying connection. Used by the CLI binary and
    /// the `queries` submodule to invoke typed query helpers (e.g.
    /// `queries::create_session`) without exposing the connection mutex publicly.
    ///
    /// The mutex is held for the duration of the closure; keep the closure short.
    pub fn with_conn<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Connection) -> anyhow::Result<T>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
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
        let inserted = s
            .record_unlock(480, "ACH_X", "goldberg", Some("session-1"))
            .unwrap();
        assert!(inserted, "first insert should report Ok(true)");
        assert_eq!(s.count_unlocks().unwrap(), 1);
    }

    #[test]
    fn record_unlock_dedup_via_unique_index() {
        let s = SqliteStore::open_in_memory().unwrap();
        assert!(s
            .record_unlock(480, "ACH_X", "goldberg", Some("session-1"))
            .unwrap());
        // Same triplet again — UNIQUE INDEX collision; INSERT OR IGNORE returns 0 rows.
        let inserted_again = s
            .record_unlock(480, "ACH_X", "goldberg", Some("session-1"))
            .unwrap();
        assert!(!inserted_again, "duplicate insert should report Ok(false)");
        assert_eq!(s.count_unlocks().unwrap(), 1, "no new row should exist");
    }

    #[test]
    fn record_unlock_different_session_succeeds() {
        let s = SqliteStore::open_in_memory().unwrap();
        assert!(s
            .record_unlock(480, "ACH_X", "goldberg", Some("session-1"))
            .unwrap());
        // Same (app_id, ach_api_name) but different session — composite UNIQUE allows this.
        assert!(s
            .record_unlock(480, "ACH_X", "goldberg", Some("session-2"))
            .unwrap());
        assert_eq!(s.count_unlocks().unwrap(), 2);
    }

    #[test]
    fn record_unlock_null_session_treated_as_distinct() {
        // Documented SQLite behavior: NULL is distinct from NULL in UNIQUE INDEX.
        // Plan 05 always passes Some(_) so this only matters as future-regression armor.
        let s = SqliteStore::open_in_memory().unwrap();
        assert!(s.record_unlock(480, "ACH_X", "goldberg", None).unwrap());
        assert!(s.record_unlock(480, "ACH_X", "goldberg", None).unwrap());
        assert_eq!(
            s.count_unlocks().unwrap(),
            2,
            "two NULL session_ids should both insert (SQLite NULL semantics)"
        );
    }
}

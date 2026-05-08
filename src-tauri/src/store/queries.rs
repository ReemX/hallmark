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
    // IN-03: surface u64 → i64 overflow as an error rather than silent wrap.
    // Steam app IDs are 32-bit unsigned today, so this is a forward-compat guard.
    let app_id_i64 = match app_id {
        Some(a) => Some(i64::try_from(a)?),
        None => None,
    };
    conn.execute(
        "INSERT INTO sessions (session_id, app_id, started_at, ended_at)
         VALUES (?1, ?2, ?3, NULL)",
        params![session_id, app_id_i64, now],
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
    // IN-03: surface u64 → i64 overflow as an error rather than silent wrap.
    let app_id_i64 = i64::try_from(app_id)?;
    conn.execute(
        "UPDATE unlock_history SET notified = 1
         WHERE app_id = ?1 AND ach_api_name = ?2 AND session_id = ?3",
        params![app_id_i64, ach_api_name, session_id],
    )?;
    Ok(())
}

/// Count unlock events recorded for a given session. Used by Plan 05 for
/// "X unlocks this session" diagnostic logging.
pub fn unlock_count_for_session(conn: &Connection, session_id: &str) -> anyhow::Result<i64> {
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

    fn fresh_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn create_and_end_session_roundtrip() {
        let s = fresh_store();
        let conn = s.conn.lock().unwrap();
        create_session(&conn, "test-session-1", None).unwrap();
        end_session(&conn, "test-session-1").unwrap();
        let ended_at: Option<i64> = conn
            .query_row(
                "SELECT ended_at FROM sessions WHERE session_id = ?1",
                params!["test-session-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            ended_at.is_some(),
            "ended_at should be set after end_session"
        );
    }

    #[test]
    fn unlock_count_for_session_filters_correctly() {
        let s = fresh_store();
        s.record_unlock(480, "ACH_A", "goldberg", "s1").unwrap();
        s.record_unlock(480, "ACH_B", "goldberg", "s1").unwrap();
        s.record_unlock(480, "ACH_C", "goldberg", "s2").unwrap();
        let conn = s.conn.lock().unwrap();
        assert_eq!(unlock_count_for_session(&conn, "s1").unwrap(), 2);
        assert_eq!(unlock_count_for_session(&conn, "s2").unwrap(), 1);
        assert_eq!(unlock_count_for_session(&conn, "s3").unwrap(), 0);
    }

    #[test]
    fn mark_notified_updates_only_matching_row() {
        let s = fresh_store();
        s.record_unlock(480, "ACH_A", "goldberg", "s1").unwrap();
        s.record_unlock(480, "ACH_B", "goldberg", "s1").unwrap();
        let conn = s.conn.lock().unwrap();
        mark_notified(&conn, 480, "ACH_A", "s1").unwrap();
        let notified_a: i64 = conn
            .query_row(
                "SELECT notified FROM unlock_history WHERE ach_api_name = ?1",
                params!["ACH_A"],
                |row| row.get(0),
            )
            .unwrap();
        let notified_b: i64 = conn
            .query_row(
                "SELECT notified FROM unlock_history WHERE ach_api_name = ?1",
                params!["ACH_B"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(notified_a, 1);
        assert_eq!(notified_b, 0);
    }
}

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

// ============================================================================
// Phase 2 additions: 100%-completion flag (D-11) + companion preferences (D-15, D-18) + earned-count.
// Settings table re-used for completion flag (key='completion_<app_id>', value='1').
// companion_prefs is its own table per migration 002.
// ============================================================================

/// Mark the 100% celebration as fired for a given app. Idempotent —
/// INSERT OR REPLACE on the settings (key, value) PK. Plan 05's popup_queue
/// calls this after emitting the completion variant popup.
pub fn mark_completion_fired(conn: &Connection, app_id: u64) -> anyhow::Result<()> {
    let _app_id_i64 = i64::try_from(app_id)?; // overflow guard
    let key = format!("completion_{}", app_id);
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, '1')",
        params![key],
    )?;
    Ok(())
}

/// Check whether the 100% celebration has already been fired for a game
/// (D-11: once per app_id ever; wiped DB re-fires once).
pub fn is_completion_fired(conn: &Connection, app_id: u64) -> anyhow::Result<bool> {
    let _app_id_i64 = i64::try_from(app_id)?;
    let key = format!("completion_{}", app_id);
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    );
    match result {
        Ok(v) => Ok(v == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Companion window per-game preferences. Mirrors the companion_prefs row.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompanionPrefs {
    pub app_id: u64,
    pub filter: Option<String>,       // 'all' | 'earned' | 'locked'
    pub sort: Option<String>,         // 'earned-first' | 'a-z'
    pub expanded_id: Option<String>,  // last-expanded ach_api_name
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
}

pub fn set_companion_prefs(conn: &Connection, prefs: &CompanionPrefs) -> anyhow::Result<()> {
    let app_id_i64 = i64::try_from(prefs.app_id)?;
    conn.execute(
        "INSERT OR REPLACE INTO companion_prefs
           (app_id, filter, sort, expanded_id, width, height, pos_x, pos_y)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            app_id_i64,
            prefs.filter,
            prefs.sort,
            prefs.expanded_id,
            prefs.width,
            prefs.height,
            prefs.pos_x,
            prefs.pos_y,
        ],
    )?;
    Ok(())
}

pub fn get_companion_prefs(
    conn: &Connection,
    app_id: u64,
) -> anyhow::Result<Option<CompanionPrefs>> {
    let app_id_i64 = i64::try_from(app_id)?;
    let row_result = conn.query_row(
        "SELECT app_id, filter, sort, expanded_id, width, height, pos_x, pos_y
         FROM companion_prefs WHERE app_id = ?1",
        params![app_id_i64],
        |r| {
            Ok(CompanionPrefs {
                app_id: r.get::<_, i64>(0)? as u64,
                filter: r.get(1)?,
                sort: r.get(2)?,
                expanded_id: r.get(3)?,
                width: r.get(4)?,
                height: r.get(5)?,
                pos_x: r.get(6)?,
                pos_y: r.get(7)?,
            })
        },
    );
    match row_result {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Count earned achievements for one (app_id, session_id) pair — used by
/// Plan 05 to detect when the burst's last unlock completes the set
/// (compare to schema_count_for_app from cache.rs).
pub fn count_earned_for_app_session(
    conn: &Connection,
    app_id: u64,
    session_id: &str,
) -> anyhow::Result<i64> {
    let app_id_i64 = i64::try_from(app_id)?;
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM unlock_history WHERE app_id = ?1 AND session_id = ?2",
        params![app_id_i64, session_id],
        |r| r.get(0),
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

    #[test]
    fn completion_fired_round_trip() {
        let s = fresh_store();
        s.with_conn(|c| {
            assert!(!is_completion_fired(c, 480).unwrap(), "fresh DB has no completion flag");
            mark_completion_fired(c, 480).unwrap();
            assert!(is_completion_fired(c, 480).unwrap(), "after mark, flag is true");
            assert!(!is_completion_fired(c, 999).unwrap(), "different app unaffected");
            // Idempotent
            mark_completion_fired(c, 480).unwrap();
            assert!(is_completion_fired(c, 480).unwrap());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn companion_prefs_round_trip() {
        let s = fresh_store();
        let prefs = CompanionPrefs {
            app_id: 480,
            filter: Some("earned".into()),
            sort: Some("a-z".into()),
            expanded_id: None,
            width: Some(520),
            height: Some(800),
            pos_x: Some(100),
            pos_y: Some(200),
        };
        s.with_conn(|c| {
            assert!(get_companion_prefs(c, 480).unwrap().is_none());
            set_companion_prefs(c, &prefs).unwrap();
            assert_eq!(get_companion_prefs(c, 480).unwrap().as_ref(), Some(&prefs));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn count_earned_for_app_session_isolates_apps_and_sessions() {
        let s = fresh_store();
        s.with_conn(|c| {
            create_session(c, "sess-1", Some(480)).unwrap();
            Ok(())
        })
        .unwrap();
        assert!(s.record_unlock(480, "ACH_A", "goldberg", "sess-1").unwrap());
        assert!(s.record_unlock(480, "ACH_B", "goldberg", "sess-1").unwrap());
        assert!(s.record_unlock(999, "ACH_X", "goldberg", "sess-1").unwrap());
        s.with_conn(|c| {
            assert_eq!(count_earned_for_app_session(c, 480, "sess-1").unwrap(), 2);
            assert_eq!(count_earned_for_app_session(c, 999, "sess-1").unwrap(), 1);
            assert_eq!(count_earned_for_app_session(c, 480, "sess-2").unwrap(), 0);
            Ok(())
        })
        .unwrap();
    }
}

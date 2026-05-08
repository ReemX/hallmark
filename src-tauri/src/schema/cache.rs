//! Typed SQLite query helpers for the `schema_cache` table introduced in
//! migration `002_schema_cache.sql`. Plan 02 owns reads/writes; Plan 05's
//! popup_queue reads at fire-time; Plan 06's companion reads on game-start.
//!
//! Pattern matches `crate::store::queries` (typed `(conn: &Connection, ...)`
//! signatures, `i64::try_from` for u64→i64 overflow guard, `params!` for
//! parameter binding, no async — invoke via `SqliteStore::with_conn(closure)`).

use rusqlite::{params, Connection};

/// One row from `schema_cache`. All metadata fields are nullable because
/// the resolution chain may populate (a) only rarity from Web API, then
/// later (b) display_name+description from Goldberg JSON. Either ordering
/// is valid; `upsert_schema` merges on PK.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaCacheRow {
    pub app_id: u64,
    pub ach_api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub icon_path: Option<String>,
    pub hidden: bool,
    pub global_pct: Option<f64>,
    pub cached_at: i64,
}

/// Insert-or-replace by (app_id, ach_api_name) PK. Always overwrites — callers
/// should fold any "preserve existing display_name when only updating rarity"
/// logic at the call site (read row → merge → upsert) since SQLite UPSERT
/// with COALESCE per-column gets verbose. Phase 2 callers always have the
/// full row at write time, so straight REPLACE is correct.
pub fn upsert_schema(conn: &Connection, row: &SchemaCacheRow) -> anyhow::Result<()> {
    // IN-03: u64 → i64 overflow guard.
    let app_id_i64 = i64::try_from(row.app_id)?;
    conn.execute(
        "INSERT OR REPLACE INTO schema_cache
           (app_id, ach_api_name, display_name, description, icon_path, hidden, global_pct, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            app_id_i64,
            row.ach_api_name,
            row.display_name,
            row.description,
            row.icon_path,
            if row.hidden { 1_i64 } else { 0_i64 },
            row.global_pct,
            row.cached_at,
        ],
    )?;
    Ok(())
}

/// Read one row by composite PK. Returns None if not cached yet.
pub fn get_schema_row(
    conn: &Connection,
    app_id: u64,
    ach_api_name: &str,
) -> anyhow::Result<Option<SchemaCacheRow>> {
    let app_id_i64 = i64::try_from(app_id)?;
    let row_result = conn.query_row(
        "SELECT app_id, ach_api_name, display_name, description, icon_path, hidden, global_pct, cached_at
         FROM schema_cache WHERE app_id = ?1 AND ach_api_name = ?2",
        params![app_id_i64, ach_api_name],
        |r| {
            Ok(SchemaCacheRow {
                app_id: r.get::<_, i64>(0)? as u64,
                ach_api_name: r.get(1)?,
                display_name: r.get(2)?,
                description: r.get(3)?,
                icon_path: r.get(4)?,
                hidden: r.get::<_, i64>(5)? != 0,
                global_pct: r.get(6)?,
                cached_at: r.get(7)?,
            })
        },
    );
    match row_result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Read every cached achievement for one game, ordered by ach_api_name for
/// deterministic test output and stable A–Z sort baseline (D-18 sort toggle).
pub fn get_schema_for_app(
    conn: &Connection,
    app_id: u64,
) -> anyhow::Result<Vec<SchemaCacheRow>> {
    let app_id_i64 = i64::try_from(app_id)?;
    let mut stmt = conn.prepare(
        "SELECT app_id, ach_api_name, display_name, description, icon_path, hidden, global_pct, cached_at
         FROM schema_cache WHERE app_id = ?1 ORDER BY ach_api_name ASC",
    )?;
    let rows = stmt.query_map(params![app_id_i64], |r| {
        Ok(SchemaCacheRow {
            app_id: r.get::<_, i64>(0)? as u64,
            ach_api_name: r.get(1)?,
            display_name: r.get(2)?,
            description: r.get(3)?,
            icon_path: r.get(4)?,
            hidden: r.get::<_, i64>(5)? != 0,
            global_pct: r.get(6)?,
            cached_at: r.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Count cached rows for an app — used by Plan 05's popup_queue to detect
/// 100% completion (count_earned_for_app_session == schema_count_for_app).
pub fn schema_count_for_app(conn: &Connection, app_id: u64) -> anyhow::Result<i64> {
    let app_id_i64 = i64::try_from(app_id)?;
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schema_cache WHERE app_id = ?1",
        params![app_id_i64],
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

    fn sample_row(app_id: u64, name: &str) -> SchemaCacheRow {
        SchemaCacheRow {
            app_id,
            ach_api_name: name.to_string(),
            display_name: Some(format!("{} display", name)),
            description: Some(format!("{} desc", name)),
            icon_path: None,
            hidden: false,
            global_pct: Some(42.5),
            cached_at: 1700000000,
        }
    }

    #[test]
    fn upsert_then_get_round_trip() {
        let s = fresh_store();
        s.with_conn(|c| {
            upsert_schema(c, &sample_row(480, "ACH_A")).unwrap();
            let got = get_schema_row(c, 480, "ACH_A").unwrap();
            assert_eq!(got, Some(sample_row(480, "ACH_A")));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn get_returns_none_for_missing() {
        let s = fresh_store();
        s.with_conn(|c| {
            let got = get_schema_row(c, 480, "MISSING").unwrap();
            assert!(got.is_none());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn get_for_app_returns_ordered_by_api_name() {
        let s = fresh_store();
        s.with_conn(|c| {
            upsert_schema(c, &sample_row(480, "ACH_C")).unwrap();
            upsert_schema(c, &sample_row(480, "ACH_A")).unwrap();
            upsert_schema(c, &sample_row(480, "ACH_B")).unwrap();
            upsert_schema(c, &sample_row(999, "ACH_X")).unwrap(); // different app
            let rows = get_schema_for_app(c, 480).unwrap();
            assert_eq!(rows.len(), 3);
            let names: Vec<_> = rows.iter().map(|r| r.ach_api_name.as_str()).collect();
            assert_eq!(names, vec!["ACH_A", "ACH_B", "ACH_C"]);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn count_for_app_excludes_other_apps() {
        let s = fresh_store();
        s.with_conn(|c| {
            upsert_schema(c, &sample_row(480, "A")).unwrap();
            upsert_schema(c, &sample_row(480, "B")).unwrap();
            upsert_schema(c, &sample_row(999, "C")).unwrap();
            assert_eq!(schema_count_for_app(c, 480).unwrap(), 2);
            assert_eq!(schema_count_for_app(c, 999).unwrap(), 1);
            assert_eq!(schema_count_for_app(c, 12345).unwrap(), 0);
            Ok(())
        })
        .unwrap();
    }
}

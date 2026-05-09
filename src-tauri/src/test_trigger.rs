//! Synthetic RawUnlockEvent injector for the "Fire test popup" tray menu item.
//!
//! D-04: events are injected at the adapter→dedup boundary (the same
//! `mpsc::Sender<RawUnlockEvent>` real adapters feed). The full pipeline
//! runs: CrossSourceDedup → SchemaCache::lookup → AudioDispatcher → popup_queue
//! → monitor placement → ui::popup window animation. The ONLY production stage
//! the test trigger does NOT exercise is the file-watcher kernel → notify-debouncer
//! callback chain — that is what real game unlocks already validate, and inserting
//! synthetic file writes would be slow + path-fragile (RESEARCH Pattern 3 rationale).
//!
//! D-05: SchemaCache::lookup short-circuits to a pre-seeded fixture row keyed by
//! (TEST_APP_ID, TEST_API_NAME). The pre-seed runs once at startup from
//! lib.rs::run() so subsequent test fires hit a warm cache.
//!
//! D-06: dedup TTL of 10s applies — rapid double-clicks correctly suppress the
//! second event (production behavior, not a bug). User-perceptible cooldown.

use std::time::SystemTime;
use tauri::{AppHandle, Manager};

use crate::sources::{RawUnlockEvent, SourceKind};

pub const TEST_API_NAME: &str = "HALLMARK_TEST_UNLOCK";
pub const TEST_APP_ID: u64 = 480; // Spacewar — official Steam test app

/// Per UI-SPEC § Copywriting Contract "Test popup fixture copy".
const FIXTURE_DISPLAY_NAME: &str = "Test Achievement";
const FIXTURE_DESCRIPTION: &str = "Hallmark is working correctly on your system.";

/// Tray-menu "Fire test popup" handler. Synthesizes a RawUnlockEvent and pushes
/// it via AppState.raw_tx — the same channel real adapters write to.
pub fn fire(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<crate::commands::AppState>();
    let raw_tx = state.raw_tx.clone();

    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let evt = RawUnlockEvent {
        app_id: TEST_APP_ID,
        ach_api_name: TEST_API_NAME.into(),
        timestamp,
        source: SourceKind::Goldberg,
    };

    // Tray menu handlers are sync — use blocking_send. If the channel is closed
    // (process is shutting down or run_pipeline panicked), log warn and bail.
    // blocking_send is preferred over try_send: try_send silently drops when the
    // channel buffer (capacity 64) is full; blocking_send waits for backpressure
    // to clear, which is correct UX — user expects a popup after clicking.
    match raw_tx.blocking_send(evt) {
        Ok(()) => {
            tracing::info!(
                app_id = TEST_APP_ID,
                api_name = TEST_API_NAME,
                "test popup fired (synthetic event injected at adapter\u{2192}dedup boundary)"
            );
            Ok(())
        }
        Err(e) => {
            tracing::warn!(error = %e, "test_trigger send failed (channel closed?)");
            anyhow::bail!("test channel closed: {e}")
        }
    }
}

/// Pre-seed schema_cache with the fixture row for the test popup. Idempotent
/// (INSERT OR REPLACE on PK). Called once from lib.rs::run() after the
/// SchemaCache is constructed.
///
/// The fixture has `global_pct: None` — `classify_tier` will route this to
/// Tier::Standard (D-05; rare / completion are not the demo tier per RESEARCH
/// Pitfall 3). `icon_path: None` — the popup falls back to the bundled
/// placeholder per Phase 2's `display_name fallback to ach_api_name` pattern.
pub fn seed_test_fixture(store: &crate::store::SqliteStore) -> anyhow::Result<()> {
    use crate::schema::cache::{upsert_schema, SchemaCacheRow};
    let cached_at = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let row = SchemaCacheRow {
        app_id: TEST_APP_ID,
        ach_api_name: TEST_API_NAME.into(),
        display_name: Some(FIXTURE_DISPLAY_NAME.into()),
        description: Some(FIXTURE_DESCRIPTION.into()),
        icon_path: None,
        hidden: false,
        global_pct: None,
        cached_at,
    };
    store.with_conn(|c| upsert_schema(c, &row))?;
    tracing::info!(
        app_id = TEST_APP_ID,
        api_name = TEST_API_NAME,
        "test popup fixture seeded into schema_cache"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::cache::{get_schema_row, SchemaCacheRow};
    use crate::store::SqliteStore;

    fn fresh_store() -> SqliteStore {
        SqliteStore::open_in_memory().expect("in-memory store opens")
    }

    #[test]
    fn seed_inserts_one_row_with_canonical_copy() {
        let s = fresh_store();
        seed_test_fixture(&s).unwrap();
        let row = s
            .with_conn(|c| get_schema_row(c, TEST_APP_ID, TEST_API_NAME))
            .unwrap()
            .expect("seed produced a row");
        assert_eq!(row.app_id, TEST_APP_ID);
        assert_eq!(row.ach_api_name, TEST_API_NAME);
        assert_eq!(row.display_name.as_deref(), Some("Test Achievement"));
        assert_eq!(
            row.description.as_deref(),
            Some("Hallmark is working correctly on your system.")
        );
        assert!(row.icon_path.is_none());
        assert!(row.global_pct.is_none());
        assert!(!row.hidden);
    }

    #[test]
    fn seed_is_idempotent() {
        let s = fresh_store();
        seed_test_fixture(&s).unwrap();
        seed_test_fixture(&s).unwrap(); // second call must not error
        // Row count for the test PK must be exactly 1.
        let row = s
            .with_conn(|c| get_schema_row(c, TEST_APP_ID, TEST_API_NAME))
            .unwrap();
        assert!(row.is_some());
    }

    #[test]
    fn seed_does_not_overwrite_other_rows_at_same_app_id() {
        let s = fresh_store();
        // Insert a non-fixture row at the same app_id.
        let other = SchemaCacheRow {
            app_id: TEST_APP_ID,
            ach_api_name: "ACH_REAL_SPACEWAR".into(),
            display_name: Some("Real Spacewar Achievement".into()),
            description: Some("This is a real Spacewar achievement.".into()),
            icon_path: None,
            hidden: false,
            global_pct: Some(42.0),
            cached_at: 1_700_000_000,
        };
        s.with_conn(|c| crate::schema::cache::upsert_schema(c, &other))
            .unwrap();

        // Now seed the test fixture.
        seed_test_fixture(&s).unwrap();

        // The other row should be untouched.
        let preserved = s
            .with_conn(|c| get_schema_row(c, TEST_APP_ID, "ACH_REAL_SPACEWAR"))
            .unwrap()
            .expect("non-fixture row preserved");
        assert_eq!(
            preserved.display_name.as_deref(),
            Some("Real Spacewar Achievement")
        );
        assert_eq!(preserved.global_pct, Some(42.0));

        // The fixture row exists alongside.
        let fixture = s
            .with_conn(|c| get_schema_row(c, TEST_APP_ID, TEST_API_NAME))
            .unwrap()
            .expect("fixture row exists");
        assert_eq!(fixture.display_name.as_deref(), Some("Test Achievement"));
    }
}

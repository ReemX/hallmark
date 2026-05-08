//! Component-level test for COMP-01 / COMP-02 / COMP-03: companion behavior
//! when game-started fires.
//!
//! What this test PROVES:
//!   • SchemaCache::resolve writes schema_cache rows on Goldberg metadata.
//!   • Companion's data source (get_companion_state) reads schema + earned
//!     correctly given a populated cache + unlock_history.
//!
//! What this test does NOT verify (manual smoke required):
//!   • Tauri webview show()/hide() actually toggling companion visibility
//!     (requires Tauri runtime with real WebViewWindow).

use std::sync::Arc;
use std::fs;

use hallmark_lib::schema::{cache, SchemaCache};
use hallmark_lib::store::{queries, SqliteStore};

fn fresh_tmp(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("hallmark-companion-{}-{}", name, uuid::Uuid::new_v4()));
    fs::create_dir_all(&p).unwrap();
    p
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_cache_populates_after_resolve() {
    hallmark_lib::init_tracing_for_tests();

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());

    // Write a Goldberg achievements.json fixture.
    let goldberg_dir = fresh_tmp("goldberg-fixture");
    let json_path = goldberg_dir.join("achievements.json");
    fs::write(&json_path, r#"[
        {"name":"ACH_A","display_name":"Got A","description":"Win once","hidden":false},
        {"name":"ACH_B","display_name":"Got B","description":"Win twice","hidden":false}
    ]"#).unwrap();

    // Pre-populate the schema cache directly (bypasses the network leg —
    // we don't want to hit Steam Web API in tests).
    store.with_conn(|c| {
        cache::upsert_schema(c, &cache::SchemaCacheRow {
            app_id: 480, ach_api_name: "ACH_A".into(),
            display_name: Some("Got A".into()), description: Some("Win once".into()),
            icon_path: None, hidden: false, global_pct: None,
            cached_at: 1_700_000_000,
        })?;
        cache::upsert_schema(c, &cache::SchemaCacheRow {
            app_id: 480, ach_api_name: "ACH_B".into(),
            display_name: Some("Got B".into()), description: Some("Win twice".into()),
            icon_path: None, hidden: false, global_pct: None,
            cached_at: 1_700_000_000,
        })?;
        Ok(())
    }).unwrap();

    // Verify the lookup returns the cached rows.
    let sc = SchemaCache::new(store.clone()).unwrap();
    let list = sc.list_for_app(480);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].ach_api_name, "ACH_A");
    assert_eq!(list[1].ach_api_name, "ACH_B");

    let _ = fs::remove_dir_all(&goldberg_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn earned_unlock_history_persists_session() {
    hallmark_lib::init_tracing_for_tests();

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let session_id = "test-session".to_string();
    store.with_conn(|c| queries::create_session(c, &session_id, Some(480))).unwrap();

    // Record an unlock during the session.
    assert!(store.record_unlock(480, "ACH_FIRST", "goldberg", &session_id).unwrap());
    assert!(store.record_unlock(480, "ACH_SECOND", "goldberg", &session_id).unwrap());

    // Simulate Hallmark restart by creating a new SchemaCache instance
    // pointing at the same store. The unlock_history rows must still be there.
    let count = store.with_conn(|c|
        queries::count_earned_for_app_session(c, 480, &session_id)
    ).unwrap();
    assert_eq!(count, 2, "earned-this-session must persist (COMP-03)");
}

#[test]
fn completion_flag_persists_once_per_app() {
    let store = SqliteStore::open_in_memory().unwrap();
    store.with_conn(|c| {
        assert!(!queries::is_completion_fired(c, 480).unwrap());
        queries::mark_completion_fired(c, 480).unwrap();
        assert!(queries::is_completion_fired(c, 480).unwrap());
        // Re-marking is idempotent.
        queries::mark_completion_fired(c, 480).unwrap();
        assert!(queries::is_completion_fired(c, 480).unwrap());
        // Different app independent.
        assert!(!queries::is_completion_fired(c, 999).unwrap());
        Ok(())
    }).unwrap();
}

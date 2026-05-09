//! Synthetic RawUnlockEvent test trigger — Phase 4 Plan 04-03 owns implementation.
//! See CONTEXT.md D-04..D-06.

use tauri::AppHandle;

pub const TEST_API_NAME: &str = "HALLMARK_TEST_UNLOCK";
pub const TEST_APP_ID: u64 = 480;  // Spacewar — Steam test app

/// Inject a synthetic unlock event at the adapter→dedup boundary (D-04).
/// Plan 04-03 implements; Plan 04-02's tray menu calls this on "Fire test popup".
#[allow(unused_variables)]
pub fn fire(app: &AppHandle) -> anyhow::Result<()> {
    tracing::warn!("test_trigger::fire STUB — Plan 04-03 not yet implemented");
    Ok(())
}

/// Pre-seed schema_cache with the test fixture row (D-05). Plan 04-03 implements;
/// `lib.rs::run()` calls this once after schema_cache is constructed (04-01b).
#[allow(unused_variables)]
pub fn seed_test_fixture(store: &crate::store::SqliteStore) -> anyhow::Result<()> {
    tracing::warn!("test_trigger::seed_test_fixture STUB — Plan 04-03 not yet implemented");
    Ok(())
}

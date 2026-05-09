//! tauri-plugin-updater background-check + AppState pending-update stash.
//! Phase 4 Plan 04-04 owns implementation. See CONTEXT.md D-18..D-21.

use tauri::AppHandle;

/// Background-check `latest.json` on startup. If newer version available,
/// stash on AppState.pending_update and emit `update-available` to companion.
/// Skipped silently when `portable_mode::is_portable()` returns true (D-23).
/// Plan 04-04 implements.
#[allow(unused_variables)]
pub fn spawn_background_check(app: AppHandle) {
    tracing::warn!("updater_glue::spawn_background_check STUB — Plan 04-04 not yet implemented");
}

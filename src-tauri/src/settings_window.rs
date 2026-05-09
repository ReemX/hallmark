//! Settings window builder — Phase 4 Plan 04-04 owns implementation.
//! See CONTEXT.md D-10, D-11, D-12.

use tauri::AppHandle;

/// Open (or re-focus if already open) the Settings window.
/// Plan 04-04 implements.
#[allow(unused_variables)]
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    tracing::warn!("settings_window::open STUB — Plan 04-04 not yet implemented");
    Ok(())
}

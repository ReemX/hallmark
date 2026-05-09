//! First-run wizard window builder + flag-aware open logic.
//! Phase 4 Plan 04-05 owns implementation. See CONTEXT.md D-13..D-17.

use tauri::AppHandle;

/// Build and show the wizard window when the first-run-done flag is unset
/// OR when 0 paths are detected (D-14 re-fire logic).
/// Plan 04-05 implements.
#[allow(unused_variables)]
pub fn open_wizard(app: AppHandle, any_path_detected: bool) -> tauri::Result<()> {
    tracing::warn!("first_run::open_wizard STUB — Plan 04-05 not yet implemented");
    Ok(())
}

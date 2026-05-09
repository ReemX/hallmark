//! Tray icon + menu — Phase 4 Plan 04-02 owns implementation.
//! See CONTEXT.md D-01 for menu structure, D-02 for icon presence,
//! D-03 for Quit semantics, D-09 for autostart-toggle state sync.

use tauri::App;

/// Build and register the system-tray icon. Plan 04-02 implements.
#[allow(unused_variables)]
pub fn build_tray(app: &App) -> tauri::Result<()> {
    tracing::warn!("tray::build_tray STUB — Plan 04-02 not yet implemented");
    Ok(())
}

//! First-run wizard window builder. D-13: standalone borderless rounded card.
//! D-14: trigger lifecycle is owned by lib.rs::run() (calls open_wizard when
//! first_run_done is unset OR no paths detected). This module only builds
//! the window — the React side handles the conditional rendering of N>0 vs
//! N=0 layouts based on what `rescan_paths` returns.
//!
//! NO close button per D-13 — user exits only via `Get started` or `Continue anyway`,
//! both of which call the `wizard_dismiss` Tauri command.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Build and show the first-run wizard. Idempotent: if a "wizard" window
/// already exists, focus it instead of creating a duplicate.
///
/// `any_path_detected` is currently unused at the Rust layer — the React
/// side calls `rescan_paths` on mount and decides which layout to render
/// based on what comes back. Keeping the parameter for future use (e.g.,
/// passing initial state via window-init payload).
pub fn open_wizard(app: AppHandle, _any_path_detected: bool) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("wizard") {
        let _ = w.show();
        let _ = w.set_focus();
        tracing::info!("wizard window re-focused");
        return Ok(());
    }
    let _win = WebviewWindowBuilder::new(&app, "wizard", WebviewUrl::App("wizard.html".into()))
        .title("Welcome to Hallmark")
        .decorations(false)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .focused(true)
        .resizable(false)
        .visible(true)
        .inner_size(480.0, 560.0)
        .center()
        .build()?;
    tracing::info!("first-run wizard window built (480x560 logical)");
    Ok(())
}

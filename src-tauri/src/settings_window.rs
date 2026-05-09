//! Settings window builder (D-10..D-12). Borderless rounded card 520×580.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Open or focus the existing Settings window.
/// Idempotent: if a "settings" window already exists, show + focus instead of creating.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
        tracing::info!("settings window re-focused");
        return Ok(());
    }
    let _win = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Hallmark Settings")
        .decorations(false)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .focused(true)
        .resizable(false)
        .visible(true)
        .inner_size(520.0, 580.0)
        .center()
        .build()?;
    tracing::info!("settings window built (520x580 logical)");
    Ok(())
}

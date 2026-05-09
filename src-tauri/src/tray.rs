//! System-tray icon + D-01 locked menu.
//!
//! ## Menu structure (D-01, locked)
//! ```text
//! Hallmark            ← non-clickable header
//! ─────────────────
//! Show companion
//! Fire test popup
//! ─────────────────
//! Settings…
//! ☑ Start with Windows
//! ─────────────────
//! Quit
//! ```
//!
//! Left-click on the tray icon = Show companion (D-02).
//! Quit drains popup queue with 1.5 s grace, then calls `app.exit(0)` (D-03).
//! "Start with Windows" reflects live HKCU registry on each menu open (D-09).
//! Menu is rebuilt on every autostart toggle so the check-mark state stays
//! current (Pitfall 2 — `set_menu` trick).

use std::time::Duration;
use tauri::{
    image::Image,
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

/// Build and register the system-tray icon with the D-01 locked menu structure.
/// Called once from `lib.rs::run()` setup(). Failures are logged and tolerated
/// (warn-and-continue — tray is best-effort; detection pipeline still runs).
pub fn build_tray(app: &App) -> tauri::Result<()> {
    let autostart_on = autostart_state();
    let menu = build_menu(app.handle(), autostart_on)?;
    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray.ico"))?;

    TrayIconBuilder::with_id("hallmark-tray")
        .tooltip("Hallmark")
        .icon(tray_icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_event)
        .build(app)?;

    tracing::info!("tray icon registered with D-01 menu structure");
    Ok(())
}

// ---- Menu builder --------------------------------------------------------

/// Build the D-01 menu with the current autostart state.
/// Re-called on every autostart toggle so the check-mark reflects live state.
fn build_menu(
    app: &AppHandle,
    autostart_on: bool,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    // D-01 LOCKED order:
    //   Hallmark / sep / Show companion / Fire test popup / sep /
    //   Settings… / ☑ Start with Windows / sep / Quit
    let header = MenuItem::with_id(app, "header", "Hallmark", false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show_companion", "Show companion", true, None::<&str>)?;
    let test = MenuItem::with_id(app, "fire_test", "Fire test popup", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
    let autostart = CheckMenuItemBuilder::with_id("toggle_autostart", "Start with Windows")
        .checked(autostart_on)
        .enabled(true)
        .build(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;

    MenuBuilder::new(app)
        .items(&[
            &header,
            &sep1,
            &show,
            &test,
            &sep2,
            &settings,
            &autostart,
            &sep3,
            &quit,
        ])
        .build()
}

/// Read the live HKCU autostart state, defaulting to false on error.
fn autostart_state() -> bool {
    crate::autostart::is_enabled().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "autostart::is_enabled failed; defaulting to false");
        false
    })
}

// ---- Event handlers -------------------------------------------------------

/// Menu-item event handler.
fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show_companion" => show_companion(app),

        "fire_test" => {
            // Plan 04-03 implements fire(); stub WARNs until then.
            if let Err(e) = crate::test_trigger::fire(app) {
                tracing::warn!(error = %e, "test_trigger::fire failed");
            }
        }

        "open_settings" => {
            // Plan 04-04 implements open(); stub WARNs until then.
            if let Err(e) = crate::settings_window::open(app) {
                tracing::warn!(error = %e, "settings_window::open failed");
            }
        }

        "toggle_autostart" => {
            // Read current state, then flip it.
            let now_on = crate::autostart::is_enabled().unwrap_or(false);
            let result = if now_on {
                crate::autostart::disable()
            } else {
                crate::autostart::enable()
            };
            match result {
                Ok(()) => {
                    tracing::info!(was_on = now_on, is_on = !now_on, "autostart toggled");
                    // Pitfall 2: rebuild menu so next open shows updated check state (D-09).
                    if let Some(tray) = app.tray_by_id("hallmark-tray") {
                        match build_menu(app, !now_on) {
                            Ok(m) => {
                                let _ = tray.set_menu(Some(m));
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "menu rebuild failed after autostart toggle"
                                );
                            }
                        }
                    }
                }
                Err(e) => tracing::warn!(error = %e, "autostart toggle failed"),
            }
        }

        "quit" => initiate_quit(app),

        "header" => {
            // Non-clickable header row — no-op (disabled in menu but event may still fire).
        }

        other => {
            tracing::debug!(id = other, "unknown tray menu event");
        }
    }
}

/// Tray-icon click handler (D-02: left-click = Show companion).
fn handle_tray_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_companion(tray.app_handle());
    }
}

// ---- Actions --------------------------------------------------------------

/// Show and focus the existing companion window. Shared by left-click (D-02)
/// and the "Show companion" menu item.
fn show_companion(app: &AppHandle) {
    match app.get_webview_window("companion") {
        Some(w) => {
            let _ = w.show();
            let _ = w.set_focus();
            tracing::info!("companion window shown via tray");
        }
        None => tracing::warn!("companion window not found; cannot show from tray"),
    }
}

/// Quit with drain (D-03). Spawns an async task that gives the popup queue
/// 1.5 s to finish any in-flight exit animation, then calls `app.exit(0)`.
///
/// The 1.5 s grace matches the popup's exit-animation duration per UI-SPEC
/// §Animation Contract. AnimatePresence in React handles the visual; the
/// timeout is the backend backstop (RESEARCH Pitfall 5).
fn initiate_quit(app: &AppHandle) {
    tracing::info!("Quit requested — draining popup queue (1.5 s grace)");
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1_500)).await;
        tracing::info!("Quit drain complete; calling app.exit(0)");
        app_clone.exit(0);
    });
}

//! Window builders for the popup overlay + companion. Plan 05 owns this module.
//! Both windows are created programmatically from Rust (rather than declaratively
//! in tauri.conf.json's `app.windows`) so the WS_EX_NOACTIVATE HWND patch can
//! run immediately after `build()` returns — closing the focus-steal window.

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW,
        GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    },
};

/// Build the popup overlay window (label "popup"). Borderless, transparent,
/// always-on-top, click-through, non-focusable. Defense-in-depth focus-steal
/// prevention: builder `.focused(false)` AND post-creation `WS_EX_NOACTIVATE`
/// HWND patch (RESEARCH.md Pattern 1; Pitfall 2 documents why both are needed).
pub fn create_popup_window(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("popup.html".into()))
        .title("Hallmark Popup")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .resizable(false)
        .visible(false)              // queue task shows on first popup
        .inner_size(440.0, 96.0)     // logical px; per UI-SPEC.md popup surface
        .accept_first_mouse(false)
        .visible_on_all_workspaces(true)
        .shadow(false)               // we paint our own shadow in CSS
        .build()?;

    // ----- Defense-in-depth WS_EX_NOACTIVATE patch (POPUP-08) -----
    // Tauri issues #7519/#11566/#12055 show focused(false) has not been
    // 100% reliable on Windows. Manually OR-in the raw flag.
    #[cfg(target_os = "windows")]
    {
        let hwnd = HWND(win.hwnd()?.0 as *mut _);
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_style = current
                | (WS_EX_NOACTIVATE.0 as isize)
                | (WS_EX_TRANSPARENT.0 as isize)
                | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
        tracing::info!("popup window: WS_EX_NOACTIVATE HWND patch applied");
    }

    // Click-through (popup is non-interactive — no controls).
    win.set_ignore_cursor_events(true)?;
    tracing::info!("popup window built (440x96 logical, hidden until first fire)");
    Ok(())
}

/// Build the companion window (label "companion"). Borderless rounded card
/// (D-13), normal focus (D-16 NOT always-on-top), 480×720 default size (D-14).
/// Hidden initially — Plan 06's game-start handler shows it.
pub fn create_companion_window(app: &AppHandle) -> tauri::Result<()> {
    let _win = WebviewWindowBuilder::new(app, "companion", WebviewUrl::App("index.html".into()))
        .title("Hallmark Companion")
        .decorations(false)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)            // visible in alt-tab
        .focused(false)                 // don't grab focus on creation
        .resizable(true)
        .visible(false)                 // game-start handler shows on launch
        .inner_size(480.0, 720.0)       // D-14 default
        .min_inner_size(360.0, 480.0)
        .center()                       // D-15 first-run; persisted prefs override later
        .build()?;
    tracing::info!("companion window built (480x720 logical, hidden until game-start)");
    Ok(())
}

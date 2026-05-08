//! Win32 HWND-by-PID + monitor placement helpers used by Plan 05's popup_queue
//! to position the popup on the running game's monitor (POPUP-03).
//!
//! Pattern mirrors paths.rs: real impl under `#[cfg(target_os = "windows")]`,
//! non-Windows stub for compile-cross-platform safety. The pure-math
//! `popup_position()` has no cfg gate — it works on any target.

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, HMONITOR,
        MONITOR_DEFAULTTONEAREST, MONITORINFO,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    },
};

/// On Windows, find the first visible top-level window owned by the given PID.
/// Returns None on non-Windows targets.
///
/// Implementation note (Pitfall 12): the EnumWindows callback uses ONLY
/// non-panicking operations (struct field assigns + BOOL returns). Rust
/// panics across the FFI boundary are UB.
#[cfg(target_os = "windows")]
pub fn hwnd_for_pid(pid: u32) -> Option<HWND> {
    struct Ctx { pid: u32, found: Option<HWND> }
    let mut ctx = Ctx { pid, found: None };

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is a valid &mut Ctx for the duration of EnumWindows.
        let ctx = &mut *(lparam.0 as *mut Ctx);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1); // continue
        }
        let mut wpid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut wpid));
        if wpid == ctx.pid {
            ctx.found = Some(hwnd);
            return BOOL(0); // stop
        }
        BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut ctx as *mut Ctx as isize));
    }
    ctx.found
}

#[cfg(not(target_os = "windows"))]
pub fn hwnd_for_pid(_pid: u32) -> Option<()> { None }

/// Returns (left, top, width, height) in PHYSICAL pixels for the rcWork rect
/// of the monitor closest to the given HWND. rcWork excludes the taskbar.
/// Returns None on non-Windows or when GetMonitorInfoW fails.
#[cfg(target_os = "windows")]
pub fn monitor_rect_for_hwnd(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let hmon: HMONITOR = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            let r: RECT = info.rcWork;
            Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
        } else { None }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn monitor_rect_for_hwnd(_hwnd: ()) -> Option<(i32, i32, i32, i32)> { None }

/// Compute popup placement in PHYSICAL pixels.
/// Anchor: top-right, ~25% from top edge, 32px margin from screen edge (D-01).
/// Pure math — no cfg gate. Tests run on any target.
pub fn popup_position(
    mon_x: i32, mon_y: i32, mon_w: i32, mon_h: i32,
    popup_w: i32, popup_h: i32,
) -> (i32, i32) {
    let margin = 32_i32;
    let x = mon_x + mon_w - popup_w - margin;
    let y = mon_y + (mon_h / 4) - (popup_h / 2);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::popup_position;

    #[test]
    fn popup_position_top_right_quarter_down_1080p() {
        // 1920×1080 monitor at origin, popup 440×96 logical (matches D-14 size).
        // Tauri set_position takes physical px; this is also physical for 100% DPI.
        let (x, y) = popup_position(0, 0, 1920, 1080, 440, 96);
        assert_eq!(x, 1920 - 440 - 32, "x = right-edge - popup_w - margin");
        assert_eq!(y, (1080 / 4) - (96 / 2), "y = quarter-down - popup_h/2");
    }

    #[test]
    fn popup_position_top_right_4k_secondary() {
        // 4K secondary monitor positioned to the right of a primary.
        let (x, y) = popup_position(1920, 0, 3840, 2160, 880, 192); // 2x DPI popup
        assert_eq!(x, 1920 + 3840 - 880 - 32);
        assert_eq!(y, 0 + (2160 / 4) - (192 / 2));
    }

    #[test]
    fn popup_position_negative_secondary_to_left() {
        // Secondary monitor at negative x (to the left of primary).
        let (x, y) = popup_position(-1920, 0, 1920, 1080, 440, 96);
        assert_eq!(x, -1920 + 1920 - 440 - 32);
        assert_eq!(y, 0 + (1080 / 4) - (96 / 2));
    }

    #[test]
    fn popup_position_uses_32px_margin_consistently() {
        for (w, _h) in [(1920, 1080), (3840, 2160), (2560, 1440)] {
            let (x, _) = popup_position(0, 0, w, 1080, 440, 96);
            assert_eq!(x + 440 + 32, w, "right edge of popup + margin = monitor_w");
        }
    }
}

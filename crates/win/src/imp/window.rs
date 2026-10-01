//! The popup window: shown without taking focus, so the app being read keeps its caret
//! and keyboard; placed on the right monitor in physical pixels.

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::{Error, Rect};

fn hwnd(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

/// Makes a window never activate (clicks work, focus stays where it was) and keeps it
/// out of the taskbar and Alt+Tab.
pub fn prepare_popup_window(raw: isize) -> Result<(), Error> {
    let h = hwnd(raw);
    // SAFETY: `raw` is a live top-level window owned by this process.
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let new = (ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as isize)
            & !(WS_EX_APPWINDOW.0 as isize);
        SetWindowLongPtrW(h, GWL_EXSTYLE, new);
    }
    Ok(())
}

/// Shows the window at a position and size (physical pixels) without activating it.
pub fn show_window_at(raw: isize, x: i32, y: i32, w: i32, h: i32) -> Result<(), Error> {
    // SAFETY: see `prepare_popup_window`.
    unsafe {
        SetWindowPos(
            hwnd(raw),
            Some(HWND_TOPMOST),
            x,
            y,
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    }
    .map_err(|e| Error::Os(e.message()))
}

pub fn hide_window(raw: isize) {
    // SAFETY: see `prepare_popup_window`.
    unsafe {
        let _ = ShowWindow(hwnd(raw), SW_HIDE);
    }
}

/// The mouse position in physical pixels.
pub fn cursor_position() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    // SAFETY: writes into `p`.
    unsafe { GetCursorPos(&mut p) }.ok()?;
    Some((p.x, p.y))
}

/// The usable area (without the taskbar) of the monitor containing a point.
pub fn work_area_at(x: i32, y: i32) -> Option<Rect> {
    // SAFETY: MONITORINFO is initialised with its size before the call.
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let r = info.rcWork;
        Some(Rect::new(r.left, r.top, r.right, r.bottom))
    }
}

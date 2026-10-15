//! The popup window: shown without taking focus, so the app being read keeps its caret
//! and keyboard; placed on the right monitor in physical pixels.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, GetWindowRect, HWND_TOPMOST, SW_HIDE,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_SIZING, WMSZ_BOTTOMLEFT, WMSZ_LEFT, WMSZ_TOP,
    WMSZ_TOPLEFT, WMSZ_TOPRIGHT, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};

use crate::{Error, Rect, SizeMove};

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

/// Lets the card take keyboard focus (`focus`: the person opened something to type in, such
/// as the language search) or makes it non-activating again.
pub fn set_popup_focus(raw: isize, focus: bool) -> Result<(), Error> {
    let h = hwnd(raw);
    // SAFETY: see `prepare_popup_window`.
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let no_activate = WS_EX_NOACTIVATE.0 as isize;
        let new = if focus {
            ex & !no_activate
        } else {
            ex | no_activate
        };
        SetWindowLongPtrW(h, GWL_EXSTYLE, new);
        if focus {
            // Allowed: the person just clicked this window.
            let _ = SetForegroundWindow(h);
        }
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

type SizeMoveCallback = Box<dyn Fn(SizeMove) + Send + Sync>;
static ON_SIZE_MOVE: OnceLock<(SizeMoveCallback, (i32, i32))> = OnceLock::new();
static SIZING: AtomicBool = AtomicBool::new(false);

/// Reports when the person moves or resizes the window with Windows' own dragging (its
/// start, the first resize step, and where the window ended up), and keeps it at least
/// `min_css` (CSS pixels) while resizing. Call once, on the window's thread.
pub fn watch_size_move(
    raw: isize,
    min_css: (i32, i32),
    on_change: impl Fn(SizeMove) + Send + Sync + 'static,
) -> Result<(), Error> {
    if ON_SIZE_MOVE.set((Box::new(on_change), min_css)).is_err() {
        return Ok(());
    }
    // SAFETY: `raw` is a live window created by this thread; the procedure lives forever.
    let ok = unsafe { SetWindowSubclass(hwnd(raw), Some(size_move_proc), 1, 0) };
    if ok.as_bool() {
        Ok(())
    } else {
        Err(Error::Os("couldn't watch the window's moves".into()))
    }
}

unsafe extern "system" fn size_move_proc(
    h: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if let Some((callback, (min_w, min_h))) = ON_SIZE_MOVE.get() {
        match msg {
            WM_ENTERSIZEMOVE => {
                SIZING.store(false, Ordering::SeqCst);
                callback(SizeMove::Start);
            }
            WM_SIZING => {
                // SAFETY: for WM_SIZING, lparam points to the window's proposed RECT.
                let r = unsafe { &mut *(lparam.0 as *mut RECT) };
                // SAFETY: a plain query on a live window.
                let dpi = i32::try_from(unsafe { GetDpiForWindow(h) })
                    .unwrap_or(96)
                    .max(96);
                let (min_w, min_h) = (min_w * dpi / 96, min_h * dpi / 96);
                let edge = wparam.0 as u32;
                if r.right - r.left < min_w {
                    if matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
                        r.left = r.right - min_w;
                    } else {
                        r.right = r.left + min_w;
                    }
                }
                if r.bottom - r.top < min_h {
                    if matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
                        r.top = r.bottom - min_h;
                    } else {
                        r.bottom = r.top + min_h;
                    }
                }
                if !SIZING.swap(true, Ordering::SeqCst) {
                    callback(SizeMove::Sizing);
                }
            }
            WM_EXITSIZEMOVE => {
                let mut r = RECT::default();
                // SAFETY: writes into `r`.
                if unsafe { GetWindowRect(h, &mut r) }.is_ok() {
                    callback(SizeMove::End(Rect::new(r.left, r.top, r.right, r.bottom)));
                }
            }
            _ => {}
        }
    }
    // SAFETY: passes every message on to the window's own procedure.
    unsafe { DefSubclassProc(h, msg, wparam, lparam) }
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

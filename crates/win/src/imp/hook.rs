//! Noticing when the person clicks anywhere else while the card is open (it then gives
//! Esc back to that app). The card never has focus, so it can't rely on losing it; a
//! low-level mouse hook sees clicks in every app. The same
//! hook sees a click into the card's web page before the page gets it, so the card can
//! become focusable in time for that click to focus it (typing in a search box).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, MSG, MSLLHOOKSTRUCT, SetWindowsHookExW,
    TranslateMessage, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
};

use crate::Rect;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static RECT: Mutex<Option<Rect>> = Mutex::new(None);
static FOCUS_RECT: Mutex<Option<Rect>> = Mutex::new(None);
type Callback = Box<dyn Fn() + Send + Sync>;
static ON_OUTSIDE: OnceLock<Callback> = OnceLock::new();
static ON_FOCUS_AREA: OnceLock<Callback> = OnceLock::new();

pub struct ClickOutsideWatcher;

impl ClickOutsideWatcher {
    /// Installs the hook on its own thread (once per process). `on_focus_area` runs on the
    /// hook's thread before the click is delivered, so it must be quick.
    pub fn start(
        on_click_outside: impl Fn() + Send + Sync + 'static,
        on_focus_area: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let _ = ON_FOCUS_AREA.set(Box::new(on_focus_area));
        if ON_OUTSIDE.set(Box::new(on_click_outside)).is_ok() {
            let _ = std::thread::Builder::new()
                .name("click-outside".into())
                .spawn(|| {
                    // SAFETY: the hook lives as long as this thread's message loop.
                    unsafe {
                        let module = GetModuleHandleW(None).ok();
                        if SetWindowsHookExW(
                            WH_MOUSE_LL,
                            Some(mouse_proc),
                            module.map(Into::into),
                            0,
                        )
                        .is_err()
                        {
                            tracing::warn!(
                                "couldn't install the mouse hook; clicks elsewhere go unnoticed"
                            );
                            return;
                        }
                        let mut msg = MSG::default();
                        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                });
        }
        ClickOutsideWatcher
    }

    /// The card's rectangle (physical pixels) while it is shown, `None` while hidden.
    pub fn set_rect(&self, rect: Option<Rect>) {
        if let Ok(mut r) = RECT.lock() {
            *r = rect;
        }
        update_active();
    }

    /// Where the card's web page is (physical pixels) while one shows, else `None`.
    pub fn set_focus_rect(&self, rect: Option<Rect>) {
        if let Ok(mut r) = FOCUS_RECT.lock() {
            *r = rect;
        }
        update_active();
    }
}

fn update_active() {
    let card = RECT.lock().ok().is_some_and(|r| r.is_some());
    let focus = FOCUS_RECT.lock().ok().is_some_and(|r| r.is_some());
    ACTIVE.store(card || focus, Ordering::SeqCst);
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && ACTIVE.load(Ordering::Relaxed) {
        let msg = wparam.0 as u32;
        if matches!(
            msg,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        ) {
            // SAFETY: for WH_MOUSE_LL, lparam points to an MSLLHOOKSTRUCT.
            let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
            let (x, y) = (info.pt.x, info.pt.y);
            let in_page = FOCUS_RECT
                .lock()
                .ok()
                .and_then(|r| *r)
                .is_some_and(|r| r.contains(x, y));
            let outside = RECT
                .lock()
                .ok()
                .and_then(|r| *r)
                .is_some_and(|r| !r.contains(x, y));
            if in_page {
                if let Some(cb) = ON_FOCUS_AREA.get() {
                    cb();
                }
            } else if outside {
                // Once is enough, until the card is placed again.
                if let Ok(mut r) = RECT.lock() {
                    *r = None;
                }
                update_active();
                if let Some(cb) = ON_OUTSIDE.get() {
                    cb();
                }
            }
        }
    }
    // SAFETY: always pass the event on; we never swallow clicks.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

//! Closing the card when the person clicks anywhere else. The card never has focus, so
//! it can't rely on losing it; a low-level mouse hook sees clicks in every app.

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
type Callback = Box<dyn Fn() + Send + Sync>;
static ON_OUTSIDE: OnceLock<Callback> = OnceLock::new();

pub struct ClickOutsideWatcher;

impl ClickOutsideWatcher {
    /// Installs the hook on its own thread (once per process).
    pub fn start(on_click_outside: impl Fn() + Send + Sync + 'static) -> Self {
        if ON_OUTSIDE.set(Box::new(on_click_outside)).is_ok() {
            let _ = std::thread::Builder::new().name("click-outside".into()).spawn(|| {
                // SAFETY: the hook lives as long as this thread's message loop.
                unsafe {
                    let module = GetModuleHandleW(None).ok();
                    if SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module.map(Into::into), 0).is_err() {
                        tracing::warn!("couldn't install the mouse hook; click outside won't close the card");
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
        ACTIVE.store(rect.is_some(), Ordering::SeqCst);
    }
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
            let outside = RECT
                .lock()
                .ok()
                .and_then(|r| *r)
                .is_some_and(|r| !r.contains(info.pt.x, info.pt.y));
            if outside {
                ACTIVE.store(false, Ordering::SeqCst);
                if let Some(cb) = ON_OUTSIDE.get() {
                    cb();
                }
            }
        }
    }
    // SAFETY: always pass the event on; we never swallow clicks.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

//! Noticing presses of the mouse while the card is shown. A press inside the card is seen
//! before the card gets it, so the card can become focusable in time for that press to
//! activate it (the card is opened without focus, and takes it once the person clicks in
//! it). A press anywhere else gives Esc back to that app. A low-level mouse hook sees
//! presses in every app.

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
static ON_INSIDE: OnceLock<Callback> = OnceLock::new();

pub struct ClickOutsideWatcher;

impl ClickOutsideWatcher {
    /// Installs the hook on its own thread (once per process). `on_click_inside` runs on
    /// the hook's thread before the press is delivered, so it must be quick.
    pub fn start(
        on_click_outside: impl Fn() + Send + Sync + 'static,
        on_click_inside: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let _ = ON_INSIDE.set(Box::new(on_click_inside));
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
            let inside = RECT
                .lock()
                .ok()
                .and_then(|r| *r)
                .map(|r| r.contains(info.pt.x, info.pt.y));
            let callback = match inside {
                Some(true) => ON_INSIDE.get(),
                Some(false) => ON_OUTSIDE.get(),
                None => None,
            };
            if let Some(cb) = callback {
                cb();
            }
        }
    }
    // SAFETY: always pass the event on; we never swallow clicks.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

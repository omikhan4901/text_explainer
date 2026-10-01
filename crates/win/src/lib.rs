//! Windows integration: reading the selected text from any app, the clipboard-safe copy
//! fallback, the non-activating popup window, click-outside detection, keeping the model
//! process tied to the app, and system memory.
//!
//! On other platforms the Windows functions return `Error::Unsupported`, so the
//! workspace still builds and tests everywhere.

pub mod placement;

#[cfg(windows)]
mod imp;

pub use placement::Rect;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not supported on this platform")]
    Unsupported,
    #[error("the clipboard is busy (another app has it open)")]
    ClipboardBusy,
    #[error("Windows error: {0}")]
    Os(String),
}

/// How the text was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// UI Automation: no clipboard involved.
    Accessibility,
    /// A simulated Ctrl+C with the clipboard restored afterwards.
    Clipboard,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Selection {
    pub text: String,
    /// The paragraph around the selection (UI Automation only), for single words.
    pub context: Option<String>,
    /// The selection on screen (physical pixels), when the app reports it.
    pub bounds: Option<Rect>,
    pub method: Method,
}

/// Memory, for picking a model that fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Memory {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[cfg(windows)]
pub use imp::{
    ClickOutsideWatcher, DoubleCopyWatcher, capture_selection, cursor_position, hide_window,
    kill_with_app, memory, prepare_popup_window, show_window_at, work_area_at,
};

#[cfg(not(windows))]
mod stub {
    use super::*;

    pub fn capture_selection(_allow_clipboard: bool) -> Result<Option<Selection>, Error> {
        Err(Error::Unsupported)
    }
    pub fn cursor_position() -> Option<(i32, i32)> {
        None
    }
    pub fn work_area_at(_x: i32, _y: i32) -> Option<Rect> {
        None
    }
    pub fn prepare_popup_window(_hwnd: isize) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
    pub fn show_window_at(_hwnd: isize, _x: i32, _y: i32, _w: i32, _h: i32) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
    pub fn hide_window(_hwnd: isize) {}
    pub fn kill_with_app(_pid: u32) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
    pub fn memory() -> Option<Memory> {
        None
    }

    /// Calls back with the text when Ctrl+C is pressed twice quickly (Windows only).
    pub struct DoubleCopyWatcher;
    impl DoubleCopyWatcher {
        pub fn start(
            _on_double_copy: impl Fn(String) + Send + Sync + 'static,
        ) -> Result<Self, Error> {
            Err(Error::Unsupported)
        }
    }

    /// Calls back when the mouse is pressed outside a rectangle (Windows only).
    pub struct ClickOutsideWatcher;
    impl ClickOutsideWatcher {
        pub fn start(_on_click_outside: impl Fn() + Send + Sync + 'static) -> Self {
            ClickOutsideWatcher
        }
        pub fn set_rect(&self, _rect: Option<Rect>) {}
    }
}

#[cfg(not(windows))]
pub use stub::*;

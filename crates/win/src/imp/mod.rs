//! The Windows implementation. Each piece is small and talks to Win32 directly.

mod clipboard;
mod hook;
mod input;
mod system;
mod uia;
mod window;

use std::sync::mpsc;
use std::time::Duration;

pub use clipboard::{DoubleCopyWatcher, set_text as set_clipboard_text};
pub use hook::ClickOutsideWatcher;
pub use system::{kill_with_app, memory, process_memory};
pub use window::{
    cursor_position, hide_window, prepare_popup_window, show_window_at, work_area_at,
};

use crate::{Error, Method, Selection};

/// UI Automation gets this long before the clipboard fallback is tried. Some apps answer
/// accessibility queries slowly or not at all.
const UIA_TIMEOUT: Duration = Duration::from_millis(450);

/// Reads the selected text in the app that has focus.
///
/// First asks UI Automation (no side effects). If the app doesn't expose its selection
/// and `allow_clipboard` is set, simulates Ctrl+C and restores the clipboard afterwards.
/// Returns `Ok(None)` when nothing is selected.
pub fn capture_selection(allow_clipboard: bool) -> Result<Option<Selection>, Error> {
    // UI Automation runs on its own thread so a hung app can't hang us.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(uia::selection());
    });
    match rx.recv_timeout(UIA_TIMEOUT) {
        Ok(Ok(Some(sel))) if !sel.text.trim().is_empty() => return Ok(Some(sel)),
        Ok(Err(e)) => tracing::debug!("UI Automation: {e}"),
        Err(_) => tracing::debug!("UI Automation timed out"),
        _ => {}
    }
    if !allow_clipboard {
        return Ok(None);
    }
    let text = clipboard::copy_selection()?;
    Ok(text.filter(|t| !t.trim().is_empty()).map(|text| Selection {
        text,
        context: None,
        bounds: None,
        method: Method::Clipboard,
    }))
}

fn os_err(e: windows::core::Error) -> Error {
    Error::Os(e.message())
}

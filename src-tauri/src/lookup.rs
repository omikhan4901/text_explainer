//! The Look up box: a small window with one text field, opened from the tray, its own
//! shortcut, or the main shortcut when nothing is selected. Unlike the card it takes
//! focus (it has a text field). Enter opens the card on the text where the box was; Esc
//! or clicking elsewhere hides it.

use tauri::{AppHandle, Emitter, Manager};
use te_win::{Rect, place_lookup};

use crate::popup;

pub const LABEL: &str = "lookup";
/// The box's size and the transparent margin around it for its shadow, in CSS pixels
/// (match `src/lookup/Lookup.tsx`).
const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 96.0;
const MARGIN: f64 = 12.0;

/// Shows the box, empty and focused, in the upper third of the monitor with the mouse.
/// False when the window is missing.
pub fn show(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window(LABEL) else {
        return false;
    };
    let Ok(raw) = window.hwnd().map(|h| h.0 as isize) else {
        return false;
    };
    // A new look-up replaces the card (which also frees Esc, registered while it's open).
    popup::close(app);
    let (cx, cy) = te_win::cursor_position().unwrap_or((200, 200));
    let work = te_win::work_area_at(cx, cy).unwrap_or(Rect::new(0, 0, 1280, 720));
    let scale = app
        .monitor_from_point(f64::from(cx), f64::from(cy))
        .ok()
        .flatten()
        .map_or(1.0, |m| m.scale_factor());
    let px = |css: f64| (css * scale).round() as i32;
    let size = (px(WIDTH + 2.0 * MARGIN), px(HEIGHT + 2.0 * MARGIN));
    let (x, y) = place_lookup(size, work);
    let _ = app.emit_to(LABEL, "te://lookup", ());
    if let Err(e) = te_win::show_window_at(raw, x, y, size.0, size.1) {
        tracing::warn!("couldn't show the Look up box: {e}");
        return false;
    }
    if let Err(e) = window.set_focus() {
        tracing::warn!("couldn't focus the Look up box: {e}");
    }
    true
}

/// The box's own shortcut: shows it, or hides it when it's already open.
pub fn toggle(app: &AppHandle) {
    let visible = app
        .get_webview_window(LABEL)
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    if visible {
        hide(app);
    } else {
        show(app);
    }
}

/// Hides the box. Returns the top edge of where it was (screen pixels), so the card can
/// open in its place.
pub fn hide(app: &AppHandle) -> Option<Rect> {
    let window = app.get_webview_window(LABEL)?;
    let place = window
        .outer_position()
        .ok()
        .zip(window.outer_size().ok())
        .map(|(p, s)| {
            let margin = (MARGIN * window.scale_factor().unwrap_or(1.0)).round() as i32;
            let (left, top) = (p.x + margin, p.y + margin);
            let right = p.x + s.width as i32 - margin;
            Rect::new(left, top, right.max(left + 1), top + 1)
        });
    let _ = window.hide();
    place
}

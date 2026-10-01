//! The reading card's window: opened next to the selection without taking focus, resized
//! to its content as text streams in, and closed by Esc, a click elsewhere, or its X.

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use te_core::explain::Event;
use te_win::{Rect, choose_side, place_on_side};

use crate::state::{AppState, PopupPlacement};

/// Card width and the transparent margin around it (for its shadow), in CSS pixels.
const CARD_WIDTH: f64 = 440.0;
const MARGIN: f64 = 12.0;
const INITIAL_HEIGHT: f64 = 140.0;
pub const MAX_CARD_HEIGHT: f64 = 560.0;
const GAP: f64 = 4.0;
pub const ESCAPE: &str = "Escape";

/// Everything the popup page is told.
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PopupEvent {
    /// A new request starts (the page clears and shows the source).
    Open {
        id: u64,
    },
    Explain {
        id: u64,
        event: Event,
    },
    NoSelection {
        hotkey: String,
    },
    NoModel,
    CaptureFailed {
        message: String,
    },
    /// Settings changed (appearance, level); the page reloads them.
    Settings,
}

pub fn emit(app: &AppHandle, event: PopupEvent) {
    let _ = app.emit_to("popup", "te://popup", event);
}

fn hwnd(app: &AppHandle) -> Option<isize> {
    let window = app.get_webview_window("popup")?;
    window.hwnd().ok().map(|h| h.0 as isize)
}

/// Makes the popup window non-activating. Called once at startup.
pub fn prepare(app: &AppHandle) {
    if let Some(h) = hwnd(app)
        && let Err(e) = te_win::prepare_popup_window(h)
    {
        tracing::warn!("couldn't prepare the popup window: {e}");
    }
}

/// Opens the card next to `selection` (screen pixels), or under the mouse pointer.
pub fn open(app: &AppHandle, selection: Option<Rect>) {
    let Some(h) = hwnd(app) else { return };
    let state = app.state::<AppState>();
    let (cx, cy) = te_win::cursor_position().unwrap_or((200, 200));
    let pointer = Rect::new(cx, cy - 12, cx, cy + 20);
    let work = te_win::work_area_at(cx, cy).unwrap_or(Rect::new(0, 0, 1280, 720));
    // A selection taller than a third of the screen (a whole page) is no use as an
    // anchor; the pointer is where the person is looking.
    let anchor = selection
        .filter(|r| r.height() > 0 && r.height() < work.height() / 3 && r.width() < work.width())
        .unwrap_or(pointer);
    let work = te_win::work_area_at(anchor.left, anchor.top).unwrap_or(work);
    let scale = app
        .monitor_from_point(f64::from(anchor.left), f64::from(anchor.top))
        .ok()
        .flatten()
        .map_or(1.0, |m| m.scale_factor());
    let px = |css: f64| (css * scale).round() as i32;
    let size = (
        px(CARD_WIDTH + 2.0 * MARGIN),
        px(INITIAL_HEIGHT + 2.0 * MARGIN),
    );
    let side = choose_side(anchor, px(MAX_CARD_HEIGHT + 2.0 * MARGIN), work, px(GAP));
    let (x, y) = place_on_side(anchor, size, work, px(GAP), side);
    let rect = Rect::new(x, y, x + size.0, y + size.1);
    if let Err(e) = te_win::show_window_at(h, x, y, size.0, size.1) {
        tracing::warn!("couldn't show the popup: {e}");
        return;
    }
    *state.popup.lock().expect("popup lock") = Some(PopupPlacement {
        anchor,
        work,
        side,
        scale,
        rect,
    });
    if !state.pinned.load(Ordering::SeqCst) {
        state.click_watcher.set_rect(Some(rect));
    }
    let _ = app.global_shortcut().register(ESCAPE);
}

/// Fits the window to the card's height (CSS pixels, including the margin).
pub fn resize(app: &AppHandle, css_height: f64) {
    let Some(h) = hwnd(app) else { return };
    let state = app.state::<AppState>();
    let mut popup = state.popup.lock().expect("popup lock");
    let Some(p) = popup.as_mut() else { return };
    let px = |css: f64| (css * p.scale).round() as i32;
    let height = css_height.clamp(60.0, MAX_CARD_HEIGHT + 2.0 * MARGIN);
    let size = (px(CARD_WIDTH + 2.0 * MARGIN), px(height));
    let (x, y) = place_on_side(p.anchor, size, p.work, px(GAP), p.side);
    p.rect = Rect::new(x, y, x + size.0, y + size.1);
    let _ = te_win::show_window_at(h, x, y, size.0, size.1);
    if !state.pinned.load(Ordering::SeqCst) {
        state.click_watcher.set_rect(Some(p.rect));
    }
}

/// Hides the card and stops any answer still being written.
pub fn close(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Some(task) = state.task.lock().expect("task lock").take() {
        task.abort();
    }
    state.click_watcher.set_rect(None);
    state.pinned.store(false, Ordering::SeqCst);
    *state.popup.lock().expect("popup lock") = None;
    if let Some(h) = hwnd(app) {
        te_win::hide_window(h);
    }
    let _ = app.global_shortcut().unregister(ESCAPE);
}

pub fn set_pinned(app: &AppHandle, pinned: bool) {
    let state = app.state::<AppState>();
    state.pinned.store(pinned, Ordering::SeqCst);
    let rect = state.popup.lock().expect("popup lock").map(|p| p.rect);
    state
        .click_watcher
        .set_rect(if pinned { None } else { rect });
}

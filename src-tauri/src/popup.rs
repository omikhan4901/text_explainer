//! The reading card's window: opened next to the selection without taking focus, resized
//! to its content as text streams in, and closed by Esc, a click elsewhere, or its X.
//! Web source tabs show their page in a child web view placed over the card's content
//! area (private, no access to the app, never focused on its own).

use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use tauri::webview::{Color, NewWindowResponse, WebviewBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_opener::OpenerExt;
use te_core::explain::Event;
use te_core::sources::{self, WebTab};
use te_core::text::{self, SelectionKind};
use te_core::translate::Availability;
use te_win::{Rect, choose_side, place_on_side};

use crate::state::{AppState, PopupPlacement};

/// Card width and the transparent margin around it (for its shadow), in CSS pixels.
const CARD_WIDTH: f64 = 480.0;
const MARGIN: f64 = 12.0;
const INITIAL_HEIGHT: f64 = 140.0;
pub const MAX_CARD_HEIGHT: f64 = 560.0;
const GAP: f64 = 4.0;
pub const ESCAPE: &str = "Escape";
/// The card's web view (one, reused by every web tab).
const WEB_LABEL: &str = "popup-web";

/// Everything the popup page is told.
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PopupEvent {
    /// A new request starts: what was selected, and the tabs it gets.
    Open {
        id: u64,
        ai: AiMode,
        kind: SelectionKind,
        /// The selection, shortened for the card's header.
        query: String,
        /// Web source tabs, in order (empty while "Offline only" is on).
        web: Vec<WebTab>,
        /// Whether the AI tab can translate this selection into `settings.translate_to`.
        translate: Availability,
    },
    Explain {
        id: u64,
        /// The task these events answer, so a switch drops the other task's late events.
        task: AiTask,
        event: Event,
    },
    /// The dictionary entry for a selected word (arrives before the model's answer).
    Dictionary {
        id: u64,
        entry: te_core::dictionary::Entry,
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

/// Whether the model works on a request: now, once the card's Explain tab is opened, or
/// not at all (no model set up; the dictionary entry is the whole answer).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiMode {
    Running,
    Deferred,
    Off,
}

/// What the card's AI tab asks for: an explanation at the reading level, or a translation
/// into the language chosen in Settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTask {
    #[default]
    Explain,
    Translate,
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
    // The last request's page stays hidden until the new card asks for a web tab.
    web_hide(app);
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
    // The web view's memory (a WebView2 renderer) is freed with the card.
    if let Some(view) = app.get_webview(WEB_LABEL) {
        let _ = view.close();
    }
    *state.web_url.lock().expect("web lock") = None;
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

/// Where the card's content area is, in CSS pixels from the window's top left.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct WebBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The page a web source shows for the current selection.
pub fn web_url(app: &AppHandle, source_id: &str) -> Result<tauri::Url, String> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    if settings.offline_only {
        return Err("Web sources are off (Offline only).".into());
    }
    let source = settings
        .sources
        .iter()
        .find(|s| s.id == source_id && s.enabled)
        .ok_or("That source is turned off.")?;
    let selection = state
        .last
        .lock()
        .expect("last lock")
        .as_ref()
        .map(|l| text::clean(&l.text))
        .ok_or("Nothing is selected.")?;
    sources::build_url(&source.url, &selection, &settings.translate_to).map_err(|e| e.to_string())
}

/// Shows `source_id`'s page in the card's content area, creating the web view the first
/// time. Returns the address shown.
pub fn web_show(
    app: &AppHandle,
    source_id: &str,
    bounds: WebBounds,
    background: Option<&str>,
) -> Result<String, String> {
    let url = web_url(app, source_id)?;
    let state = app.state::<AppState>();
    let position = LogicalPosition::new(bounds.x, bounds.y);
    let size = LogicalSize::new(bounds.width.max(1.0), bounds.height.max(1.0));
    let mut shown = state.web_url.lock().expect("web lock");
    if let Some(view) = app.get_webview(WEB_LABEL) {
        view.set_position(position).map_err(|e| e.to_string())?;
        view.set_size(size).map_err(|e| e.to_string())?;
        if shown.as_deref() != Some(url.as_str()) {
            view.navigate(url.clone()).map_err(|e| e.to_string())?;
        }
        view.show().map_err(|e| e.to_string())?;
    } else {
        let window = app
            .get_window("popup")
            .ok_or("The card's window is missing.")?;
        let opener = app.clone();
        let mut builder = WebviewBuilder::new(WEB_LABEL, WebviewUrl::External(url.clone()))
            // A private profile: nothing is kept after the card closes.
            .incognito(true)
            // Reading never takes focus from the app underneath; clicking into it does.
            .focused(false)
            .on_navigation(|u| matches!(u.scheme(), "https" | "http"))
            // Links that want a new window open in the person's browser instead.
            .on_new_window(move |u, _| {
                if matches!(u.scheme(), "https" | "http") {
                    let _ = opener.opener().open_url(u.as_str(), None::<&str>);
                }
                NewWindowResponse::Deny
            });
        if let Some(color) = background.and_then(parse_hex) {
            builder = builder.background_color(color);
        }
        window
            .add_child(builder, position, size)
            .map_err(|e| e.to_string())?;
    }
    *shown = Some(url.to_string());
    Ok(url.to_string())
}

/// Hides the web view (another tab was opened, or a new request started).
pub fn web_hide(app: &AppHandle) {
    if let Some(view) = app.get_webview(WEB_LABEL) {
        let _ = view.hide();
    }
}

/// "#1f1f1f" → an opaque colour.
fn parse_hex(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(Color((n >> 16) as u8, (n >> 8) as u8, n as u8, 255))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_card_colours() {
        assert_eq!(parse_hex("#1f2e3d"), Some(Color(0x1f, 0x2e, 0x3d, 255)));
        assert_eq!(parse_hex("1f2e3d"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex("#zzzzzz"), None);
    }
}

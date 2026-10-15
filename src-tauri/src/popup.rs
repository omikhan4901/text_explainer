//! The reading card's window: opened next to the selection without taking focus (the
//! person keeps typing where they were), resized to its content as text streams in, and
//! closed by its X (or Esc). It takes focus once the person clicks in it, so its own
//! controls, the web page and Windows' own moving and resizing all work as in any window.
//! Web source tabs show their page in a child web view placed over the card's content
//! area (private, no access to the app). The size the person gives the card is remembered.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::webview::{Color, NewWindowResponse, WebviewBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_opener::OpenerExt;
use te_core::explain::Event;
use te_core::settings::CardSize;
use te_core::sources::{self, WebTab};
use te_core::text::{self, SelectionKind};
use te_core::translate::Offer;
use te_win::{Rect, SizeMove, choose_side, place_on_side};

use crate::state::{AppState, PopupPlacement};

/// Card width and the transparent margin around it (for its shadow), in CSS pixels.
const CARD_WIDTH: f64 = 480.0;
const MARGIN: f64 = 12.0;
const INITIAL_HEIGHT: f64 = 140.0;
/// The window's size before the page first measures the card (CSS pixels, with margin).
pub const INITIAL_SIZE: (f64, f64) = (CARD_WIDTH + 2.0 * MARGIN, INITIAL_HEIGHT + 2.0 * MARGIN);
pub const MAX_CARD_HEIGHT: f64 = 560.0;
const GAP: f64 = 4.0;
pub const ESCAPE: &str = "Escape";
/// The card's web view (one, reused by every web tab).
const WEB_LABEL: &str = "popup-web";
/// Phones' pages fit a card-sized view; desktop pages scroll sideways in one.
const WEB_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36";
/// The popup's window handle, for the mouse hook's thread (asking Tauri for it there
/// would wait on the main thread).
static POPUP_HWND: OnceLock<isize> = OnceLock::new();
/// The person is moving or resizing the card: the page's size reports wait until they
/// let go.
static DRAGGING: AtomicBool = AtomicBool::new(false);

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
        /// What the AI tab can translate this selection into, and its own language.
        translate: Offer,
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

/// Makes the popup window non-activating and watches the person move and resize it.
/// Called once at startup, on the main thread.
pub fn prepare(app: &AppHandle) {
    let Some(h) = hwnd(app) else { return };
    let _ = POPUP_HWND.set(h);
    if let Err(e) = te_win::prepare_popup_window(h) {
        tracing::warn!("couldn't prepare the popup window: {e}");
    }
    let handle = app.clone();
    let min = (
        (CardSize::MIN.width + 2.0 * MARGIN) as i32,
        (CardSize::MIN.height + 2.0 * MARGIN) as i32,
    );
    if let Err(e) = te_win::watch_size_move(h, min, move |change| size_move(&handle, change)) {
        tracing::warn!("{e}");
    }
}

/// Opens the card next to `selection` (screen pixels), or under the mouse pointer.
pub fn open(app: &AppHandle, selection: Option<Rect>) {
    let Some(h) = hwnd(app) else { return };
    // The last request's page stays hidden until the new card asks for a web tab.
    web_hide(app);
    let state = app.state::<AppState>();
    release_focus(&state, h);
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
    // The size the person gave the card, or the card as the page last measured it: the
    // page reports only when that changes, and the next card is often the same size.
    let (width, height) = state.settings().card_size.map_or_else(
        || *state.card_size.lock().expect("card size lock"),
        |c| (c.width + 2.0 * MARGIN, c.height + 2.0 * MARGIN),
    );
    let size = (px(width), px(height));
    let tallest = px(MAX_CARD_HEIGHT + 2.0 * MARGIN).max(size.1);
    let side = choose_side(anchor, tallest, work, px(GAP));
    let (x, y) = place_on_side(anchor, size, work, px(GAP), side);
    let rect = Rect::new(x, y, x + size.0, y + size.1);
    if let Err(e) = te_win::show_window_at(h, x, y, size.0, size.1) {
        tracing::warn!("couldn't show the popup: {e}");
        return;
    }
    let placement = PopupPlacement {
        anchor,
        work,
        side,
        scale,
        rect,
        placed: false,
    };
    *state.popup.lock().expect("popup lock") = Some(placement);
    watch(&state, &placement);
    let _ = app.global_shortcut().register(ESCAPE);
}

/// Fits the window to the card (CSS pixels, including the margin). A card the person
/// sized fills its window instead, so this only applies to cards sized by their content.
pub fn resize(app: &AppHandle, width: f64, height: f64) {
    let state = app.state::<AppState>();
    *state.card_size.lock().expect("card size lock") = (width, height);
    if DRAGGING.load(Ordering::SeqCst) || state.settings().card_size.is_some() {
        return;
    }
    let Some(h) = hwnd(app) else { return };
    let mut popup = state.popup.lock().expect("popup lock");
    let Some(p) = popup.as_mut() else { return };
    let px = |css: f64| (css * p.scale).round() as i32;
    let size = (
        px(width).clamp(px(100.0), p.work.width()),
        px(height).clamp(px(60.0), p.work.height()),
    );
    let (x, y) = if p.placed {
        (p.rect.left, p.rect.top)
    } else {
        place_on_side(p.anchor, size, p.work, px(GAP), p.side)
    };
    p.rect = Rect::new(x, y, x + size.0, y + size.1);
    let _ = te_win::show_window_at(h, x, y, size.0, size.1);
    watch(&state, p);
}

/// What the person drags: the card's header moves it, its corner resizes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drag {
    Move,
    Resize,
}

/// The card's header or corner was pressed: Windows' own moving or resizing takes over
/// (the card is the active window by now, see `focus_card`), and `size_move` hears how
/// it went.
pub fn drag(app: &AppHandle, kind: Drag) {
    let Some(window) = app.get_window("popup") else {
        return;
    };
    let result = match kind {
        Drag::Move => window.start_dragging(),
        Drag::Resize => window.start_resize_dragging(tauri_runtime::ResizeDirection::SouthEast),
    };
    if let Err(e) = result {
        tracing::warn!("couldn't start {kind:?}: {e}");
    }
}

/// Windows reports the person moving or resizing the card (from its header, corner or
/// edges). Runs on the main thread, inside the window's own message handling.
fn size_move(app: &AppHandle, change: SizeMove) {
    let state = app.state::<AppState>();
    match change {
        SizeMove::Start => DRAGGING.store(true, Ordering::SeqCst),
        // The first resize step: from now on the card fills its window.
        SizeMove::Sizing => {
            let mut settings = state.settings.lock().expect("settings lock");
            if settings.card_size.is_none() {
                let (w, h) = *state.card_size.lock().expect("card size lock");
                settings.card_size = Some(CardSize {
                    width: w - 2.0 * MARGIN,
                    height: h - 2.0 * MARGIN,
                });
                drop(settings);
                emit(app, PopupEvent::Settings);
            }
        }
        SizeMove::End(rect) => {
            DRAGGING.store(false, Ordering::SeqCst);
            let resized = {
                let mut popup = state.popup.lock().expect("popup lock");
                let Some(p) = popup.as_mut() else { return };
                let resized = (rect.width(), rect.height()) != (p.rect.width(), p.rect.height());
                if rect != p.rect {
                    p.placed = true;
                    p.rect = rect;
                }
                watch(&state, p);
                resized.then_some(p.scale)
            };
            match resized {
                Some(scale) => {
                    let size = CardSize {
                        width: f64::from(rect.width()) / scale - 2.0 * MARGIN,
                        height: f64::from(rect.height()) / scale - 2.0 * MARGIN,
                    };
                    tracing::info!("card resized to {} x {}", size.width, size.height);
                    if let Err(e) = save_card_size(app, Some(size)) {
                        tracing::warn!("{e}");
                    }
                }
                None => {
                    // A move: fit any text that arrived meanwhile, and tell the page (a
                    // press on the corner that didn't resize stops it filling the window).
                    let (w, h) = *state.card_size.lock().expect("card size lock");
                    resize(app, w, h);
                    emit(app, PopupEvent::Settings);
                }
            }
        }
    }
}

/// Remembers the size the person gave the card (`None` fits it to what it shows again)
/// and tells the card.
pub fn save_card_size(app: &AppHandle, size: Option<CardSize>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state.settings();
    settings.card_size = size;
    let settings = settings.sanitized();
    settings
        .save(&state.paths.settings)
        .map_err(|e| format!("Couldn't save settings: {e}"))?;
    *state.settings.lock().expect("settings lock") = settings;
    emit(app, PopupEvent::Settings);
    Ok(())
}

/// The person clicked in another app: the card stays open, and Esc goes back to that app.
pub fn clicked_elsewhere(app: &AppHandle) {
    let _ = app.global_shortcut().unregister(ESCAPE);
}

/// Tells the mouse hook where the card is.
fn watch(state: &AppState, p: &PopupPlacement) {
    state.click_watcher.set_rect(Some(p.rect));
}

/// The person pressed the mouse inside the card. Runs on the mouse hook's thread before
/// the press arrives, so the card is activatable in time for that press to activate it:
/// its buttons, the web page's search box and Windows' own moving and resizing then work
/// as in any window. The card becomes non-activating again when it closes or reopens.
pub fn focus_card(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(&h) = POPUP_HWND.get() else { return };
    if state.focused.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Err(e) = te_win::set_popup_focus(h, true) {
        tracing::warn!("couldn't let the card take focus: {e}");
    }
    // The card has the keyboard now, and closes on Esc itself.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = handle.global_shortcut().unregister(ESCAPE);
    });
}

/// Makes the card non-activating again.
fn release_focus(state: &AppState, h: isize) {
    if state.focused.swap(false, Ordering::SeqCst)
        && let Err(e) = te_win::set_popup_focus(h, false)
    {
        tracing::warn!("couldn't give focus back: {e}");
    }
}

/// Hides the card and stops any answer still being written. Unless the person chose to
/// keep it loaded, the model's memory is freed too (the AI tab loads it again).
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
    DRAGGING.store(false, Ordering::SeqCst);
    let was_open = state.popup.lock().expect("popup lock").take().is_some();
    if let Some(h) = hwnd(app) {
        te_win::hide_window(h);
        release_focus(&state, h);
    }
    let _ = app.global_shortcut().unregister(ESCAPE);
    if was_open && state.settings().unload_on_close {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = handle.state::<AppState>();
            let mut engine = state.engine.lock().await;
            // A new card may have opened while an answer held the engine.
            if state.popup.lock().expect("popup lock").is_none() && engine.is_loaded() {
                tracing::info!("the card closed: freeing the model's memory");
                engine.unload().await;
            }
        });
    }
}

/// Where the card's content area is, in CSS pixels from the window's top left.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
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
            .user_agent(WEB_USER_AGENT)
            // Showing it never takes focus; clicking into it does (`focus_card`).
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

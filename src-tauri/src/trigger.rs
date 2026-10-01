//! From "the person asked" to an answer streaming into the card: the hotkey reads the
//! selection (UI Automation, then the clipboard fallback); a double Ctrl+C hands us the
//! copied text directly.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use te_core::engine::{Backend, Sampling};
use te_core::explain::{self, Event, Request};
use te_core::text::{self, SelectionKind};
use te_win::Rect;

use crate::popup::{self, PopupEvent};
use crate::state::{AppState, LastRequest};

/// The hotkey was pressed.
pub fn on_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.paused.load(Ordering::SeqCst) {
        return;
    }
    let allow_clipboard = state.settings().clipboard_fallback;
    let app = app.clone();
    // Reading the selection can take a moment (and simulates keys); never on the UI thread.
    std::thread::spawn(move || {
        let result = te_win::capture_selection(allow_clipboard);
        let main = app.clone();
        let _ = main.run_on_main_thread(move || match result {
            Ok(Some(sel)) => start(&app, sel.text, sel.context, sel.bounds),
            Ok(None) => {
                popup::open(&app, None);
                let hotkey = app.state::<AppState>().settings().hotkey;
                popup::emit(&app, PopupEvent::NoSelection { hotkey });
            }
            Err(e) => {
                popup::open(&app, None);
                popup::emit(
                    &app,
                    PopupEvent::CaptureFailed {
                        message: e.to_string(),
                    },
                );
            }
        });
    });
}

/// Ctrl+C was pressed twice on the same text.
pub fn on_double_copy(app: &AppHandle, text: String) {
    let state = app.state::<AppState>();
    if state.paused.load(Ordering::SeqCst) || !state.settings().double_copy {
        return;
    }
    start(app, text, None, None);
}

pub fn start(app: &AppHandle, text: String, context: Option<String>, bounds: Option<Rect>) {
    let state = app.state::<AppState>();
    *state.last.lock().expect("last lock") = Some(LastRequest { text, context });
    popup::open(app, bounds);
    run(app);
}

/// Explains the last selection with the current settings (also used when the reading
/// level changes).
pub fn run(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(last) = state.last.lock().expect("last lock").clone() else {
        return;
    };
    let settings = state.settings();
    let backend = state.backend(&settings);
    let id = state.request_id.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(previous) = state.task.lock().expect("task lock").take() {
        previous.abort();
    }

    // Single words and short terms: the dictionary answers at once.
    let cleaned = text::clean(&last.text);
    let entry = match text::classify(&cleaned) {
        SelectionKind::Word | SelectionKind::Phrase => state
            .dictionary
            .lock()
            .expect("dictionary lock")
            .as_ref()
            .and_then(|d| d.lookup(&cleaned)),
        _ => None,
    };
    if backend == Backend::None && entry.is_none() {
        popup::emit(app, PopupEvent::NoModel);
        return;
    }
    popup::emit(app, PopupEvent::Open { id });
    if let Some(entry) = entry {
        popup::emit(app, PopupEvent::Dictionary { id, entry });
    }
    if backend == Backend::None {
        // No model yet: the dictionary entry is the whole answer.
        return;
    }

    let engine = state.engine.clone();
    let app2 = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let mut engine = engine.lock().await;
        engine.set_backend(backend).await;
        let request = Request {
            text: last.text,
            context: last.context,
            level: settings.level,
            output: settings.output.clone(),
            custom_prompt: settings.prompts.for_level(settings.level).map(String::from),
            sampling: Sampling {
                temperature: settings.temperature,
                top_p: settings.top_p,
                max_tokens: 512,
            },
            part_words: 180,
        };
        let emitter = app2.clone();
        let mut emit = move |event: Event| popup::emit(&emitter, PopupEvent::Explain { id, event });
        if let Err(e) = explain::run(&mut engine, &request, &mut emit).await {
            tracing::info!("explain request {id} ended: {e}");
        }
    });
    *state.task.lock().expect("task lock") = Some(task);
}

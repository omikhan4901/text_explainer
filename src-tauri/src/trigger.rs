//! From "the person asked" to an answer streaming into the card: the hotkey reads the
//! selection (UI Automation, then the clipboard fallback); a double Ctrl+C hands us the
//! copied text directly. Neither does anything in an app on the "Don't open in these
//! apps" list (`te_core::rules`).

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use te_core::engine::{Backend, Sampling};
use te_core::explain::{self, Event, Request};
use te_core::rules;
use te_core::sources;
use te_core::text::{self, SelectionKind};
use te_win::Rect;

use crate::lookup;
use crate::popup::{self, AiMode, PopupEvent};
use crate::state::{AppState, LastRequest};

/// Notes the app in front (for "Add <app>" in Settings) and says whether the card may
/// read from it: false for apps on the "Don't open in these apps" list.
fn allowed_in_foreground(app: &AppHandle) -> bool {
    let Some(name) = te_win::foreground_process_name().and_then(|n| rules::normalize(&n).ok())
    else {
        return true;
    };
    let state = app.state::<AppState>();
    let own = std::env::current_exe()
        .ok()
        .and_then(|p| rules::normalize(&p.to_string_lossy()).ok());
    if own.as_ref() != Some(&name) {
        *state.last_app.lock().expect("last app lock") = Some(name.clone());
    }
    if rules::is_excluded(&state.settings().excluded_apps, &name) {
        tracing::debug!("not opening in {name} (on the excluded list)");
        return false;
    }
    true
}

/// The hotkey was pressed.
pub fn on_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.paused.load(Ordering::SeqCst) || !allowed_in_foreground(app) {
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
            // Nothing selected: type what to look up instead.
            Ok(None) => {
                if !lookup::show(&app) {
                    popup::open(&app, None);
                    let hotkey = app.state::<AppState>().settings().hotkey;
                    popup::emit(&app, PopupEvent::NoSelection { hotkey });
                }
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
    if state.paused.load(Ordering::SeqCst)
        || !state.settings().double_copy
        || !allowed_in_foreground(app)
    {
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

/// Opens the card on the last selection with the current settings (also used when the
/// reading level changes). The card's tabs come first: web searches, then a word's
/// dictionary entry. The model only starts when the person opens the "Explain with AI" tab
/// (`explain_now`), or at once when there is nothing else to show.
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
    let kind = text::classify(&cleaned);
    let word = matches!(kind, SelectionKind::Word | SelectionKind::Phrase);
    let web = sources::tabs_for(&settings.sources, settings.offline_only, word);
    let entry = match kind {
        SelectionKind::Word | SelectionKind::Phrase => state
            .dictionary
            .lock()
            .expect("dictionary lock")
            .as_ref()
            .and_then(|d| d.lookup(&cleaned)),
        _ => None,
    };
    let something_else = !web.is_empty() || entry.is_some();
    if backend == Backend::None && !something_else {
        popup::emit(app, PopupEvent::NoModel);
        return;
    }
    let ai = if backend == Backend::None {
        AiMode::Off
    } else if something_else {
        AiMode::Deferred
    } else {
        AiMode::Running
    };
    crate::words::record(app, &cleaned, kind);
    popup::emit(
        app,
        PopupEvent::Open {
            id,
            ai,
            kind,
            query: header_text(&cleaned),
            web,
        },
    );
    if let Some(entry) = entry {
        popup::emit(app, PopupEvent::Dictionary { id, entry });
    }
    if ai == AiMode::Running {
        explain(app, id);
    }
}

/// The selection as the card's header shows it: one line, at most 120 characters.
fn header_text(cleaned: &str) -> String {
    let line = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    match line.char_indices().nth(120) {
        Some((cut, _)) => format!("{}…", line[..cut].trim_end()),
        None => line,
    }
}

/// The card's Explain tab was opened on request `id`: start the model if that request is
/// still the current one.
pub fn explain_now(app: &AppHandle, id: u64) {
    let state = app.state::<AppState>();
    if state.request_id.load(Ordering::SeqCst) != id {
        return;
    }
    if state.backend(&state.settings()) == Backend::None {
        return;
    }
    explain(app, id);
}

/// Explains the current request again (the reading level changed); its "started" event
/// clears the card's previous answer.
pub fn explain_again(app: &AppHandle) {
    let id = app.state::<AppState>().request_id.load(Ordering::SeqCst);
    explain_now(app, id);
}

/// Streams the model's answer for the last selection into the card as request `id`.
fn explain(app: &AppHandle, id: u64) {
    let state = app.state::<AppState>();
    let Some(last) = state.last.lock().expect("last lock").clone() else {
        return;
    };
    let settings = state.settings();
    let backend = state.backend(&settings);
    if let Some(previous) = state.task.lock().expect("task lock").take() {
        previous.abort();
    }
    let cleaned = text::clean(&last.text);
    // Capitalised ordinary words ("Lessee", "Agreement") aren't names for the meaning check.
    let common_words: Vec<String> = state
        .dictionary
        .lock()
        .expect("dictionary lock")
        .as_ref()
        .map(|d| {
            te_core::meaning::single_word_names(&cleaned)
                .into_iter()
                .filter(|w| d.is_common_word(w))
                .map(|w| w.to_lowercase())
                .collect()
        })
        .unwrap_or_default();

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
            common_words,
        };
        let emitter = app2.clone();
        let mut emit = move |event: Event| popup::emit(&emitter, PopupEvent::Explain { id, event });
        if let Err(e) = explain::run(&mut engine, &request, &mut emit).await {
            tracing::info!("explain request {id} ended: {e}");
        }
    });
    *state.task.lock().expect("task lock") = Some(task);
}

#[cfg(test)]
mod tests {
    use super::header_text;

    #[test]
    fn header_is_one_short_line() {
        assert_eq!(header_text("two\n\nlines  here"), "two lines here");
        let long = "ভা".repeat(200);
        let h = header_text(&long);
        assert_eq!(h.chars().count(), 121);
        assert!(h.ends_with('…'));
    }
}

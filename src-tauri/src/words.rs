//! The Word book (saved words), the lookup history and Listen: commands for the card and
//! the Words page. The database work runs on a blocking thread, never the UI thread, and
//! the book's lock is never held across an await.

use std::path::PathBuf;
use std::sync::MutexGuard;

use tauri::{AppHandle, Manager};
use te_core::history::{
    self, ExportFormat, History, Kind, Lookup, NewWord, Page, Retention, SavedWord,
};
use te_core::text::{self, SelectionKind};

use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

/// The most text Listen reads aloud.
const MAX_SPEAK_CHARS: usize = 200;

fn book(state: &AppState) -> MutexGuard<'_, History> {
    // A panic while holding the lock leaves the connection usable; carry on with it.
    state
        .history
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Runs `f` on the book on a blocking thread.
async fn with_book<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&mut History) -> history::Result<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&mut book(&app.state::<AppState>())))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Records that the card opened on a selection, if the person keeps a history.
pub fn record(app: &AppHandle, cleaned: &str, kind: SelectionKind) {
    let Some(kind) = Kind::of(kind) else {
        return;
    };
    let settings = app.state::<AppState>().settings();
    if !settings.keep_history {
        return;
    }
    let retention = Retention {
        keep: true,
        days: settings.history_days,
    };
    // The history keeps a few hundred characters; don't carry a whole page along.
    let query = history::truncate_chars(cleaned, history::MAX_QUERY_CHARS * 2);
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = book(&app.state::<AppState>()).record(&query, kind, retention) {
            tracing::warn!("couldn't record the lookup: {e}");
        }
    });
}

/// Saves a word from the card (or updates it when it's saved already). The paragraph
/// around the selection, when the app reported one, is kept as its context.
#[tauri::command]
pub async fn save_word(app: AppHandle, mut word: NewWord) -> CmdResult<SavedWord> {
    if word.context.is_none() {
        let last = app
            .state::<AppState>()
            .last
            .lock()
            .ok()
            .and_then(|l| l.clone());
        if let Some(last) = last
            && history::term_key(&text::clean(&last.text)).contains(&history::term_key(&word.term))
        {
            word.context = last.context;
        }
    }
    with_book(app, move |b| b.save(&word)).await
}

#[tauri::command]
pub async fn remove_word(app: AppHandle, term: String) -> CmdResult<bool> {
    with_book(app, move |b| b.remove(&term)).await
}

#[tauri::command]
pub async fn is_saved(app: AppHandle, term: String) -> CmdResult<bool> {
    with_book(app, move |b| b.is_saved(&term)).await
}

#[tauri::command]
pub async fn list_words(
    app: AppHandle,
    search: Option<String>,
    limit: u32,
    offset: u32,
) -> CmdResult<Page<SavedWord>> {
    with_book(app, move |b| {
        b.words(search.as_deref().unwrap_or(""), limit, offset)
    })
    .await
}

#[tauri::command]
pub async fn list_history(app: AppHandle, limit: u32, offset: u32) -> CmdResult<Page<Lookup>> {
    with_book(app, move |b| b.history(limit, offset)).await
}

#[tauri::command]
pub async fn clear_history(app: AppHandle) -> CmdResult<usize> {
    with_book(app, |b| b.clear_history()).await
}

/// Writes every saved word to `path` (from the save dialog); returns how many.
#[tauri::command]
pub async fn export_words(app: AppHandle, format: ExportFormat, path: PathBuf) -> CmdResult<usize> {
    let path = format
        .file_path(&path)
        .ok_or("Choose where to save the file.")?;
    let words = with_book(app, |b| b.all_words()).await?;
    let count = words.len();
    tauri::async_runtime::spawn_blocking(move || {
        // Written next to the target, then renamed: never a half-written file.
        let tmp = path.with_extension("export.tmp");
        std::fs::write(&tmp, format.render(&words))
            .and_then(|()| std::fs::rename(&tmp, &path))
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                format!("Couldn't write {}: {e}", path.display())
            })
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(count)
}

/// Reads a word or short term aloud with the Windows voice (offline).
#[tauri::command]
pub async fn speak(text: String) -> CmdResult<()> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Ok(());
    }
    if text.chars().count() > MAX_SPEAK_CHARS {
        return Err("That is too long to read aloud.".into());
    }
    tauri::async_runtime::spawn_blocking(move || te_win::speak(&text))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

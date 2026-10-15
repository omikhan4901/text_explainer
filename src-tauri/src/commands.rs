//! Commands the front end calls.

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;
use te_core::download::{self, Cancel, Download};
use te_core::language::LANGUAGES;
use te_core::models::{self, CATALOG, Fit, Tier};
use te_core::prompt::{self, Level, OutputLanguage};
use te_core::settings::{CardSize, ModelChoice, Settings};
use te_core::translate;

use crate::popup::{self, AiTask, PopupEvent};
use crate::state::AppState;
use crate::{hotkey, lookup, trigger};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Saves settings and applies what changed (shortcuts, start with Windows, model).
#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> CmdResult<Settings> {
    if hotkey::same(&settings.hotkey, &settings.lookup_hotkey) {
        return Err("That shortcut can't be used: it already opens the card.".into());
    }
    let settings = settings.sanitized();
    let state = app.state::<AppState>();
    let previous = state.settings();
    if settings.hotkey != previous.hotkey {
        hotkey::apply(&app, &settings.hotkey)
            .map_err(|e| format!("That shortcut can't be used: {e}"))?;
    }
    if settings.lookup_hotkey != previous.lookup_hotkey {
        hotkey::apply_lookup(&app, &settings.lookup_hotkey)
            .map_err(|e| format!("That shortcut can't be used: {e}"))?;
    }
    settings
        .save(&state.paths.settings)
        .map_err(|e| format!("Couldn't save settings: {e}"))?;
    *state.settings.lock().expect("settings lock") = settings.clone();
    if settings.start_with_windows != previous.start_with_windows {
        let autostart = app.autolaunch();
        let _ = if settings.start_with_windows {
            autostart.enable()
        } else {
            autostart.disable()
        };
    }
    if state.backend(&settings) != state.backend(&previous) {
        // Load the new model on the next request; free the old one now.
        let engine = state.engine.clone();
        let backend = state.backend(&settings);
        tauri::async_runtime::spawn(async move {
            engine.lock().await.set_backend(backend).await;
        });
    }
    popup::emit(&app, PopupEvent::Settings);
    let _ = app.emit_to("main", "te://settings", ());
    Ok(settings)
}

#[derive(Serialize)]
pub struct ModelInfo {
    #[serde(flatten)]
    model: &'static models::CatalogModel,
    ram_needed_bytes: u64,
    fit: Fit,
    installed: bool,
    /// Bytes of an unfinished download, if any.
    partial_bytes: u64,
}

#[derive(Serialize)]
pub struct LanguageInfo {
    code: &'static str,
    name: &'static str,
    native: &'static str,
}

#[derive(Serialize)]
pub struct TranslateLanguage {
    #[serde(flatten)]
    choice: translate::Choice,
    ai: bool,
    /// The guard's code for it ("zh" for "zh-CN"), to tell when a selection is already in it.
    lang: Option<&'static str>,
}

#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    models: Vec<ModelInfo>,
    recommended: &'static str,
    total_ram_bytes: Option<u64>,
    languages: Vec<LanguageInfo>,
    /// The "Translate into" choices; `ai` is false for those only the web tab can do.
    translate_languages: Vec<TranslateLanguage>,
    engine_found: bool,
    /// This build can update itself (it carries the update key).
    updates: bool,
    model_loaded: bool,
    downloading: Option<String>,
    paused: bool,
    /// False when the shortcut couldn't be registered (another app has it).
    hotkey_active: bool,
    /// The built-in prompts, shown in the prompt editor.
    default_prompts: DefaultPrompts,
}

#[derive(Serialize)]
pub struct DefaultPrompts {
    simpler: String,
    plain: String,
    clearer: String,
}

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> CmdResult<AppInfo> {
    let memory = te_win::memory();
    let total = memory.map(|m| m.total_bytes);
    let models = CATALOG
        .iter()
        .map(|m| {
            let path = state.paths.model_file(m);
            ModelInfo {
                model: m,
                ram_needed_bytes: m.ram_needed_bytes(),
                fit: total.map_or(Fit::Tight, |t| m.fit(t)),
                installed: path.is_file(),
                partial_bytes: std::fs::metadata(download::part_path(&path))
                    .map_or(0, |md| md.len()),
            }
        })
        .collect();
    let recommended = total.map_or(models::DEFAULT_MODEL, |t| models::recommend(t).id);
    let named = |level| prompt::rewrite_system(level, OutputLanguage::Named(&LANGUAGES[0]));
    let model_loaded = match state.engine.try_lock() {
        Ok(mut e) => e.is_loaded(),
        Err(_) => true,
    };
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        models,
        recommended,
        total_ram_bytes: total,
        languages: LANGUAGES
            .iter()
            .map(|l| LanguageInfo {
                code: l.code,
                name: l.name,
                native: l.native,
            })
            .collect(),
        translate_languages: translate::CHOICES
            .iter()
            .map(|&choice| TranslateLanguage {
                choice,
                ai: translate::target(choice.code).is_some(),
                lang: translate::target(choice.code).map(|l| l.code),
            })
            .collect(),
        engine_found: state.paths.llama_server.is_file(),
        updates: crate::updater::available(&app),
        model_loaded,
        downloading: state
            .download
            .lock()
            .expect("download lock")
            .as_ref()
            .map(|(id, _)| id.clone()),
        paused: state.paused.load(Ordering::SeqCst),
        hotkey_active: state.hotkey.lock().expect("hotkey lock").is_some(),
        default_prompts: DefaultPrompts {
            simpler: named(Level::Simpler).replace("English", "{language}"),
            plain: named(Level::Plain).replace("English", "{language}"),
            clearer: named(Level::Clearer).replace("English", "{language}"),
        },
    })
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum DownloadEvent {
    Progress {
        id: String,
        downloaded: u64,
        total: u64,
        bytes_per_sec: f64,
    },
    Done {
        id: String,
    },
    Failed {
        id: String,
        message: String,
    },
    Cancelled {
        id: String,
    },
}

/// Downloads a catalog model; it becomes the active model when done (if `activate`).
#[tauri::command]
pub async fn download_model(app: AppHandle, id: String, activate: bool) -> CmdResult<()> {
    let model = models::by_id(&id).ok_or("Unknown model")?;
    let state = app.state::<AppState>();
    let cancel = Cancel::new();
    {
        let mut current = state.download.lock().expect("download lock");
        if current.is_some() {
            return Err("Another download is running.".into());
        }
        *current = Some((id.clone(), cancel.clone()));
    }
    let dest = state.paths.model_file(model);
    let emitter = app.clone();
    let progress_id = id.clone();
    let result = download::download(
        Download {
            url: model.url,
            dest: &dest,
            sha256: Some(model.sha256),
            size: Some(model.size_bytes),
        },
        &cancel,
        move |p| {
            let _ = emitter.emit_to(
                "main",
                "te://download",
                DownloadEvent::Progress {
                    id: progress_id.clone(),
                    downloaded: p.downloaded,
                    total: p.total,
                    bytes_per_sec: p.bytes_per_sec,
                },
            );
        },
    )
    .await;
    *state.download.lock().expect("download lock") = None;
    let event = match &result {
        Ok(()) => DownloadEvent::Done { id: id.clone() },
        Err(download::DownloadError::Cancelled) => DownloadEvent::Cancelled { id: id.clone() },
        Err(e) => DownloadEvent::Failed {
            id: id.clone(),
            message: e.to_string(),
        },
    };
    let _ = app.emit_to("main", "te://download", event);
    match result {
        Ok(()) => {
            if activate {
                let mut settings = state.settings();
                settings.model = ModelChoice::Catalog { id };
                save_settings(app.clone(), settings).await?;
            }
            Ok(())
        }
        Err(download::DownloadError::Cancelled) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    if let Some((_, cancel)) = state.download.lock().expect("download lock").as_ref() {
        cancel.cancel();
    }
}

/// Deletes a downloaded catalog model (and any unfinished download of it).
#[tauri::command]
pub async fn delete_model(app: AppHandle, id: String) -> CmdResult<()> {
    let model = models::by_id(&id).ok_or("Unknown model")?;
    let state = app.state::<AppState>();
    let mut settings = state.settings();
    if settings.model == (ModelChoice::Catalog { id: id.clone() }) {
        state.engine.lock().await.unload().await;
        settings.model = ModelChoice::None;
        save_settings(app.clone(), settings).await?;
    }
    let path = state.paths.model_file(model);
    let _ = std::fs::remove_file(download::part_path(&path));
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Couldn't delete the model: {e}")),
    }
}

#[tauri::command]
pub async fn unload_model(state: State<'_, AppState>) -> CmdResult<()> {
    state.engine.lock().await.unload().await;
    Ok(())
}

#[tauri::command]
pub fn popup_resize(app: AppHandle, width: f64, height: f64) {
    popup::resize(&app, width, height);
}

/// The person pressed the card's header (move) or its corner (resize).
#[tauri::command]
pub fn popup_drag_start(app: AppHandle, kind: popup::Drag) {
    popup::drag_start(&app, kind);
}

/// The mouse moved while the header or corner is held.
#[tauri::command]
pub fn popup_drag_move(app: AppHandle) {
    popup::drag_to(&app);
}

/// The mouse button was released.
#[tauri::command]
pub fn popup_drag_end(app: AppHandle) {
    popup::drag_end(&app);
}

/// Remembers the size the person gave the card (`None`: fit it to what it shows again).
#[tauri::command]
pub fn set_card_size(app: AppHandle, size: Option<CardSize>) -> CmdResult<()> {
    popup::save_card_size(&app, size)
}

#[tauri::command]
pub fn popup_close(app: AppHandle) {
    popup::close(&app);
}

/// Changes the reading level from the card and explains the same text again (the card
/// stays on its AI tab).
#[tauri::command]
pub async fn set_level(app: AppHandle, level: Level) -> CmdResult<()> {
    let state = app.state::<AppState>();
    let mut settings = state.settings();
    if settings.level != level {
        settings.level = level;
        settings
            .save(&state.paths.settings)
            .map_err(|e| format!("Couldn't save settings: {e}"))?;
        *state.settings.lock().expect("settings lock") = settings;
    }
    trigger::explain_again(&app);
    Ok(())
}

/// The card's Explain tab was opened on a word: start the model for request `id`.
#[tauri::command]
pub fn explain_now(app: AppHandle, id: u64) {
    trigger::explain_now(&app, id);
}

/// The card's AI tab switched between Explain and Translate: run request `id` again with
/// that task (the card stays on its tab).
#[tauri::command]
pub fn set_ai_task(app: AppHandle, id: u64, task: AiTask, to: Option<String>) {
    trigger::set_ai_task(&app, id, task, to);
}

/// The card's language search opened (`true`) or closed: the card takes keyboard focus
/// only while it's open.
#[tauri::command]
pub fn popup_focus(app: AppHandle, focus: bool) {
    popup::set_focus(&app, focus);
}

/// Shows a web source's page in the card's content area. Async: creating a web view from
/// a synchronous command can deadlock on Windows.
#[tauri::command]
pub async fn web_show(
    app: AppHandle,
    source: String,
    bounds: popup::WebBounds,
    background: Option<String>,
) -> CmdResult<String> {
    popup::web_show(&app, &source, bounds, background.as_deref())
}

/// GGUF models already on this computer (LM Studio, Ollama, Jan, GPT4All, Hugging Face,
/// Downloads), best fit first.
#[tauri::command]
pub async fn find_models(
    state: State<'_, AppState>,
) -> CmdResult<Vec<te_core::discover::FoundModel>> {
    use te_core::discover;
    let total = te_win::memory().map_or(0, |m| m.total_bytes);
    let own = state.paths.models.clone();
    tauri::async_runtime::spawn_blocking(move || {
        discover::discover(
            &discover::default_roots(),
            total,
            Some(&own),
            discover::MIN_MODEL_BYTES,
        )
    })
    .await
    .map_err(|e| e.to_string())
}

/// Checks a web source address the person typed in Settings.
#[tauri::command]
pub fn validate_source(url: String) -> CmdResult<()> {
    te_core::sources::validate(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn web_hide(app: AppHandle) {
    popup::web_hide(&app);
}

/// Opens a web source's page for the current selection in the person's browser.
#[tauri::command]
pub fn web_open_in_browser(app: AppHandle, source: String) -> CmdResult<()> {
    let url = popup::web_url(&app, &source)?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_text(text: String) -> CmdResult<()> {
    te_win::set_clipboard_text(&text).map_err(|e| e.to_string())
}

/// Explains text typed or pasted in the main window ("Try it").
#[tauri::command]
pub fn explain_text(app: AppHandle, text: String) {
    trigger::start(&app, text, None, None);
}

#[tauri::command]
pub fn open_main(app: AppHandle) {
    popup::close(&app);
    crate::show_main(&app);
}

#[tauri::command]
pub fn open_logs(app: AppHandle) -> CmdResult<()> {
    let logs = &app.state::<AppState>().paths.logs;
    let _ = std::fs::create_dir_all(logs);
    app.opener()
        .open_path(logs.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    if !url.starts_with("https://") {
        return Err("Only https links can be opened.".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_hotkey(hotkey: String) -> CmdResult<()> {
    hotkey::parse(&hotkey).map(|_| ())
}

#[tauri::command]
pub fn set_paused(app: AppHandle, paused: bool) {
    app.state::<AppState>()
        .paused
        .store(paused, Ordering::SeqCst);
    crate::tray::refresh(&app);
}

/// Every tier, so the UI can label them consistently.
#[tauri::command]
pub fn tiers() -> Vec<Tier> {
    vec![Tier::Fastest, Tier::Fast, Tier::Balanced, Tier::Best]
}

/// Looks up text typed in the Look up box: the box hides and the card opens where it was.
/// Works while paused too: it's an explicit request, not a shortcut.
#[tauri::command]
pub fn look_up(app: AppHandle, text: String) -> CmdResult<()> {
    // Whitespace collapsed, at most `MAX_QUERY_CHARS` characters.
    let query = te_core::sources::query_text(&text);
    if query.is_empty() {
        return Err("Type a word or phrase to look up.".into());
    }
    let place = lookup::hide(&app);
    trigger::start(&app, query, None, place);
    Ok(())
}

#[tauri::command]
pub fn lookup_hide(app: AppHandle) {
    lookup::hide(&app);
}

/// The app that was in front at the last shortcut (`keepass.exe`), if known.
#[tauri::command]
pub fn last_app(state: State<'_, AppState>) -> Option<String> {
    state.last_app.lock().expect("last app lock").clone()
}

/// Checks an app name typed in Settings; returns it as the list keeps it.
#[tauri::command]
pub fn validate_app(name: String) -> CmdResult<String> {
    te_core::rules::normalize(&name).map_err(|e| e.to_string())
}

/// "Check now" on the About page: asks and installs when there is a new version.
#[tauri::command]
pub async fn check_updates(app: AppHandle) -> CmdResult<crate::updater::Status> {
    crate::updater::check(&app).await
}

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
use te_core::settings::{ModelChoice, Settings};

use crate::popup::{self, PopupEvent};
use crate::state::AppState;
use crate::{hotkey, trigger};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Saves settings and applies what changed (hotkey, start with Windows, model).
#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> CmdResult<Settings> {
    let settings = settings.sanitized();
    let state = app.state::<AppState>();
    let previous = state.settings();
    if settings.hotkey != previous.hotkey {
        hotkey::apply(&app, &settings.hotkey)
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
pub struct AppInfo {
    version: &'static str,
    models: Vec<ModelInfo>,
    recommended: &'static str,
    total_ram_bytes: Option<u64>,
    languages: Vec<LanguageInfo>,
    engine_found: bool,
    model_loaded: bool,
    downloading: Option<String>,
    paused: bool,
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
pub async fn app_info(state: State<'_, AppState>) -> CmdResult<AppInfo> {
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
        engine_found: state.paths.llama_server.is_file(),
        model_loaded,
        downloading: state
            .download
            .lock()
            .expect("download lock")
            .as_ref()
            .map(|(id, _)| id.clone()),
        paused: state.paused.load(Ordering::SeqCst),
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
pub fn popup_resize(app: AppHandle, height: f64) {
    popup::resize(&app, height);
}

#[tauri::command]
pub fn popup_close(app: AppHandle) {
    popup::close(&app);
}

#[tauri::command]
pub fn popup_pin(app: AppHandle, pinned: bool) {
    popup::set_pinned(&app, pinned);
}

/// Changes the reading level from the card and explains the same text again.
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
    trigger::run(&app);
    Ok(())
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

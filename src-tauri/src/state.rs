//! Everything the app keeps while it runs.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::Shortcut;
use te_core::dictionary::Dictionary;
use te_core::download::Cancel;
use te_core::engine::{Backend, Engine, ServerConfig};
use te_core::history::History;
use te_core::models;
use te_core::settings::{ModelChoice, Settings};
use te_win::{ClickOutsideWatcher, Rect, Side};

use crate::popup::AiTask;

pub struct Paths {
    pub settings: PathBuf,
    pub models: PathBuf,
    pub logs: PathBuf,
    pub llama_server: PathBuf,
    pub dictionary: PathBuf,
    /// The Word book and lookup history (`history.rs`).
    pub history: PathBuf,
}

impl Paths {
    pub fn resolve(app: &AppHandle) -> tauri::Result<Self> {
        let config = app.path().app_config_dir()?;
        let data = app.path().app_local_data_dir()?;
        let resources = app.path().resource_dir()?;
        Ok(Paths {
            settings: config.join("settings.json"),
            models: data.join("models"),
            logs: data.join("logs"),
            llama_server: resources
                .join("resources")
                .join("llama")
                .join("llama-server.exe"),
            dictionary: resources.join("resources").join("dictionary.sqlite"),
            history: data.join("history.sqlite"),
        })
    }

    pub fn model_file(&self, m: &models::CatalogModel) -> PathBuf {
        self.models.join(m.file_name)
    }
}

/// The selection being explained, kept so changing the reading level can redo it.
#[derive(Clone)]
pub struct LastRequest {
    pub text: String,
    pub context: Option<String>,
}

/// Where the popup is and which way it grows.
#[derive(Clone, Copy)]
pub struct PopupPlacement {
    pub anchor: Rect,
    pub work: Rect,
    pub side: Side,
    pub scale: f64,
    pub rect: Rect,
}

pub struct AppState {
    pub paths: Paths,
    pub settings: Mutex<Settings>,
    pub engine: Arc<tokio::sync::Mutex<Engine>>,
    pub task: Mutex<Option<JoinHandle<()>>>,
    pub request_id: AtomicU64,
    pub last: Mutex<Option<LastRequest>>,
    /// The AI tab's task and the request it was chosen on; any other request explains.
    pub ai_task: Mutex<(u64, AiTask)>,
    pub popup: Mutex<Option<PopupPlacement>>,
    pub pinned: AtomicBool,
    pub paused: AtomicBool,
    pub hotkey: Mutex<Option<Shortcut>>,
    pub click_watcher: ClickOutsideWatcher,
    pub download: Mutex<Option<(String, Cancel)>>,
    /// WordNet, for single words (None if the file is missing).
    pub dictionary: Mutex<Option<Dictionary>>,
    /// The address last loaded in the card's web view, so reopening a tab doesn't reload.
    pub web_url: Mutex<Option<String>>,
    /// The Look up box's own shortcut, when one is set and registered.
    pub lookup_hotkey: Mutex<Option<Shortcut>>,
    /// The program file of the app in front at the last hotkey or double Ctrl+C
    /// (`keepass.exe`), so Settings can offer to add it to the excluded apps.
    pub last_app: Mutex<Option<String>>,
    /// Saved words and lookup history (in memory if the file can't be used).
    pub history: Mutex<History>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.lock().expect("settings lock").clone()
    }

    /// The AI tab's task for request `id` (Explain unless the card chose otherwise on it).
    pub fn ai_task(&self, id: u64) -> AiTask {
        match *self.ai_task.lock().expect("ai task lock") {
            (chosen_on, task) if chosen_on == id => task,
            _ => AiTask::Explain,
        }
    }

    pub fn backend(&self, settings: &Settings) -> Backend {
        let local = |model: PathBuf| {
            Backend::Local(ServerConfig {
                exe: self.paths.llama_server.clone(),
                model,
                ctx: settings.context_size,
                threads: settings.threads,
                log: Some(self.paths.logs.join("llama-server.log")),
                extra_args: settings.extra_server_args.clone(),
            })
        };
        match &settings.model {
            ModelChoice::None => Backend::None,
            ModelChoice::Catalog { id } => {
                models::by_id(id).map_or(Backend::None, |m| local(self.paths.model_file(m)))
            }
            ModelChoice::File { path } => local(path.clone()),
            ModelChoice::Endpoint {
                base_url,
                model,
                api_key,
            } => Backend::Endpoint {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: model.clone(),
            },
        }
    }
}

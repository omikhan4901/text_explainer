//! Everything the app keeps while it runs.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::Shortcut;
use te_core::download::Cancel;
use te_core::engine::{Backend, Engine, ServerConfig};
use te_core::models;
use te_core::settings::{ModelChoice, Settings};
use te_win::{ClickOutsideWatcher, Rect, Side};

pub struct Paths {
    pub settings: PathBuf,
    pub models: PathBuf,
    pub logs: PathBuf,
    pub llama_server: PathBuf,
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
    pub popup: Mutex<Option<PopupPlacement>>,
    pub pinned: AtomicBool,
    pub paused: AtomicBool,
    pub hotkey: Mutex<Option<Shortcut>>,
    pub click_watcher: ClickOutsideWatcher,
    pub download: Mutex<Option<(String, Cancel)>>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.lock().expect("settings lock").clone()
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

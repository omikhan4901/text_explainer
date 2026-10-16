//! Everything the app keeps while it runs.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
    /// Where models are downloaded: `models` next to the app, so they go on the drive the
    /// person installed to (the owner's call, 16 October 2026: a nearly full C: drive).
    pub models: PathBuf,
    /// Where models were downloaded before; still read, and moved from when that's instant.
    pub old_models: PathBuf,
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
            models: models_dir(&resources, &data),
            old_models: data.join("models"),
            logs: data.join("logs"),
            llama_server: resources
                .join("resources")
                .join("llama")
                .join("llama-server.exe"),
            dictionary: resources.join("resources").join("dictionary.sqlite"),
            history: data.join("history.sqlite"),
        })
    }

    /// A catalog model's file: where it is, or where a new download goes.
    pub fn model_file(&self, m: &models::CatalogModel) -> PathBuf {
        models::locate(&[&self.models, &self.old_models], m.file_name)
    }

    /// Every place a catalog model's file may be (deleting removes all of them).
    pub fn model_files(&self, m: &models::CatalogModel) -> [PathBuf; 2] {
        [
            self.models.join(m.file_name),
            self.old_models.join(m.file_name),
        ]
    }
}

/// `models` in the app's install folder when the app can write there, else in its data
/// folder. Development builds keep using the data folder (the "install folder" is target/).
fn models_dir(install: &std::path::Path, data: &std::path::Path) -> PathBuf {
    let next_to_app = install.join("models");
    if cfg!(debug_assertions) || !writable(&next_to_app) {
        return data.join("models");
    }
    next_to_app
}

fn writable(dir: &std::path::Path) -> bool {
    let probe = dir.join(".write-test");
    let ok = std::fs::create_dir_all(dir).is_ok() && std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(probe);
    ok
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
    /// The person moved or resized the card: it stays where they put it.
    pub placed: bool,
}

pub struct AppState {
    pub paths: Paths,
    /// When the app started (a busy shortcut is tried quietly for a while first).
    pub started: Instant,
    pub settings: Mutex<Settings>,
    pub engine: Arc<tokio::sync::Mutex<Engine>>,
    pub task: Mutex<Option<JoinHandle<()>>>,
    pub request_id: AtomicU64,
    pub last: Mutex<Option<LastRequest>>,
    /// The AI tab's task, the language picked to translate into (if any), and the request
    /// they were chosen on; any other request explains.
    pub ai_task: Mutex<(u64, AiTask, Option<String>)>,
    pub popup: Mutex<Option<PopupPlacement>>,
    /// The card's last size as the page reported it (CSS pixels, with the shadow margin):
    /// a new card opens at it, since the page reports only changes.
    pub card_size: Mutex<(f64, f64)>,
    /// The person clicked in the card, so it can take focus (`popup::focus_card`).
    pub focused: AtomicBool,
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
    pub fn ai_task(&self, id: u64) -> (AiTask, Option<String>) {
        match &*self.ai_task.lock().expect("ai task lock") {
            (chosen_on, task, to) if *chosen_on == id => (*task, to.clone()),
            _ => (AiTask::Explain, None),
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

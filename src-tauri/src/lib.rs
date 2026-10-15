//! The Text Explainer desktop app: tray, global hotkey and double Ctrl+C, the reading
//! card, settings and model management.

mod commands;
mod hotkey;
mod lookup;
mod popup;
mod state;
mod tray;
mod trigger;
mod updater;
mod words;

use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use te_core::engine::{Backend, Engine};
use te_core::settings::Settings;
use te_win::{ClickOutsideWatcher, DoubleCopyWatcher};

use crate::state::{AppState, Paths};

/// Shows and focuses the main window (settings, models, first run).
pub(crate) fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn init_logging(paths: &Paths) {
    let _ = std::fs::create_dir_all(&paths.logs);
    if let Ok(file) = std::fs::File::create(paths.logs.join("app.log")) {
        let _ = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .try_init();
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let paths = Paths::resolve(&handle)?;
    init_logging(&paths);
    let settings = Settings::load(&paths.settings);

    let mut engine = Engine::new(Backend::None);
    engine.set_spawn_hook(|pid| {
        if let Err(e) = te_win::kill_with_app(pid) {
            tracing::warn!("couldn't tie the model process to the app: {e}");
        }
    });

    let click_app = handle.clone();
    let click_watcher = ClickOutsideWatcher::start(move || {
        let app = click_app.clone();
        let _ = click_app.run_on_main_thread(move || popup::close(&app));
    });

    let dictionary = te_core::dictionary::Dictionary::open(&paths.dictionary)
        .map_err(|e| tracing::warn!("dictionary unavailable: {e}"))
        .ok();
    let history = te_core::history::History::open(&paths.history);
    app.manage(AppState {
        paths,
        settings: Mutex::new(settings.clone()),
        engine: Arc::new(tokio::sync::Mutex::new(engine)),
        task: Mutex::new(None),
        request_id: AtomicU64::new(0),
        last: Mutex::new(None),
        ai_task: Mutex::new((0, popup::AiTask::Explain, None)),
        popup: Mutex::new(None),
        pinned: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        hotkey: Mutex::new(None),
        click_watcher,
        download: Mutex::new(None),
        dictionary: Mutex::new(dictionary),
        web_url: Mutex::new(None),
        lookup_hotkey: Mutex::new(None),
        last_app: Mutex::new(None),
        history: Mutex::new(history),
    });

    popup::prepare(&handle);
    if let Err(e) = hotkey::apply(&handle, &settings.hotkey) {
        tracing::warn!("couldn't register {}: {e}", settings.hotkey);
    }
    if let Err(e) = hotkey::apply_lookup(&handle, &settings.lookup_hotkey) {
        tracing::warn!("couldn't register {}: {e}", settings.lookup_hotkey);
    }

    let copy_app = handle.clone();
    if let Err(e) = DoubleCopyWatcher::start(move |text| {
        let app = copy_app.clone();
        let _ = copy_app.run_on_main_thread(move || trigger::on_double_copy(&app, text));
    }) {
        tracing::warn!("double Ctrl+C won't work: {e}");
    }

    let autostart = handle.autolaunch();
    let enabled = autostart.is_enabled().unwrap_or(false);
    if settings.start_with_windows && !enabled {
        let _ = autostart.enable();
    } else if !settings.start_with_windows && enabled {
        let _ = autostart.disable();
    }

    tray::create(&handle)?;
    spawn_idle_unloader(handle.clone());

    let launched_at_login = std::env::args().any(|a| a == "--autostart");
    if !settings.first_run_done || !launched_at_login {
        show_main(&handle);
    }

    // The Windows CI smoke test waits for this line.
    let state = handle.state::<AppState>();
    tracing::info!(
        version = %handle.package_info().version,
        engine_found = state.paths.llama_server.is_file(),
        dictionary = state.dictionary.lock().is_ok_and(|d| d.is_some()),
        hotkey_registered = state.hotkey.lock().is_ok_and(|h| h.is_some()),
        "Text Explainer started"
    );
    updater::init(&handle);
    Ok(())
}

/// Frees the model's memory after the configured idle time.
fn spawn_idle_unloader(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let state = app.state::<AppState>();
            let minutes = state.settings().idle_unload_minutes;
            if minutes == 0 {
                continue;
            }
            // Busy (a request is streaming): it isn't idle.
            let Ok(mut engine) = state.engine.try_lock() else {
                continue;
            };
            if engine.is_loaded()
                && engine.idle_for() > Duration::from_secs(u64::from(minutes) * 60)
            {
                tracing::info!("unloading the model after {minutes} idle minutes");
                engine.unload().await;
            }
        }
    });
}

pub fn run() {
    te_core::tls_init();
    tauri::Builder::default()
        // A second launch just opens the running app's window.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app)
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(hotkey::handle)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::app_info,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
            commands::unload_model,
            commands::popup_resize,
            commands::popup_close,
            commands::popup_pin,
            commands::set_level,
            commands::explain_now,
            commands::set_ai_task,
            commands::popup_focus,
            commands::web_show,
            commands::web_hide,
            commands::validate_source,
            commands::find_models,
            commands::web_open_in_browser,
            commands::copy_text,
            commands::explain_text,
            commands::open_main,
            commands::open_logs,
            commands::open_url,
            commands::validate_hotkey,
            commands::set_paused,
            commands::tiers,
            commands::look_up,
            commands::lookup_hide,
            commands::last_app,
            commands::validate_app,
            words::save_word,
            words::remove_word,
            words::is_saved,
            words::list_words,
            words::list_history,
            words::clear_history,
            words::export_words,
            words::speak,
            words::word_book_on_disk,
            commands::check_updates,
        ])
        .on_window_event(|window, event| match event {
            // Closing the main window keeps the app running in the tray.
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            // Clicking elsewhere hides the Look up box.
            WindowEvent::Focused(false) if window.label() == lookup::LABEL => {
                lookup::hide(window.app_handle());
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Text Explainer");
}

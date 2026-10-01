//! The Text Explainer desktop app: tray, global hotkey and double Ctrl+C, the reading
//! card, settings and model management.

mod commands;
mod hotkey;
mod popup;
mod state;
mod tray;
mod trigger;

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

    app.manage(AppState {
        paths,
        settings: Mutex::new(settings.clone()),
        engine: Arc::new(tokio::sync::Mutex::new(engine)),
        task: Mutex::new(None),
        request_id: AtomicU64::new(0),
        last: Mutex::new(None),
        popup: Mutex::new(None),
        pinned: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        hotkey: Mutex::new(None),
        click_watcher,
        download: Mutex::new(None),
    });

    popup::prepare(&handle);
    if let Err(e) = hotkey::apply(&handle, &settings.hotkey) {
        tracing::warn!("couldn't register {}: {e}", settings.hotkey);
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
            commands::copy_text,
            commands::explain_text,
            commands::open_main,
            commands::open_logs,
            commands::open_url,
            commands::validate_hotkey,
            commands::set_paused,
            commands::tiers,
        ])
        .on_window_event(|window, event| {
            // Closing the main window keeps the app running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Text Explainer");
}

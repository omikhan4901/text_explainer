//! The Plainly desktop app: tray, global hotkey and double Ctrl+C, the reading
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

use tauri::{AppHandle, Emitter, Manager, WindowEvent};
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
    let focus_app = handle.clone();
    let click_watcher = ClickOutsideWatcher::start(
        move || {
            let app = click_app.clone();
            let _ = click_app.run_on_main_thread(move || popup::clicked_elsewhere(&app));
        },
        move || popup::focus_card(&focus_app),
    );

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
        card_size: Mutex::new(popup::INITIAL_SIZE),
        focused: AtomicBool::new(false),
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

    // The windows are created only now (`"create": false` in tauri.conf.json), once the
    // state exists: a page that calls a command while starting up must never find the app
    // half set up (that panicked on a slow WebView2 start: "state() called before manage()").
    for window in handle.config().app.windows.clone() {
        tauri::WebviewWindowBuilder::from_config(&handle, &window)?.build()?;
    }

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
    spawn_shortcut_retry(handle.clone());

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
        "Plainly started"
    );
    updater::init(&handle);
    Ok(())
}

/// Keeps trying a shortcut that couldn't be registered. Right after an update or a restart,
/// the copy of the app that is closing can still hold it for a moment, and a single try at
/// startup then left the shortcut dead until the next restart.
fn spawn_shortcut_retry(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let state = app.state::<AppState>();
            let settings = state.settings();
            let mut changed = false;
            if state.hotkey.lock().expect("hotkey lock").is_none()
                && hotkey::apply(&app, &settings.hotkey).is_ok()
            {
                tracing::info!("registered {} on a later try", settings.hotkey);
                changed = true;
            }
            if !settings.lookup_hotkey.trim().is_empty()
                && state
                    .lookup_hotkey
                    .lock()
                    .expect("lookup hotkey lock")
                    .is_none()
                && hotkey::apply_lookup(&app, &settings.lookup_hotkey).is_ok()
            {
                tracing::info!("registered {} on a later try", settings.lookup_hotkey);
                changed = true;
            }
            if changed {
                // The main window's "another app uses this shortcut" notice goes away.
                let _ = app.emit_to("main", "te://settings", ());
            }
        }
    });
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

/// Writes a panic's message and backtrace straight to `logs/panic.log` before the process
/// aborts (release builds abort on panic, and that can happen before logging starts).
fn install_panic_log() {
    let Some(base) = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("HOME")) else {
        return;
    };
    let dir = std::path::PathBuf::from(base)
        .join("dev.omikhan.textexplainer")
        .join("logs");
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::create_dir_all(&dir);
        let backtrace = std::backtrace::Backtrace::force_capture();
        let _ = std::fs::write(dir.join("panic.log"), format!("{info}\n\n{backtrace}\n"));
        default(info);
    }));
}

pub fn run() {
    install_panic_log();
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
            commands::popup_drag,
            commands::set_card_size,
            commands::popup_close,
            commands::set_level,
            commands::explain_now,
            commands::set_ai_task,
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
        .expect("error while running Plainly");
}

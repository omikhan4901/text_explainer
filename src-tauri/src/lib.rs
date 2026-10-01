//! The Text Explainer desktop app: tray, global hotkey, popup window and IPC.

mod tray;

use tauri::{Manager, WindowEvent};

/// Shows and focuses the main window (settings, models, first run).
pub(crate) fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        // A second launch just opens the running app's window.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app)
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
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

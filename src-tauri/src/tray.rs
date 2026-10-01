//! The tray icon is the app's only permanent presence: no taskbar button.

use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let paused = app.state::<AppState>().paused.load(Ordering::SeqCst);
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        if paused { "Resume" } else { "Pause" },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Text Explainer", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[
            &settings,
            &pause,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

fn tooltip(app: &AppHandle) -> String {
    let state = app.state::<AppState>();
    if state.paused.load(Ordering::SeqCst) {
        "Text Explainer (paused)".into()
    } else {
        format!(
            "Text Explainer: select text, press {}",
            state.settings().hotkey
        )
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip(tooltip(app))
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => crate::show_main(app),
            "pause" => {
                let state = app.state::<AppState>();
                let paused = !state.paused.load(Ordering::SeqCst);
                state.paused.store(paused, Ordering::SeqCst);
                refresh(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Updates the menu and tooltip after pausing or changing the shortcut.
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_tooltip(Some(tooltip(app)));
    }
}

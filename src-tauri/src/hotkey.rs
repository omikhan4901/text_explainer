//! The global shortcut. Escape is registered only while the card is open.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::popup;
use crate::state::AppState;
use crate::trigger;

pub fn parse(hotkey: &str) -> Result<Shortcut, String> {
    let shortcut = Shortcut::from_str(hotkey).map_err(|e| e.to_string())?;
    // A shortcut without a modifier would fire while typing.
    if shortcut.mods.is_empty() {
        return Err("Use at least one of Ctrl, Alt or Shift.".into());
    }
    Ok(shortcut)
}

/// Replaces the registered shortcut with `hotkey`.
pub fn apply(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut = parse(hotkey)?;
    let state = app.state::<AppState>();
    let mut current = state.hotkey.lock().expect("hotkey lock");
    if let Some(old) = current.take() {
        let _ = app.global_shortcut().unregister(old);
    }
    if let Err(e) = app.global_shortcut().register(shortcut) {
        return Err(format!("{e} (another app may already use it)"));
    }
    *current = Some(shortcut);
    Ok(())
}

pub fn handle(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let is_escape = Shortcut::from_str(popup::ESCAPE).is_ok_and(|esc| esc.id() == shortcut.id());
    if is_escape {
        popup::close(app);
        return;
    }
    let ours = app
        .state::<AppState>()
        .hotkey
        .lock()
        .expect("hotkey lock")
        .is_some_and(|h| h.id() == shortcut.id());
    if ours {
        trigger::on_hotkey(app);
    }
}

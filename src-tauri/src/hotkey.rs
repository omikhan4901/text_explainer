//! The global shortcuts: the main one, and the Look up box's own (optional). Escape is
//! registered only while the card is open.

use std::str::FromStr;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::state::AppState;
use crate::{lookup, popup, trigger};

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

/// Replaces the Look up box's shortcut with `hotkey`; an empty string turns it off.
pub fn apply_lookup(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut = match hotkey.trim() {
        "" => None,
        h => Some(parse(h)?),
    };
    let state = app.state::<AppState>();
    let main = *state.hotkey.lock().expect("hotkey lock");
    if let (Some(s), Some(main)) = (shortcut, main)
        && s.id() == main.id()
    {
        return Err("it's already the shortcut that opens the card".into());
    }
    let mut current = state.lookup_hotkey.lock().expect("lookup hotkey lock");
    let old = current.take();
    if let Some(old) = old {
        let _ = app.global_shortcut().unregister(old);
    }
    if let Some(s) = shortcut {
        if let Err(e) = app.global_shortcut().register(s) {
            // Keep the one that worked, so the saved setting stays true.
            if let Some(old) = old
                && app.global_shortcut().register(old).is_ok()
            {
                *current = Some(old);
            }
            return Err(format!("{e} (another app may already use it)"));
        }
        *current = Some(s);
    }
    Ok(())
}

/// Whether two shortcut strings name the same keys ("Shift+Ctrl+L" and "ctrl+shift+l").
pub fn same(a: &str, b: &str) -> bool {
    matches!((parse(a), parse(b)), (Ok(a), Ok(b)) if a.id() == b.id())
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
    let state = app.state::<AppState>();
    let ours = |slot: &std::sync::Mutex<Option<Shortcut>>| {
        slot.lock()
            .expect("hotkey lock")
            .is_some_and(|h| h.id() == shortcut.id())
    };
    if ours(&state.hotkey) {
        trigger::on_hotkey(app);
    } else if ours(&state.lookup_hotkey) && !state.paused.load(Ordering::SeqCst) {
        // Paused turns every shortcut off; the tray's "Look up…" still works.
        lookup::toggle(app);
    }
}

//! Updates: the app reads `latest.json` from the latest build on GitHub, asks before
//! installing, and restarts into the new version. Every update is signed with the owner's
//! key and the app refuses one that isn't (the key's public half is built in), so a file
//! changed on GitHub can't reach anyone. Builds without the key (local ones) never check.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

use crate::state::AppState;

const FIRST_CHECK: Duration = Duration::from_secs(90);
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// One check at a time, so a manual check and the timer never ask twice.
static CHECKING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Debug)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    /// This build has no update key (built locally): it can't update itself.
    Unavailable,
    UpToDate,
    /// A check is already running (it asks on its own).
    Busy,
    /// The person chose "Later".
    Declined {
        version: String,
    },
}

/// Whether this build carries the public key that update signatures are checked against.
pub fn available(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

/// Registers the updater and starts the background checks (when the setting allows).
pub fn init(app: &AppHandle) {
    if !available(app) {
        tracing::info!("updates are off: this build has no update key");
        return;
    }
    if let Err(e) = app.plugin(tauri_plugin_updater::Builder::new().build()) {
        tracing::warn!("couldn't start the updater: {e}");
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<AppState>().settings().check_updates
                && let Err(e) = check(&app).await
            {
                tracing::info!("update check failed: {e}");
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

/// Looks for a newer version; if there is one, asks, then downloads, verifies the
/// signature, installs and restarts. Returns only when nothing was installed.
pub async fn check(app: &AppHandle) -> Result<Status, String> {
    if !available(app) {
        return Ok(Status::Unavailable);
    }
    if CHECKING.swap(true, Ordering::SeqCst) {
        return Ok(Status::Busy);
    }
    let result = check_once(app).await;
    CHECKING.store(false, Ordering::SeqCst);
    result
}

async fn check_once(app: &AppHandle) -> Result<Status, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(Status::UpToDate);
    };
    let version = update.version.clone();
    let asker = app.clone();
    let question = version.clone();
    // The native dialog waits for an answer; never on the async runtime's threads.
    let install = tauri::async_runtime::spawn_blocking(move || {
        asker
            .dialog()
            .message(format!(
                "Plainly {question} is ready. Install it now? The app closes, updates \
                 and opens again."
            ))
            .title("Update available")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Install and restart".into(),
                "Later".into(),
            ))
            .blocking_show()
    })
    .await
    .unwrap_or(false);
    if !install {
        return Ok(Status::Declined { version });
    }
    tracing::info!("installing version {version}");
    // Verifies the signature before running anything; on Windows the installer takes
    // over and the app exits.
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}

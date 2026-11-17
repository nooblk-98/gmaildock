use std::{sync::Mutex, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

use crate::settings::SettingsStore;

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(10);
const CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Serialize)]
pub struct AvailableUpdate {
    pub version: String,
    notes: Option<String>,
}

#[derive(Default)]
pub struct UpdateState(Mutex<Option<AvailableUpdate>>);

impl UpdateState {
    fn cached(&self) -> Option<AvailableUpdate> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn store(&self, update: Option<AvailableUpdate>) {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = update;
    }
}

#[derive(Clone, Serialize)]
struct UpdateProgress {
    chunk: usize,
    total: Option<u64>,
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<AvailableUpdate>, String> {
    refresh(&app).await
}

/// Checks for an update, caches the result and announces it to open windows.
pub async fn refresh(app: &AppHandle) -> Result<Option<AvailableUpdate>, String> {
    let update = lookup(app).await?;
    app.state::<UpdateState>().store(update.clone());

    if let Some(update) = &update {
        if let Err(error) = app.emit("update-available", update.clone()) {
            log::warn!("could not announce the available update: {error}");
        }
    }

    Ok(update)
}

#[tauri::command]
pub fn get_update_state(state: State<'_, UpdateState>) -> Option<AvailableUpdate> {
    state.cached()
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("No update is available")?;

    let progress_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                let _ = progress_app.emit("update-progress", UpdateProgress { chunk, total });
            },
            || {},
        )
        .await
        .map_err(|error| error.to_string())?;

    log::info!("update installed; restarting GmailDock");
    app.restart();
}

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

const DEVELOPER_URL: &str = "https://github.com/nooblk-98";
const REPOSITORY_URL: &str = "https://github.com/nooblk-98/gmaildock";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AboutLink {
    Developer,
    Repository,
}

#[tauri::command]
pub fn open_about_link(link: AboutLink) -> Result<(), String> {
    let url = match link {
        AboutLink::Developer => DEVELOPER_URL,
        AboutLink::Repository => REPOSITORY_URL,
    };
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

pub fn spawn_background_checks(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK_DELAY);
        loop {
            if app.state::<SettingsStore>().snapshot().auto_check_updates {
                check_in_background(&app);
            }
            std::thread::sleep(CHECK_INTERVAL);
        }
    });
}

fn check_in_background(app: &AppHandle) {
    let update = match tauri::async_runtime::block_on(lookup(app)) {
        Ok(update) => update,
        Err(error) => {
            log::warn!("could not check for updates: {error}");
            return;
        }
    };

    let previous = app.state::<UpdateState>().cached();
    app.state::<UpdateState>().store(update.clone());

    let settings = app.state::<SettingsStore>().snapshot();
    let Some(update) = update else {
        return;
    };
    if settings.skipped_version.as_deref() == Some(update.version.as_str()) {
        return;
    }

    if let Err(error) = app.emit("update-available", update.clone()) {
        log::warn!("could not announce the available update: {error}");
    }

    let already_announced = previous
        .map(|known| known.version == update.version)
        .unwrap_or(false);
    if !already_announced && settings.notifications {
        notify(app, &update);
    }
}

/// Tray "Check for updates": opens Settings when an update exists, otherwise reports the result.
pub fn check_from_tray(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        match refresh(&app).await {
            Ok(Some(update)) => {
                log::info!("update {} is available", update.version);
                if let Err(error) = crate::settings_window::open(&app) {
                    log::warn!("could not open the settings window: {error}");
                }
            }
            Ok(None) => show(
                &app,
                "GmailDock is up to date",
                "You have the latest version.",
            ),
            Err(error) => {
                log::warn!("could not check for updates: {error}");
                show(&app, "Update check failed", "See the log file for details.");
            }
        }
    });
}

fn show(app: &AppHandle, title: &str, body: &str) {
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        log::warn!("could not show a notification: {error}");
    }
}

fn notify(app: &AppHandle, update: &AvailableUpdate) {
    let shown = app
        .notification()
        .builder()
        .title("GmailDock update available")
        .body(format!("Version {} is ready to install.", update.version))
        .show();

    if let Err(error) = shown {
        log::warn!("could not show the update notification: {error}");
    }
}

async fn lookup(app: &AppHandle) -> Result<Option<AvailableUpdate>, String> {
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?;

    Ok(update.map(|available| AvailableUpdate {
        version: available.version,
        notes: available.body,
    }))
}

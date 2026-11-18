//! Background update checks and tray-driven installs.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::{
    badge,
    settings::{self, SettingsStore},
    tray,
};

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(10);
const CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Default)]
pub struct UpdateState {
    available: Mutex<Option<String>>,
    installing: AtomicBool,
}

impl UpdateState {
    fn available(&self) -> Option<String> {
        self.available
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Stores the latest known version and returns the previous one.
    fn store(&self, version: Option<String>) -> Option<String> {
        std::mem::replace(
            &mut *self
                .available
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            version,
        )
    }
}

/// What the tray menu needs to know about updates.
pub struct UpdateStatus {
    pub available: Option<String>,
    pub installing: bool,
}

pub fn status(app: &AppHandle) -> UpdateStatus {
    let state = app.state::<UpdateState>();
    UpdateStatus {
        available: state.available(),
        installing: state.installing.load(Ordering::Relaxed),
    }
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
    let version = match tauri::async_runtime::block_on(lookup(app)) {
        Ok(version) => version,
        Err(error) => {
            log::warn!("could not check for updates: {error}");
            return;
        }
    };

    let previous = app.state::<UpdateState>().store(version.clone());
    if previous != version {
        tray::refresh(app);
    }

    let Some(version) = version else {
        return;
    };
    let settings = app.state::<SettingsStore>().snapshot();
    if settings.skipped_version.as_deref() == Some(version.as_str()) {
        return;
    }
    if previous.as_deref() != Some(version.as_str()) && settings.notifications {
        announce(app, &version);
    }
}

fn announce(app: &AppHandle, version: &str) {
    tray::notify(
        app,
        "GmailDock update available",
        &format!(
            "Version {version} is ready. Right-click the GmailDock tray icon and choose Install update."
        ),
    );
}

/// Tray "Check for updates". Asking explicitly also brings back a version that was skipped.
pub fn check_from_tray(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        match lookup(&app).await {
            Ok(Some(version)) => {
                log::info!("update {version} is available");
                app.state::<UpdateState>().store(Some(version.clone()));
                let handle = app.clone();
                let available = version.clone();
                if let Err(error) = app.run_on_main_thread(move || {
                    let mut current = handle.state::<SettingsStore>().snapshot();
                    if current.skipped_version.as_deref() == Some(available.as_str()) {
                        current.skipped_version = None;
                        if let Err(error) = settings::apply(&handle, current) {
                            log::warn!("could not clear the skipped version: {error}");
                        }
                    }
                    tray::refresh(&handle);
                }) {
                    log::warn!("could not refresh the skipped update: {error}");
                }
                announce(&app, &version);
            }
            Ok(None) => {
                app.state::<UpdateState>().store(None);
                tray::refresh(&app);
                tray::notify(
                    &app,
                    "GmailDock is up to date",
                    &format!(
                        "You have the latest version, {}.",
                        app.package_info().version
                    ),
                );
            }
            Err(error) => {
                log::warn!("could not check for updates: {error}");
                tray::notify(&app, "Update check failed", "See the log file for details.");
            }
        }
    });
}

/// Tray "Install update": downloads with progress in the tray tooltip, then restarts.
pub fn install_from_tray(app: AppHandle) {
    if app
        .state::<UpdateState>()
        .installing
        .swap(true, Ordering::Relaxed)
    {
        return;
    }
    tray::refresh(&app);

    tauri::async_runtime::spawn(async move {
        if let Err(error) = install(&app).await {
            log::warn!("could not install the update: {error}");
            app.state::<UpdateState>()
                .installing
                .store(false, Ordering::Relaxed);
            tray::refresh(&app);
            badge::restore_tooltip(&app);
            tray::notify(&app, "Update failed", &error);
        }
    });
}

async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("No update is available.")?;

    tray::notify(
        app,
        "Updating GmailDock",
        &format!(
            "Downloading version {}. GmailDock restarts when it is done.",
            update.version
        ),
    );

    let progress_app = app.clone();
    let mut downloaded: u64 = 0;
    let mut shown_percent = None;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let Some(total) = total.filter(|total| *total > 0) else {
                    return;
                };
                let percent = (downloaded * 100 / total).min(100);
                // Only touch the tooltip when the visible number changes.
                if shown_percent != Some(percent) {
                    shown_percent = Some(percent);
                    if let Some(tray) = progress_app.tray_by_id(tray::TRAY_ID) {
                        let _ = tray.set_tooltip(Some(format!(
                            "GmailDock — downloading update {percent}%"
                        )));
                    }
                }
            },
            || {},
        )
        .await
        .map_err(|error| error.to_string())?;

    log::info!("update installed; restarting GmailDock");
    app.restart();
}

async fn lookup(app: &AppHandle) -> Result<Option<String>, String> {
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?;

    Ok(update.map(|available| available.version))
}

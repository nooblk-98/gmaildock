use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::shortcut;

pub const SETTINGS_FILE: &str = "settings.json";
pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+G";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_check_updates: bool,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub open_external_in_browser: bool,
    pub shortcut: Option<String>,
    pub skipped_version: Option<String>,
    pub start_at_login: bool,
    pub start_minimized: bool,
    pub zoom: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_check_updates: true,
            close_to_tray: true,
            notifications: true,
            open_external_in_browser: true,
            shortcut: Some(DEFAULT_SHORTCUT.to_owned()),
            skipped_version: None,
            start_at_login: true,
            start_minimized: false,
            zoom: 1.0,
        }
    }
}

pub fn clamp_zoom(zoom: f64) -> f64 {
    if zoom.is_finite() {
        zoom.clamp(0.5, 2.0)
    } else {
        1.0
    }
}

pub fn settings_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(SETTINGS_FILE))
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<AppSettings>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let settings = read_settings(&path);
        Self {
            path,
            current: Mutex::new(settings),
        }
    }

    pub fn snapshot(&self) -> AppSettings {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn save(&self, settings: AppSettings) -> io::Result<AppSettings> {
        let mut next = settings;
        next.zoom = clamp_zoom(next.zoom);
        let mut current = self
            .current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        write_settings(&self.path, &next)?;
        *current = next.clone();
        Ok(next)
    }
}

fn read_settings(path: &Path) -> AppSettings {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return AppSettings::default(),
        Err(error) => {
            log::warn!("could not read {}: {error}", path.display());
            return AppSettings::default();
        }
    };

    match serde_json::from_slice::<AppSettings>(&contents) {
        Ok(mut settings) => {
            settings.zoom = clamp_zoom(settings.zoom);
            settings
        }
        Err(error) => {
            log::warn!(
                "ignoring unreadable settings in {}: {error}",
                path.display()
            );
            backup_corrupt(path);
            AppSettings::default()
        }
    }
}

fn backup_corrupt(path: &Path) {
    let backup = path.with_extension("json.corrupt");
    if let Err(error) = fs::rename(path, &backup) {
        log::warn!("could not back up {}: {error}", path.display());
    }
}

fn write_settings(path: &Path, settings: &AppSettings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)
}

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> AppSettings {
    store.snapshot()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = store.snapshot();
    let shortcut_changed = previous.shortcut != settings.shortcut;

    if shortcut_changed {
        shortcut::apply(
            &app,
            previous.shortcut.as_deref(),
            settings.shortcut.as_deref(),
        )?;
    }

    let saved = match store.save(settings) {
        Ok(saved) => saved,
        Err(error) => {
            if shortcut_changed {
                let _ = shortcut::register(&app, previous.shortcut.as_deref());
            }
            return Err(format!("could not save settings: {error}"));
        }
    };

    if let Some(window) = app.get_webview_window(crate::window::LABEL) {
        if let Err(error) = window.set_zoom(saved.zoom) {
            log::warn!("could not apply zoom: {error}");
        }
    }

    // Autostart is a side effect of the saved preference; a failure must not undo the save.
    if let Err(error) = apply_autostart(&app, saved.start_at_login) {
        log::warn!("{error}");
    }

    Ok(saved)
}

#[cfg(not(debug_assertions))]
fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    }
    .map_err(|error| format!("could not update autostart: {error}"))
}

#[cfg(debug_assertions)]
fn apply_autostart(_app: &AppHandle, _enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn store(dir: &tempfile::TempDir) -> SettingsStore {
        SettingsStore::load(dir.path().join(SETTINGS_FILE))
    }

    #[test]
    fn missing_settings_file_uses_defaults() {
        let dir = tempfile::tempdir().expect("temporary directory");
        assert_eq!(store(&dir).snapshot(), AppSettings::default());
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let saved = store(&dir)
            .save(AppSettings {
                notifications: false,
                skipped_version: Some("9.9.9".to_owned()),
                zoom: 1.25,
                ..AppSettings::default()
            })
            .expect("settings saved");

        assert_eq!(saved.zoom, 1.25);
        assert!(!saved.notifications);
        assert_eq!(
            SettingsStore::load(dir.path().join(SETTINGS_FILE)).snapshot(),
            saved
        );
    }

    #[test]
    fn zoom_is_clamped_when_saved() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let store = store(&dir);

        assert_eq!(
            store
                .save(AppSettings {
                    zoom: 9.0,
                    ..AppSettings::default()
                })
                .expect("settings saved")
                .zoom,
            2.0
        );
        assert_eq!(
            store
                .save(AppSettings {
                    zoom: 0.1,
                    ..AppSettings::default()
                })
                .expect("settings saved")
                .zoom,
            0.5
        );
        assert_eq!(
            store
                .save(AppSettings {
                    zoom: f64::NAN,
                    ..AppSettings::default()
                })
                .expect("settings saved")
                .zoom,
            1.0
        );
    }

    #[test]
    fn zoom_from_disk_is_clamped() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, br#"{"zoom":9.0}"#).expect("settings written");

        assert_eq!(SettingsStore::load(path).snapshot().zoom, 2.0);
    }

    #[test]
    fn corrupt_settings_are_backed_up() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, b"{ not json").expect("corrupt settings written");

        assert_eq!(
            SettingsStore::load(path.clone()).snapshot(),
            AppSettings::default()
        );
        assert!(!path.exists());
        assert!(path.with_extension("json.corrupt").exists());
    }

    #[test]
    fn settings_saved_without_new_fields_keep_their_defaults() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, br#"{"autoCheckUpdates":false}"#).expect("legacy settings written");

        let settings = SettingsStore::load(path).snapshot();

        assert!(!settings.auto_check_updates);
        assert!(settings.close_to_tray);
        assert!(settings.open_external_in_browser);
        assert!(settings.start_at_login);
        assert!(settings.notifications);
        assert_eq!(settings.shortcut.as_deref(), Some(DEFAULT_SHORTCUT));
        assert_eq!(settings.skipped_version, None);
        assert_eq!(settings.zoom, 1.0);
    }
}

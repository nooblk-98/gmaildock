use std::fs;

use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::{
    settings::{self, AppSettings, SettingsStore},
    settings_window, updater, window,
};

pub const TRAY_ID: &str = "gmaildock";

const PREFERENCE_PREFIX: &str = "pref:";
const ZOOM_PREFIX: &str = "zoom:";

/// On/off settings shown as checkboxes under Preferences, in menu order.
const PREFERENCES: [Preference; 6] = [
    Preference {
        key: "startAtLogin",
        label: "Start at login",
        field: |s| &mut s.start_at_login,
    },
    Preference {
        key: "startMinimized",
        label: "Start minimized to the tray",
        field: |s| &mut s.start_minimized,
    },
    Preference {
        key: "closeToTray",
        label: "Keep running in the tray when closed",
        field: |s| &mut s.close_to_tray,
    },
    Preference {
        key: "notifications",
        label: "New mail notifications",
        field: |s| &mut s.notifications,
    },
    Preference {
        key: "openExternalInBrowser",
        label: "Open non-Google links in my browser",
        field: |s| &mut s.open_external_in_browser,
    },
    Preference {
        key: "autoCheckUpdates",
        label: "Check for updates automatically",
        field: |s| &mut s.auto_check_updates,
    },
];

/// Zoom levels offered in the tray; matches the Settings window.
pub const ZOOM_LEVELS: [f64; 8] = [0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 2.0];

struct Preference {
    key: &'static str,
    label: &'static str,
    field: fn(&mut AppSettings) -> &mut bool,
}

/// Checkbox items kept so their state can follow settings changed elsewhere.
struct TrayItems {
    preferences: Vec<CheckMenuItem<Wry>>,
    zoom: Vec<CheckMenuItem<Wry>>,
}

fn same_zoom(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.001
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let current = app.state::<SettingsStore>().snapshot();

    let preferences = PREFERENCES
        .iter()
        .map(|preference| {
            let mut probe = current.clone();
            let checked = *(preference.field)(&mut probe);
            CheckMenuItemBuilder::with_id(
                format!("{PREFERENCE_PREFIX}{}", preference.key),
                preference.label,
            )
            .checked(checked)
            .build(app)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let zoom = ZOOM_LEVELS
        .iter()
        .enumerate()
        .map(|(index, level)| {
            CheckMenuItemBuilder::with_id(
                format!("{ZOOM_PREFIX}{index}"),
                format!("{}%", (level * 100.0).round()),
            )
            .checked(same_zoom(*level, current.zoom))
            .build(app)
        })
        .collect::<tauri::Result<Vec<_>>>()?;

    let mut preferences_menu = SubmenuBuilder::new(app, "Preferences");
    for item in &preferences {
        preferences_menu = preferences_menu.item(item);
    }
    let mut zoom_menu = SubmenuBuilder::new(app, "Zoom");
    for item in &zoom {
        zoom_menu = zoom_menu.item(item);
    }
    let troubleshooting = SubmenuBuilder::new(app, "Troubleshooting")
        .text("open_logs", "Open logs folder")
        .build()?;

    let menu = MenuBuilder::new(app)
        .text("accounts", "Accounts...")
        .text("browser", "Open in browser")
        .text("reload", "Reload")
        .separator()
        .item(&preferences_menu.build()?)
        .item(&zoom_menu.build()?)
        .separator()
        .text("check_updates", "Check for updates")
        .text("settings", "Settings...")
        .item(&troubleshooting)
        .separator()
        .text("quit", "Quit")
        .build()?;
    app.manage(TrayItems { preferences, zoom });
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| std::io::Error::other("GmailDock icon is missing"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip("GmailDock")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = window::show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .build(app)?;

    Ok(())
}

/// Updates the tray checkboxes to match `settings`.
pub fn sync(app: &AppHandle, settings: &AppSettings) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let mut probe = settings.clone();
    for (preference, item) in PREFERENCES.iter().zip(&items.preferences) {
        let _ = item.set_checked(*(preference.field)(&mut probe));
    }
    for (level, item) in ZOOM_LEVELS.iter().zip(&items.zoom) {
        let _ = item.set_checked(same_zoom(*level, settings.zoom));
    }
}

fn change_setting(app: &AppHandle, change: impl FnOnce(&mut AppSettings)) {
    let mut next = app.state::<SettingsStore>().snapshot();
    change(&mut next);
    if let Err(error) = settings::apply(app, next) {
        log::warn!("could not save the setting from the tray: {error}");
        sync(app, &app.state::<SettingsStore>().snapshot());
    }
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    if let Some(key) = id.strip_prefix(PREFERENCE_PREFIX) {
        if let Some(preference) = PREFERENCES.iter().find(|p| p.key == key) {
            change_setting(app, |settings| {
                let value = (preference.field)(settings);
                *value = !*value;
            });
        }
        return;
    }
    if let Some(index) = id.strip_prefix(ZOOM_PREFIX) {
        if let Some(level) = index.parse::<usize>().ok().and_then(|i| ZOOM_LEVELS.get(i)) {
            change_setting(app, |settings| settings.zoom = *level);
        }
        return;
    }

    match id {
        "check_updates" => updater::check_from_tray(app.clone()),
        "accounts" => {
            if let Some(window) = window::show_main_window(app) {
                if let Ok(url) = tauri::Url::parse(window::ACCOUNTS_URL) {
                    if let Err(error) = window.navigate(url) {
                        log::warn!("could not open the Google account chooser: {error}");
                    }
                }
            }
        }
        "settings" => {
            if let Err(error) = settings_window::open(app) {
                log::warn!("could not open the settings window: {error}");
            }
        }
        "browser" => open_in_browser(),
        "reload" => {
            if let Some(window) = window::show_main_window(app) {
                if let Err(error) = window.reload() {
                    log::warn!("could not reload Gmail: {error}");
                }
            }
        }
        "open_logs" => open_logs(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn open_in_browser() {
    if let Err(error) = tauri_plugin_opener::open_url(window::GMAIL_URL, None::<&str>) {
        log::warn!("could not open Gmail in the browser: {error}");
    }
}

fn open_logs(app: &AppHandle) {
    let result = app
        .path()
        .app_log_dir()
        .map_err(|error| error.to_string())
        .and_then(|dir| {
            fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
            tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|error| error.to_string())
        });

    if let Err(error) = result {
        log::warn!("could not open the logs folder: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{PREFERENCES, ZOOM_LEVELS};
    use crate::settings::AppSettings;

    #[test]
    fn each_preference_toggles_its_own_field() {
        for preference in &PREFERENCES {
            let mut settings = AppSettings::default();
            let before = serde_json::to_value(&settings).expect("serializable");
            let value = (preference.field)(&mut settings);
            *value = !*value;
            let after = serde_json::to_value(&settings).expect("serializable");

            let changed: Vec<_> = after
                .as_object()
                .expect("object")
                .iter()
                .filter(|(key, value)| before[key.as_str()] != **value)
                .map(|(key, _)| key.as_str())
                .collect();
            assert_eq!(changed, [preference.key]);
        }
    }

    #[test]
    fn zoom_levels_match_the_settings_window() {
        let html = include_str!("../../frontend/settings.html");
        for level in ZOOM_LEVELS {
            assert!(
                html.contains(&format!("<option value=\"{level}\">")),
                "{level} missing from settings.html"
            );
        }
    }
}

//! The tray icon and its menu, which is where every GmailDock setting lives.
//!
//! The menu is rebuilt from the current settings and update state whenever either changes,
//! so its checkmarks and update items never go stale.

use std::fs;

use tauri::{
    menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};
use tauri_plugin_notification::NotificationExt;

use crate::{
    settings::{self, AppSettings, SettingsStore},
    updater, window,
};

pub const TRAY_ID: &str = "gmaildock";
const REPOSITORY_URL: &str = "https://github.com/nooblk-98/gmaildock";

const PREFERENCE_PREFIX: &str = "pref:";
const ZOOM_PREFIX: &str = "zoom:";
const SHORTCUT_PREFIX: &str = "shortcut:";

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

const ZOOM_LEVELS: [f64; 8] = [0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 2.0];

/// Global shortcuts offered under Shortcut. The first one is the default.
const SHORTCUTS: [&str; 3] = [
    settings::DEFAULT_SHORTCUT,
    "CommandOrControl+Alt+G",
    "CommandOrControl+Shift+M",
];

struct Preference {
    key: &'static str,
    label: &'static str,
    field: fn(&mut AppSettings) -> &mut bool,
}

fn same_zoom(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.001
}

/// Turns an accelerator such as `CommandOrControl+Shift+G` into what the keyboard shows.
fn shortcut_label(accelerator: &str) -> String {
    let (control, alt) = if cfg!(target_os = "macos") {
        ("Cmd", "Option")
    } else {
        ("Ctrl", "Alt")
    };
    accelerator
        .split('+')
        .map(|part| match part {
            "CommandOrControl" | "CmdOrCtrl" => control,
            "Alt" | "Option" => alt,
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| std::io::Error::other("GmailDock icon is missing"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu(app)?)
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

/// Rebuilds the tray menu so it reflects the current settings and update state.
pub fn refresh(app: &AppHandle) {
    // Queue from a worker: run_on_main_thread may execute inline on the GUI thread.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || refresh_now(&handle)) {
            log::warn!("could not schedule tray refresh: {error}");
        }
    });
}

fn refresh_now(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match menu(app) {
        Ok(menu) => {
            if let Err(error) = tray.set_menu(Some(menu)) {
                log::warn!("could not update the tray menu: {error}");
            }
        }
        Err(error) => log::warn!("could not build the tray menu: {error}"),
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let current = app.state::<SettingsStore>().snapshot();
    let updates = updater::status(app);

    let mut preferences = SubmenuBuilder::new(app, "Preferences");
    for preference in &PREFERENCES {
        let mut probe = current.clone();
        let checked = *(preference.field)(&mut probe);
        preferences = preferences.item(
            &CheckMenuItemBuilder::with_id(
                format!("{PREFERENCE_PREFIX}{}", preference.key),
                preference.label,
            )
            .checked(checked)
            .build(app)?,
        );
    }

    let mut zoom = SubmenuBuilder::new(app, "Zoom");
    for (index, level) in ZOOM_LEVELS.iter().enumerate() {
        zoom = zoom.item(
            &CheckMenuItemBuilder::with_id(
                format!("{ZOOM_PREFIX}{index}"),
                format!("{}%", (level * 100.0).round()),
            )
            .checked(same_zoom(*level, current.zoom))
            .build(app)?,
        );
    }

    let shortcut_error = crate::preferences::shortcut_error();
    let active_shortcut = current
        .shortcut
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut shortcut = SubmenuBuilder::new(app, "Show or hide shortcut");
    for (index, accelerator) in SHORTCUTS.iter().enumerate() {
        shortcut = shortcut.item(
            &CheckMenuItemBuilder::with_id(
                format!("{SHORTCUT_PREFIX}{index}"),
                shortcut_label(accelerator),
            )
            .checked(shortcut_error.is_none() && active_shortcut == Some(*accelerator))
            .build(app)?,
        );
    }
    if let Some(custom) = active_shortcut.filter(|value| !SHORTCUTS.contains(value)) {
        // A shortcut set in an older version stays selected until another one is picked.
        shortcut = shortcut.item(
            &CheckMenuItemBuilder::with_id("shortcut:custom", shortcut_label(custom))
                .checked(shortcut_error.is_none())
                .build(app)?,
        );
    }
    shortcut = shortcut.separator().item(
        &CheckMenuItemBuilder::with_id("shortcut:off", "Off")
            .checked(active_shortcut.is_none())
            .build(app)?,
    );

    if shortcut_error.is_some() {
        shortcut = shortcut.item(
            &MenuItemBuilder::with_id(
                "shortcut_unavailable",
                "Shortcut unavailable — open Preferences",
            )
            .enabled(false)
            .build(app)?,
        );
    }
    let troubleshooting = SubmenuBuilder::new(app, "Troubleshooting")
        .text("open_logs", "Open logs folder")
        .text("diagnostics", "Diagnostics and reset preferences…")
        .text("downloads", "Open Downloads folder")
        .build()?;

    let version = MenuItemBuilder::with_id(
        "version",
        format!("GmailDock {}", app.package_info().version),
    )
    .enabled(false)
    .build(app)?;

    let mut builder = MenuBuilder::new(app)
        .item(&version)
        .separator()
        .text("accounts", "Accounts...")
        .text("browser", "Open in browser")
        .text("reload", "Reload")
        .separator()
        .text("preferences", "Preferences…")
        .item(&preferences.build()?)
        .item(&zoom.build()?)
        .item(&shortcut.build()?)
        .separator();

    if updates.installing {
        builder = builder.item(
            &MenuItemBuilder::with_id("installing", "Installing update...")
                .enabled(false)
                .build(app)?,
        );
    } else if let Some(version) = updates
        .available
        .filter(|version| current.skipped_version.as_deref() != Some(version.as_str()))
    {
        builder = builder
            .text("install_update", format!("Install update ({version})"))
            .text("whats_new", format!("What's new in {version}"))
            .text("skip_update", format!("Skip {version}"));
    }
    builder = builder.text("check_updates", "Check for updates");

    builder
        .separator()
        .text("about", "About GmailDock")
        .item(&troubleshooting)
        .separator()
        .text("quit", "Quit")
        .build()
}

/// Shows a notification, or logs why it could not be shown.
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        log::warn!("could not show a notification: {error}");
    }
}

fn change_setting(app: &AppHandle, change: impl FnOnce(&mut AppSettings)) {
    let mut next = app.state::<SettingsStore>().snapshot();
    change(&mut next);
    if let Err(error) = settings::apply(app, next) {
        log::warn!("could not save the setting from the tray: {error}");
        notify(app, "GmailDock could not change that setting", &error);
        // Put the checkmarks back to what is actually saved.
        refresh(app);
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
    if let Some(choice) = id.strip_prefix(SHORTCUT_PREFIX) {
        match choice {
            "off" => change_setting(app, |settings| settings.shortcut = None),
            // Re-selecting the current custom shortcut changes nothing; just keep it checked.
            "custom" => refresh(app),
            index => {
                if let Some(accelerator) =
                    index.parse::<usize>().ok().and_then(|i| SHORTCUTS.get(i))
                {
                    change_setting(app, |settings| {
                        settings.shortcut = Some((*accelerator).to_owned());
                    });
                }
            }
        }
        return;
    }

    match id {
        "preferences" | "diagnostics" => crate::preferences::open(app),
        "downloads" => crate::downloads::open_folder(app),
        "accounts" => {
            if let Some(window) = window::show_main_window(app) {
                if let Ok(url) = tauri::Url::parse(window::ACCOUNTS_URL) {
                    if let Err(error) = window.navigate(url) {
                        log::warn!("could not open the Google account chooser: {error}");
                    }
                }
            }
        }
        "browser" => open_url(window::GMAIL_URL),
        "reload" => {
            if let Some(window) = window::show_main_window(app) {
                if let Err(error) = window.reload() {
                    log::warn!("could not reload Gmail: {error}");
                }
            }
        }
        "check_updates" => updater::check_from_tray(app.clone()),
        "install_update" => updater::install_from_tray(app.clone()),
        "whats_new" => {
            if let Some(version) = updater::status(app).available {
                open_url(&format!("{REPOSITORY_URL}/releases/tag/v{version}"));
            }
        }
        "skip_update" => {
            if let Some(version) = updater::status(app).available {
                change_setting(app, |settings| settings.skipped_version = Some(version));
            }
        }
        "about" => open_url(REPOSITORY_URL),
        "open_logs" => open_logs(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn open_url(url: &str) {
    if let Err(error) = tauri_plugin_opener::open_url(url, None::<&str>) {
        log::warn!("could not open {url} in the browser: {error}");
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
    use super::{shortcut_label, PREFERENCES, SHORTCUTS, ZOOM_LEVELS};
    use crate::settings::{clamp_zoom, AppSettings, DEFAULT_SHORTCUT};

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
    fn zoom_levels_are_within_the_saved_range() {
        for level in ZOOM_LEVELS {
            assert_eq!(clamp_zoom(level), level);
        }
        assert!(ZOOM_LEVELS.contains(&AppSettings::default().zoom));
    }

    #[test]
    fn default_shortcut_is_offered() {
        assert_eq!(SHORTCUTS[0], DEFAULT_SHORTCUT);
    }

    #[test]
    fn shortcut_labels_read_like_the_keyboard() {
        let expected = if cfg!(target_os = "macos") {
            ["Cmd+Shift+G", "Cmd+Option+G"]
        } else {
            ["Ctrl+Shift+G", "Ctrl+Alt+G"]
        };
        assert_eq!(shortcut_label(SHORTCUTS[0]), expected[0]);
        assert_eq!(shortcut_label(SHORTCUTS[1]), expected[1]);
    }
}

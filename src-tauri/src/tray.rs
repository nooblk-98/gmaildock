use std::fs;

use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::{settings_window, window};

pub const TRAY_ID: &str = "gmaildock";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let troubleshooting = SubmenuBuilder::new(app, "Troubleshooting")
        .text("open_logs", "Open logs folder")
        .build()?;
    let menu = MenuBuilder::new(app)
        .text("accounts", "Accounts...")
        .text("settings", "Settings")
        .text("browser", "Open in browser")
        .text("reload", "Reload")
        .separator()
        .item(&troubleshooting)
        .separator()
        .text("quit", "Quit")
        .build()?;
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

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
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

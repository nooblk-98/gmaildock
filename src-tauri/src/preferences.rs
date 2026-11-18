//! Trusted local preferences UI. Remote Gmail receives no settings permissions.
use crate::settings::{self, AppSettings, SettingsStore};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewWindow};

static SHORTCUT_ERROR: Mutex<Option<String>> = Mutex::new(None);

pub fn set_shortcut_error(app: &AppHandle, error: Option<String>) {
    *SHORTCUT_ERROR.lock().unwrap_or_else(|p| p.into_inner()) = error;
    crate::tray::refresh(app);
}

pub fn shortcut_error() -> Option<String> {
    SHORTCUT_ERROR
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

pub fn open(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("preferences") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let result = tauri::WebviewWindowBuilder::new(
        app,
        "preferences",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("GmailDock preferences")
    .inner_size(560.0, 680.0)
    .on_navigation(|url| {
        matches!(url.scheme(), "tauri" | "http" | "https")
            && matches!(url.host_str(), Some("localhost" | "tauri.localhost"))
    })
    .build();
    if let Err(error) = result {
        log::warn!("could not open preferences: {error}");
    }
}

/// macOS only: adds "Preferences…" (Cmd+,) to the standard app menu, keeping its Edit menu so
/// copy and paste keep working. Windows and Linux get no menu bar; Preferences is in the tray.
/// The tray's menu handler receives app-menu events too, so the "preferences" id needs no handler here.
#[cfg(target_os = "macos")]
pub fn install_menu(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItemBuilder, MenuItemKind, PredefinedMenuItem};
    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        let preferences = MenuItemBuilder::with_id("preferences", "Preferences…")
            .accelerator("CmdOrCtrl+,")
            .build(app)?;
        // After "About GmailDock" and its separator.
        app_menu.insert(&preferences, 2)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, 3)?;
    }
    app.set_menu(menu)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn install_menu(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    settings: AppSettings,
    shortcut_error: Option<String>,
    diagnostics: String,
}

#[tauri::command]
pub fn get_preferences(window: WebviewWindow, app: AppHandle) -> Result<Preferences, String> {
    if window.label() != "preferences" {
        return Err("untrusted window".into());
    }
    Ok(Preferences {
        settings: app.state::<SettingsStore>().snapshot(),
        shortcut_error: shortcut_error(),
        diagnostics: format!("GmailDock {}\nOS: {}\nArchitecture: {}\nNotification permissions: check system settings\n", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH),
    })
}

#[tauri::command]
pub fn save_preferences(
    window: WebviewWindow,
    app: AppHandle,
    settings: Option<AppSettings>,
) -> Result<AppSettings, String> {
    if window.label() != "preferences" {
        return Err("untrusted window".into());
    }
    settings::apply(&app, settings.unwrap_or_default())
}

//! Trusted local preferences UI. Remote Gmail receives no settings permissions.
use crate::settings::{self, AppSettings, SettingsStore};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Manager, WebviewWindow};

static SHORTCUT_ERROR: Mutex<Option<String>> = Mutex::new(None);
static OPENING: AtomicBool = AtomicBool::new(false);

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
    if OPENING.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    // Creating a WebView2 window inside a synchronous menu callback can deadlock.
    tauri::async_runtime::spawn_blocking(move || {
        show_window(&app);
        OPENING.store(false, Ordering::Release);
    });
}

fn show_window(app: &AppHandle) {
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
    settings: Option<PreferencesChanges>,
) -> Result<AppSettings, String> {
    if window.label() != "preferences" {
        return Err("untrusted window".into());
    }
    let next = match settings {
        Some(changes) => changes.apply_to(app.state::<SettingsStore>().snapshot()),
        None => AppSettings::default(),
    };
    settings::apply(&app, next)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesChanges {
    start_at_login: Option<bool>,
    start_minimized: Option<bool>,
    close_to_tray: Option<bool>,
    notifications: Option<bool>,
    open_external_in_browser: Option<bool>,
    auto_check_updates: Option<bool>,
    shortcut: Option<String>,
    zoom: Option<f64>,
}

impl PreferencesChanges {
    fn apply_to(self, mut current: AppSettings) -> AppSettings {
        current.start_at_login = self.start_at_login.unwrap_or(current.start_at_login);
        current.start_minimized = self.start_minimized.unwrap_or(current.start_minimized);
        current.close_to_tray = self.close_to_tray.unwrap_or(current.close_to_tray);
        current.notifications = self.notifications.unwrap_or(current.notifications);
        current.open_external_in_browser = self
            .open_external_in_browser
            .unwrap_or(current.open_external_in_browser);
        current.auto_check_updates = self
            .auto_check_updates
            .unwrap_or(current.auto_check_updates);
        current.zoom = self.zoom.unwrap_or(current.zoom);
        if let Some(shortcut) = self.shortcut {
            current.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        }
        current
    }
}

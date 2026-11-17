#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
    sync::atomic::{AtomicI64, Ordering},
};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::NewWindowResponse,
    Manager,
};
use tauri_plugin_autostart::MacosLauncher;
#[cfg(not(debug_assertions))]
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

const GMAIL_URL: &str = "https://mail.google.com/";
const TOGGLE_SHORTCUT: &str = "CommandOrControl+Shift+G";
const ACCOUNTS_URL: &str =
    "https://accounts.google.com/AccountChooser?service=mail&continue=https://mail.google.com/mail/";
const MINIMIZED_ARG: &str = "--minimized";
const SETTINGS_FILE: &str = "settings.json";
const DEVELOPER_URL: &str = "https://github.com/nooblk-98";
const REPOSITORY_URL: &str = "https://github.com/nooblk-98/gmaildock";

fn is_google_popup(url: &tauri::Url) -> bool {
    matches!(
        url.host_str(),
        Some("accounts.google.com" | "mail.google.com")
    ) || (url.scheme() == "about" && url.path() == "blank")
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct AppSettings {
    auto_check_updates: bool,
    close_to_tray: bool,
    start_at_login: bool,
    notifications: bool,
    start_minimized: bool,
    zoom: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_check_updates: true,
            close_to_tray: true,
            start_at_login: true,
            notifications: true,
            start_minimized: false,
            zoom: 1.0,
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(SETTINGS_FILE))
        .map_err(|error| error.to_string())
}

fn clamp_zoom(zoom: f64) -> f64 {
    if zoom.is_finite() {
        zoom.clamp(0.5, 2.0)
    } else {
        1.0
    }
}

fn show_main_window(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    let window = app.get_webview_window("main")?;
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    Some(window)
}

fn toggle_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Last Inbox unread count reported by the page; -1 until the first report.
struct UnreadState(AtomicI64);

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    match fs::read(path) {
        Ok(contents) => serde_json::from_slice(&contents).map_err(|error| error.to_string()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, mut settings: AppSettings) -> Result<(), String> {
    settings.zoom = clamp_zoom(settings.zoom);
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_zoom(settings.zoom)
            .map_err(|error| error.to_string())?;
    }
    #[cfg(not(debug_assertions))]
    {
        let autolaunch = app.autolaunch();
        if settings.start_at_login {
            autolaunch.enable().map_err(|error| error.to_string())?;
        } else {
            autolaunch.disable().map_err(|error| error.to_string())?;
        }
    }
    let path = settings_path(&app)?;
    let parent = path.parent().ok_or("invalid settings path")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let contents = serde_json::to_vec(&settings).map_err(|error| error.to_string())?;
    fs::write(path, contents).map_err(|error| error.to_string())
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<AvailableUpdate>, String> {
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

#[derive(Serialize)]
struct AvailableUpdate {
    version: String,
    notes: Option<String>,
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum AboutLink {
    Developer,
    Repository,
}

#[tauri::command]
fn open_about_link(link: AboutLink) -> Result<(), String> {
    let url = match link {
        AboutLink::Developer => DEVELOPER_URL,
        AboutLink::Repository => REPOSITORY_URL,
    };
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("No update is available")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;
    app.restart();
}

#[cfg(target_os = "windows")]
fn badge_image(count: u32) -> tauri::image::Image<'static> {
    const SIZE: usize = 32;
    const GLYPHS: [[u8; 5]; 11] = [
        [0b111, 0b101, 0b101, 0b101, 0b111],
        [0b010, 0b110, 0b010, 0b010, 0b111],
        [0b111, 0b001, 0b111, 0b100, 0b111],
        [0b111, 0b001, 0b111, 0b001, 0b111],
        [0b101, 0b101, 0b111, 0b001, 0b001],
        [0b111, 0b100, 0b111, 0b001, 0b111],
        [0b111, 0b100, 0b111, 0b101, 0b111],
        [0b111, 0b001, 0b010, 0b010, 0b010],
        [0b111, 0b101, 0b111, 0b101, 0b111],
        [0b111, 0b101, 0b111, 0b001, 0b111],
        [0b000, 0b010, 0b111, 0b010, 0b000],
    ];
    let label = if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    };
    let scale = match label.len() {
        1 => 4,
        2 => 3,
        _ => 2,
    };
    let width = (label.len() * 3 + label.len() - 1) * scale;
    let left = (SIZE - width) / 2;
    let top = (SIZE - 5 * scale) / 2;
    let mut rgba = vec![0; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            if dx * dx + dy * dy <= 15 * 15 {
                rgba[(y * SIZE + x) * 4..(y * SIZE + x) * 4 + 4]
                    .copy_from_slice(&[220, 38, 38, 255]);
            }
        }
    }
    for (index, digit) in label.bytes().enumerate() {
        let glyph = if digit == b'+' {
            10
        } else {
            (digit - b'0') as usize
        };
        for (row, bits) in GLYPHS[glyph].iter().enumerate() {
            for column in 0..3 {
                if bits & (0b100 >> column) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let x = left + (index * 4 + column) * scale + sx;
                        let y = top + row * scale + sy;
                        rgba[(y * SIZE + x) * 4..(y * SIZE + x) * 4 + 4]
                            .copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
            }
        }
    }
    tauri::image::Image::new_owned(rgba, SIZE as u32, SIZE as u32)
}

#[tauri::command]
fn set_unread_count(window: tauri::WebviewWindow, count: u32) -> Result<(), String> {
    let app = window.app_handle();
    let previous = app
        .state::<UnreadState>()
        .0
        .swap(i64::from(count), Ordering::Relaxed);
    let wants_notification = previous >= 0
        && i64::from(count) > previous
        && !window.is_focused().unwrap_or(false)
        && get_settings(app.clone()).unwrap_or_default().notifications;
    if wants_notification {
        let _ = app
            .notification()
            .builder()
            .title("GmailDock")
            .body(format!("{count} unread in your Inbox"))
            .show();
    }
    #[cfg(target_os = "windows")]
    let result = window.set_overlay_icon((count > 0).then(|| badge_image(count)));
    #[cfg(not(target_os = "windows"))]
    let result = window.set_badge_count((count > 0).then_some(i64::from(count)));
    result.map_err(|error| error.to_string())
}

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(UnreadState(AtomicI64::new(-1)))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = get_settings(window.app_handle().clone())
                    .unwrap_or_default()
                    .close_to_tray;
                api.prevent_close();
                if close_to_tray {
                    if let Err(error) = window.hide() {
                        eprintln!("failed to hide GmailDock window: {error}");
                    }
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            check_for_updates,
            install_update,
            set_unread_count,
            get_app_version,
            open_about_link,
            get_settings,
            save_settings
        ])
        .setup(|app| {
            #[cfg(not(debug_assertions))]
            {
                let settings = get_settings(app.handle().clone()).unwrap_or_default();
                if settings.start_at_login {
                    if let Err(error) = app.autolaunch().enable() {
                        eprintln!("failed to enable GmailDock autostart: {error}");
                    }
                }
            }
            let settings = get_settings(app.handle().clone()).unwrap_or_default();
            // Ignore failures: the shortcut may be taken by another app or unsupported (e.g. Wayland).
            let _ = app
                .global_shortcut()
                .on_shortcut(TOGGLE_SHORTCUT, |app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                });
            let start_hidden =
                settings.start_minimized && std::env::args().any(|arg| arg == MINIMIZED_ARG);
            let menu = MenuBuilder::new(app)
                .text("accounts", "Accounts...")
                .text("settings", "Settings")
                .text("reload", "Reload")
                .text("quit", "Quit")
                .build()?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| std::io::Error::other("GmailDock icon is missing"))?;
            TrayIconBuilder::new()
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
                        show_main_window(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "accounts" => {
                        if let Some(window) = show_main_window(app) {
                            if let Ok(url) = tauri::Url::parse(ACCOUNTS_URL) {
                                let _ = window.navigate(url);
                            }
                        }
                    }
                    "settings" => {
                        if let Some(window) = show_main_window(app) {
                            let _ = window.eval("window.__gmaildockOpenSettings?.()");
                        }
                    }
                    "reload" => {
                        if let Some(window) = show_main_window(app) {
                            let _ = window.eval("location.reload()");
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            let url = tauri::Url::parse(GMAIL_URL)?;
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url))
                .title("GmailDock")
                .visible(!start_hidden)
                .inner_size(1200.0, 780.0)
                .min_inner_size(780.0, 540.0)
                .initialization_script_for_all_frames(include_str!("injected.js"))
                .on_new_window(|url, _| {
                    if is_google_popup(&url) {
                        NewWindowResponse::Allow
                    } else {
                        if matches!(url.scheme(), "http" | "https") {
                            let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
                        }
                        NewWindowResponse::Deny
                    }
                })
                .build()?;
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = window.set_zoom(clamp_zoom(settings.zoom)) {
                    eprintln!("failed to restore GmailDock zoom: {error}");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::{is_google_popup, AppSettings};

    #[test]
    fn existing_settings_default_to_closing_to_tray() {
        let saved = r#"{"autoCheckUpdates":false,"hideAccountLink":false}"#;
        let settings: AppSettings = serde_json::from_str(saved).expect("valid settings JSON");
        assert!(settings.close_to_tray);
        assert!(settings.start_at_login);
        assert!(settings.notifications);
        assert!(!settings.start_minimized);
        assert_eq!(settings.zoom, 1.0);
    }

    #[test]
    fn zoom_is_clamped_to_a_usable_range() {
        assert_eq!(super::clamp_zoom(0.1), 0.5);
        assert_eq!(super::clamp_zoom(9.0), 2.0);
        assert_eq!(super::clamp_zoom(f64::NAN), 1.0);
        assert_eq!(super::clamp_zoom(1.25), 1.25);
    }

    #[test]
    fn google_account_popups_stay_in_app() {
        let account = tauri::Url::parse("https://accounts.google.com/AddSession").unwrap();
        let blank = tauri::Url::parse("about:blank#login").unwrap();
        let external = tauri::Url::parse("https://example.com/").unwrap();
        assert!(is_google_popup(&account));
        assert!(is_google_popup(&blank));
        assert!(!is_google_popup(&external));
    }
}

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, Manager, WebviewWindow,
};

use crate::{
    badge::WINDOW_TITLE,
    downloads,
    settings::{AppSettings, SettingsStore},
};

pub const LABEL: &str = "main";
pub const GMAIL_URL: &str = "https://mail.google.com/";
pub const ACCOUNTS_URL: &str =
    "https://accounts.google.com/AccountChooser?service=mail&continue=https://mail.google.com/mail/";
pub const MINIMIZED_ARG: &str = "--minimized";

/// How long to wait for the first page load before showing the window anyway.
const REVEAL_FALLBACK: Duration = Duration::from_secs(4);

pub fn show_main_window(app: &AppHandle) -> Option<WebviewWindow> {
    let window = app.get_webview_window(LABEL)?;
    reveal(&window);
    Some(window)
}

pub fn toggle_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        let _ = window.hide();
    } else {
        reveal(&window);
    }
}

pub fn create(app: &AppHandle, settings: AppSettings) -> tauri::Result<WebviewWindow> {
    let start_hidden = settings.start_minimized && std::env::args().any(|arg| arg == MINIMIZED_ARG);
    let open_settings = std::env::args().any(|arg| arg == crate::settings_window::ARG);
    let show_on_load = !start_hidden && !open_settings;

    let revealed = Arc::new(AtomicBool::new(false));
    let page_revealed = Arc::clone(&revealed);
    let loaded_once = Arc::new(AtomicBool::new(false));
    let navigation_app = app.clone();

    let gmail_url = tauri::Url::parse(GMAIL_URL).map_err(std::io::Error::other)?;
    let window =
        tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::External(gmail_url))
            .title(WINDOW_TITLE)
            .visible(false)
            .inner_size(1200.0, 780.0)
            .min_inner_size(780.0, 540.0)
            .initialization_script_for_all_frames(include_str!("injected.js"))
            .on_page_load(move |window, payload| {
                if payload.event() != PageLoadEvent::Finished {
                    return;
                }
                if !loaded_once.swap(true, Ordering::Relaxed)
                    && show_on_load
                    && !page_revealed.swap(true, Ordering::Relaxed)
                {
                    reveal(&window);
                }
            })
            .on_navigation(move |url| {
                let settings = navigation_app.state::<SettingsStore>().snapshot();
                if allow_in_app(url, settings.open_external_in_browser) {
                    return true;
                }
                open_externally(url);
                false
            })
            .on_new_window(|url, _| {
                if is_google_popup(&url) {
                    NewWindowResponse::Allow
                } else {
                    open_externally(&url);
                    NewWindowResponse::Deny
                }
            })
            .on_download(downloads::handle)
            .build()?;

    if let Err(error) = window.set_zoom(settings.zoom) {
        log::warn!("could not restore zoom: {error}");
    }

    if show_on_load {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(REVEAL_FALLBACK);
            if revealed.swap(true, Ordering::Relaxed) {
                return;
            }
            if let Some(window) = app.get_webview_window(LABEL) {
                if !window.is_visible().unwrap_or(false) {
                    log::info!("showing the window before Gmail finished loading");
                    reveal(&window);
                }
            }
        });
    }

    Ok(window)
}

fn reveal(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

pub fn allow_in_app(url: &tauri::Url, open_external_in_browser: bool) -> bool {
    if url.scheme() == "about" {
        return url.path() == "blank";
    }
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    is_google_host(url.host_str().unwrap_or_default()) || !open_external_in_browser
}

pub fn is_google_popup(url: &tauri::Url) -> bool {
    if url.scheme() == "about" {
        return url.path() == "blank";
    }
    matches!(url.scheme(), "http" | "https") && is_google_host(url.host_str().unwrap_or_default())
}

fn is_google_host(host: &str) -> bool {
    host == "google.com"
        || host.ends_with(".google.com")
        || host == "accounts.youtube.com"
        || host.ends_with(".googleusercontent.com")
}

fn open_externally(url: &tauri::Url) {
    if !matches!(url.scheme(), "http" | "https") {
        return;
    }
    if let Err(error) = tauri_plugin_opener::open_url(url.as_str(), None::<&str>) {
        log::warn!("could not open {url} in the browser: {error}");
    }
}

#[cfg(test)]
mod tests {
    use tauri::Url;

    use super::{allow_in_app, is_google_popup};

    fn url(raw: &str) -> Url {
        Url::parse(raw).expect("valid URL")
    }

    #[test]
    fn google_hosts_stay_in_the_app() {
        assert!(allow_in_app(
            &url("https://mail.google.com/mail/u/0/"),
            true
        ));
        assert!(allow_in_app(
            &url("https://accounts.google.com/AddSession"),
            true
        ));
        assert!(allow_in_app(&url("https://myaccount.google.com/"), true));
        assert!(allow_in_app(&url("https://www.google.com/"), true));
        assert!(allow_in_app(&url("about:blank"), true));
        assert!(allow_in_app(
            &url("https://mail.googleusercontent.com/attachment"),
            true
        ));
        assert!(allow_in_app(
            &url("https://accounts.youtube.com/accounts/CheckConnection"),
            true
        ));
    }

    #[test]
    fn external_sites_follow_the_setting() {
        assert!(!allow_in_app(&url("https://example.com/"), true));
        assert!(allow_in_app(&url("https://example.com/"), false));
    }

    #[test]
    fn non_web_navigations_are_never_allowed() {
        assert!(!allow_in_app(&url("file:///etc/passwd"), false));
        assert!(!allow_in_app(&url("data:text/html,hello"), false));
    }

    #[test]
    fn google_account_popups_stay_in_app() {
        assert!(is_google_popup(&url(
            "https://accounts.google.com/AddSession"
        )));
        assert!(is_google_popup(&url("about:blank#login")));
        assert!(!is_google_popup(&url("https://example.com/")));
        assert!(!is_google_popup(&url("file:///etc/passwd")));
    }
}

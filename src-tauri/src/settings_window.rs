use tauri::{AppHandle, Manager, WebviewWindow};

pub const LABEL: &str = "settings";
pub const ARG: &str = "--settings";

pub fn open(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(window);
    }

    tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("settings.html".into()))
        .title("GmailDock Settings")
        .inner_size(560.0, 720.0)
        .min_inner_size(460.0, 520.0)
        .build()
}

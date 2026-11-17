use std::path::Path;

use tauri::{webview::DownloadEvent, Manager, Webview};
use tauri_plugin_notification::NotificationExt;

pub fn handle(webview: Webview, event: DownloadEvent<'_>) -> bool {
    match event {
        DownloadEvent::Requested { url, .. } => log::info!("download requested: {url}"),
        DownloadEvent::Finished { url, path, success } => {
            if !success {
                log::warn!("download failed: {url}");
            } else if let Some(path) = path {
                log::info!("download finished: {}", path.display());
                notify(webview.app_handle(), &path);
            } else {
                log::info!("download finished: {url}");
            }
        }
        _ => {}
    }

    true
}

fn notify(app: &tauri::AppHandle, path: &Path) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let body = if name.is_empty() {
        "Saved to your Downloads folder".to_owned()
    } else {
        name
    };

    if let Err(error) = app
        .notification()
        .builder()
        .title("Download complete")
        .body(body)
        .show()
    {
        log::warn!("could not show the download notification: {error}");
    }
}

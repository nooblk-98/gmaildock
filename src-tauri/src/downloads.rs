//! Download completion handling, including WebKit's missing completion path.
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
use tauri::{webview::DownloadEvent, AppHandle, Manager, Webview};

#[derive(Default)]
pub struct DownloadState {
    pending: Mutex<HashMap<String, PendingDownloads>>,
    latest: Mutex<Option<PathBuf>>,
}

struct PendingDownloads {
    count: usize,
    destination: Option<PathBuf>,
}

pub fn handle(webview: Webview, event: DownloadEvent<'_>) -> bool {
    let app = webview.app_handle();
    let state = app.state::<DownloadState>();
    match event {
        DownloadEvent::Requested { url, destination } => {
            log::info!("download requested: {url}");
            state
                .pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .entry(url.to_string())
                .and_modify(|downloads| {
                    downloads.count += 1;
                    // A URL alone can't identify which of these downloads finished.
                    downloads.destination = None;
                })
                .or_insert_with(|| PendingDownloads {
                    count: 1,
                    destination: Some(destination.clone()),
                });
        }
        DownloadEvent::Finished { url, path, success } => {
            let mut pending = state.pending.lock().unwrap_or_else(|p| p.into_inner());
            let fallback = pending.get_mut(url.as_str()).and_then(|downloads| {
                downloads.count -= 1;
                downloads.destination.take()
            });
            if pending
                .get(url.as_str())
                .is_some_and(|downloads| downloads.count == 0)
            {
                pending.remove(url.as_str());
            }
            drop(pending);
            if !success {
                log::warn!("download failed: {url}");
                crate::tray::notify(
                    app,
                    "Download failed",
                    "Please retry the download in Gmail.",
                );
            } else {
                let path = path.or(fallback);
                match &path {
                    Some(path) => log::info!("download finished: {}", path.display()),
                    None => log::info!("download finished: {url}"),
                }
                let body = path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Your download has finished".into());
                *state.latest.lock().unwrap_or_else(|p| p.into_inner()) = path;
                crate::tray::notify(app, "Download complete", &body);
            }
        }
        _ => {}
    }
    true
}

pub fn open_folder(app: &AppHandle) {
    let latest = app
        .state::<DownloadState>()
        .latest
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    if let Some(path) = latest.filter(|p| p.exists()) {
        if tauri_plugin_opener::reveal_item_in_dir(path).is_ok() {
            return;
        }
    }
    if let Ok(dir) = app.path().download_dir() {
        if let Err(error) = tauri_plugin_opener::open_path(dir, None::<&str>) {
            log::warn!("could not open Downloads: {error}");
        }
    }
}

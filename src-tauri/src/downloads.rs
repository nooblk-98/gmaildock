//! Download completion handling, including WebKit's missing completion path.
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Mutex,
};
use tauri::{webview::DownloadEvent, AppHandle, Manager, Webview};

#[derive(Default)]
pub struct DownloadState {
    pending: Mutex<HashMap<String, VecDeque<PathBuf>>>,
    latest: Mutex<Option<PathBuf>>,
}

pub fn handle(webview: Webview, event: DownloadEvent<'_>) -> bool {
    let app = webview.app_handle();
    let state = app.state::<DownloadState>();
    match event {
        DownloadEvent::Requested { url, destination } => {
            state
                .pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .entry(url.to_string())
                .or_default()
                .push_back(destination.clone());
        }
        DownloadEvent::Finished { url, path, success } => {
            let mut pending = state.pending.lock().unwrap_or_else(|p| p.into_inner());
            let fallback = pending.get_mut(url.as_str()).and_then(VecDeque::pop_front);
            if pending.get(url.as_str()).is_some_and(VecDeque::is_empty) {
                pending.remove(url.as_str());
            }
            drop(pending);
            if !success {
                crate::tray::notify(
                    app,
                    "Download failed",
                    "Please retry the download in Gmail.",
                );
            } else {
                let path = path.or(fallback);
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

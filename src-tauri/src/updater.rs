use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
pub struct AvailableUpdate {
    version: String,
    notes: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<AvailableUpdate>, String> {
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

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
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
    log::info!("update installed; restarting GmailDock");
    app.restart();
}

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

const DEVELOPER_URL: &str = "https://github.com/nooblk-98";
const REPOSITORY_URL: &str = "https://github.com/nooblk-98/gmaildock";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AboutLink {
    Developer,
    Repository,
}

#[tauri::command]
pub fn open_about_link(link: AboutLink) -> Result<(), String> {
    let url = match link {
        AboutLink::Developer => DEVELOPER_URL,
        AboutLink::Repository => REPOSITORY_URL,
    };
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

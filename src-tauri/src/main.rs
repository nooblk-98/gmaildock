#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod badge;
mod downloads;
mod settings;
mod shortcut;
mod tray;
mod updater;
mod window;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_window_state::StateFlags;

use crate::{
    badge::UnreadState, settings::SettingsStore, updater::UpdateState, window::MINIMIZED_ARG,
};

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("gmaildock".into()),
                    }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = window::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    StateFlags::SIZE
                        | StateFlags::POSITION
                        | StateFlags::MAXIMIZED
                        | StateFlags::FULLSCREEN,
                )
                .build(),
        )
        .manage(UnreadState::new())
        .manage(UpdateState::default())
        .on_window_event(|window, event| {
            if window.label() != window::LABEL {
                return;
            }
            if matches!(
                event,
                WindowEvent::Focused(true) | WindowEvent::ScaleFactorChanged { .. }
            ) {
                if let Some(webview) = window.app_handle().get_webview_window(window::LABEL) {
                    badge::refresh(&webview);
                }
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let settings = window.app_handle().state::<SettingsStore>().snapshot();
                if settings.close_to_tray {
                    if let Err(error) = window.hide() {
                        log::warn!("could not hide the window: {error}");
                    }
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![badge::set_unread_count])
        .setup(|app| {
            log::info!("GmailDock {} starting", env!("CARGO_PKG_VERSION"));

            let store = SettingsStore::load(settings::settings_path(app.handle())?);
            let settings = store.snapshot();
            app.manage(store);

            #[cfg(not(debug_assertions))]
            if settings.start_at_login {
                use tauri_plugin_autostart::ManagerExt;

                if let Err(error) = app.autolaunch().enable() {
                    log::warn!("could not enable autostart: {error}");
                }
            }

            if let Err(error) = shortcut::register(app.handle(), settings.shortcut.as_deref()) {
                log::warn!("global shortcut is unavailable: {error}");
            }

            tray::build(app.handle())?;
            window::create(app.handle(), settings)?;
            updater::spawn_background_checks(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
}

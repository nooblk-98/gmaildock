#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod badge;
mod downloads;
mod settings;
mod tray;
mod updater;
mod window;

use std::sync::atomic::AtomicBool;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{badge::UnreadState, settings::SettingsStore, window::MINIMIZED_ARG};

const TOGGLE_SHORTCUT: &str = "CommandOrControl+Shift+G";

/// Set when the Settings panel should open once Gmail finishes loading.
#[derive(Default)]
pub struct PendingSettings(pub AtomicBool);

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
        .manage(UnreadState::new())
        .manage(PendingSettings::default())
        .on_window_event(|window, event| {
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
        .invoke_handler(tauri::generate_handler![
            badge::set_unread_count,
            settings::get_settings,
            settings::save_settings,
            updater::check_for_updates,
            updater::install_update,
            updater::get_app_version,
            updater::open_about_link,
        ])
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

            // The shortcut may be taken by another app or unsupported (e.g. Wayland).
            if let Err(error) =
                app.global_shortcut()
                    .on_shortcut(TOGGLE_SHORTCUT, |app, _, event| {
                        if event.state() == ShortcutState::Pressed {
                            window::toggle_window(app);
                        }
                    })
            {
                log::warn!("global shortcut {TOGGLE_SHORTCUT} is unavailable: {error}");
            }

            tray::build(app.handle())?;
            window::create(app.handle(), settings)?;
            Ok(())
        })
        .run(tauri::generate_context!())
}

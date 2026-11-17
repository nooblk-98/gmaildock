use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::window;

pub fn apply(app: &AppHandle, previous: Option<&str>, next: Option<&str>) -> Result<(), String> {
    if previous == next {
        return Ok(());
    }

    match register(app, next) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = register(app, previous);
            Err(error)
        }
    }
}

pub fn register(app: &AppHandle, shortcut: Option<&str>) -> Result<(), String> {
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|error| error.to_string())?;

    let Some(shortcut) = shortcut.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    manager
        .on_shortcut(shortcut, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                window::toggle_window(app);
            }
        })
        .map_err(|error| format!("{shortcut} is not available: {error}"))
}

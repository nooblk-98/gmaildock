use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_notification::NotificationExt;

use crate::{settings::SettingsStore, tray::TRAY_ID};

pub const WINDOW_TITLE: &str = "GmailDock";

/// Suppresses a burst of notifications while mail keeps arriving.
const NOTIFICATION_COOLDOWN_SECS: i64 = 60;

pub struct UnreadState {
    count: AtomicI64,
    last_notification: AtomicI64,
}

impl UnreadState {
    pub fn new() -> Self {
        Self {
            count: AtomicI64::new(-1),
            last_notification: AtomicI64::new(0),
        }
    }
}

impl Default for UnreadState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn window_title(count: u32) -> String {
    if count == 0 {
        WINDOW_TITLE.to_owned()
    } else {
        format!("({count}) {WINDOW_TITLE}")
    }
}

pub fn tray_tooltip(count: u32) -> String {
    match count {
        0 => WINDOW_TITLE.to_owned(),
        1 => format!("{WINDOW_TITLE} — 1 unread message"),
        other => format!("{WINDOW_TITLE} — {other} unread messages"),
    }
}

fn notification_body(count: u32) -> String {
    match count {
        1 => "1 unread message in your Inbox".to_owned(),
        other => format!("{other} unread messages in your Inbox"),
    }
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[tauri::command]
pub fn set_unread_count(
    window: WebviewWindow,
    state: State<'_, UnreadState>,
    store: State<'_, SettingsStore>,
    count: u32,
) -> Result<(), String> {
    let previous = state.count.swap(i64::from(count), Ordering::Relaxed);

    if let Err(error) = window.set_title(&window_title(count)) {
        log::debug!("could not update the window title: {error}");
    }
    if let Some(tray) = window.app_handle().tray_by_id(TRAY_ID) {
        if let Err(error) = tray.set_tooltip(Some(tray_tooltip(count))) {
            log::debug!("could not update the tray tooltip: {error}");
        }
    }
    if let Err(error) = apply_badge(&window, count) {
        log::debug!("could not update the unread badge: {error}");
    }

    let new_mail = previous >= 0 && i64::from(count) > previous;
    let cooling_down = unix_seconds() - state.last_notification.load(Ordering::Relaxed)
        < NOTIFICATION_COOLDOWN_SECS;
    if new_mail
        && !cooling_down
        && !window.is_focused().unwrap_or(false)
        && store.snapshot().notifications
    {
        state
            .last_notification
            .store(unix_seconds(), Ordering::Relaxed);
        if let Err(error) = window
            .app_handle()
            .notification()
            .builder()
            .title(WINDOW_TITLE)
            .body(notification_body(count))
            .show()
        {
            log::warn!("could not show the new mail notification: {error}");
        }
    }

    Ok(())
}

fn apply_badge(window: &WebviewWindow, count: u32) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    {
        window.set_overlay_icon((count > 0).then(|| badge_image(count)))
    }
    #[cfg(not(target_os = "windows"))]
    {
        window.set_badge_count((count > 0).then_some(i64::from(count)))
    }
}

#[cfg(target_os = "windows")]
fn badge_label(count: u32) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
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

    let label = badge_label(count);
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
                let offset = (y * SIZE + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&[220, 38, 38, 255]);
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
                        let offset = (y * SIZE + x) * 4;
                        rgba[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
            }
        }
    }

    tauri::image::Image::new_owned(rgba, SIZE as u32, SIZE as u32)
}

#[cfg(test)]
mod tests {
    use super::{notification_body, tray_tooltip, window_title};

    #[test]
    fn window_title_uses_the_unread_count() {
        assert_eq!(window_title(0), "GmailDock");
        assert_eq!(window_title(7), "(7) GmailDock");
    }

    #[test]
    fn tray_tooltip_is_readable() {
        assert_eq!(tray_tooltip(0), "GmailDock");
        assert_eq!(tray_tooltip(1), "GmailDock — 1 unread message");
        assert_eq!(tray_tooltip(12), "GmailDock — 12 unread messages");
    }

    #[test]
    fn notification_body_uses_the_unread_count() {
        assert_eq!(notification_body(1), "1 unread message in your Inbox");
        assert_eq!(notification_body(9), "9 unread messages in your Inbox");
    }
}

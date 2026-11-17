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

/// Puts the unread count back in the tray tooltip, e.g. after an update download used it.
pub fn restore_tooltip(app: &tauri::AppHandle) {
    let count = app.state::<UnreadState>().count.load(Ordering::Relaxed);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tray_tooltip(u32::try_from(count).unwrap_or(0))));
    }
}

/// Re-applies the last known unread count. Windows drops the taskbar overlay whenever the
/// taskbar button is recreated (hiding to the tray, Explorer restarts, DPI changes), and Gmail
/// only reports the count when it changes, so the window calls this when it is shown again.
pub fn refresh(window: &WebviewWindow) {
    let count = window
        .app_handle()
        .state::<UnreadState>()
        .count
        .load(Ordering::Relaxed);
    let Ok(count) = u32::try_from(count) else {
        return;
    };
    if let Err(error) = apply_badge(window, count) {
        log::debug!("could not refresh the unread badge: {error}");
    }
}

fn apply_badge(window: &WebviewWindow, count: u32) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let size = overlay_size(window.scale_factor().unwrap_or(1.0));
        window.set_overlay_icon((count > 0).then(|| {
            tauri::image::Image::new_owned(badge_pixels(count, size), size as u32, size as u32)
        }))
    }
    #[cfg(not(target_os = "windows"))]
    {
        window.set_badge_count((count > 0).then_some(i64::from(count)))
    }
}

/// Windows draws taskbar overlays at the small-icon size: 16 px at 100% scaling, 24 px at 150%.
/// Rendering at exactly that size avoids the resampling that blurred the old 32 px badge.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn overlay_size(scale_factor: f64) -> usize {
    let scale = if scale_factor.is_finite() {
        scale_factor
    } else {
        1.0
    };
    ((16.0 * scale).round() as usize).clamp(16, 64)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn badge_label(count: u32) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
}

/// 3x5 pixel glyphs for 0-9 and "+", drawn at whole-pixel multiples so edges stay sharp.
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

/// RGBA pixels for a red circular badge of `size` x `size` showing `count`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn badge_pixels(count: u32, size: usize) -> Vec<u8> {
    const RED: [u8; 3] = [220, 38, 38];
    let mut rgba = vec![0; size * size * 4];

    // Anti-aliased circle: alpha follows how much of each pixel the circle covers.
    let center = size as f64 / 2.0;
    let radius = center;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 + 0.5 - center;
            let dy = y as f64 + 0.5 - center;
            let coverage = (radius - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let offset = (y * size + x) * 4;
                rgba[offset..offset + 3].copy_from_slice(&RED);
                rgba[offset + 3] = (coverage * 255.0).round() as u8;
            }
        }
    }

    let label = badge_label(count);
    let glyphs = label.len();
    // Largest whole-pixel scale that keeps the text inside the circle.
    let text_width = glyphs * 3 + (glyphs - 1);
    let usable = size - size / 4;
    let scale = (usable / text_width).min(usable / 5).max(1);
    let width = text_width * scale;
    let height = 5 * scale;
    let left = size.saturating_sub(width) / 2;
    let top = size.saturating_sub(height) / 2;

    for (index, character) in label.bytes().enumerate() {
        let glyph = match character {
            b'+' => 10,
            digit => usize::from(digit - b'0'),
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
                        if x < size && y < size {
                            let offset = (y * size + x) * 4;
                            rgba[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
                        }
                    }
                }
            }
        }
    }

    rgba
}

#[cfg(test)]
mod badge_image_tests {
    use super::{badge_label, badge_pixels, overlay_size};

    #[test]
    fn overlay_matches_the_windows_small_icon_size() {
        assert_eq!(overlay_size(1.0), 16);
        assert_eq!(overlay_size(1.25), 20);
        assert_eq!(overlay_size(1.5), 24);
        assert_eq!(overlay_size(2.0), 32);
        assert_eq!(overlay_size(f64::NAN), 16);
    }

    #[test]
    fn large_counts_show_99_plus() {
        assert_eq!(badge_label(2093), "99+");
        assert_eq!(badge_label(99), "99");
        assert_eq!(badge_label(7), "7");
    }

    #[test]
    fn every_label_fits_at_every_size() {
        for size in [16, 20, 24, 32, 48] {
            for count in [1, 9, 42, 99, 2093] {
                let pixels = badge_pixels(count, size);
                assert_eq!(pixels.len(), size * size * 4);
                // Text pixels are opaque white and must sit inside the circle, not on its edge.
                let mut white = 0;
                for y in 0..size {
                    for x in 0..size {
                        let offset = (y * size + x) * 4;
                        if pixels[offset..offset + 4] == [255, 255, 255, 255] {
                            white += 1;
                            assert!(x > 0 && y > 0 && x < size - 1 && y < size - 1);
                        }
                    }
                }
                assert!(white > 0, "count {count} at {size}px drew no text");
            }
        }
    }
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

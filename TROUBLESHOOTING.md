# Troubleshooting

Common problems and fixes for GmailDock. If yours is not listed, [open an issue](https://github.com/nooblk-98/gmaildock/issues) with your OS version, package type, and the diagnostics from **Preferences…**.

## Installing: security warnings

Builds currently have no paid OS code signing or Apple notarization. Download only from this repository's [Releases page](https://github.com/nooblk-98/gmaildock/releases).

- **macOS:** if Gatekeeper blocks a trusted download, use System Settings → Privacy & Security → **Open Anyway**.
- **Windows:** an unsigned build may trigger SmartScreen.

## Linux

- **No tray icon:** GNOME may need an AppIndicator extension to display the tray.
- **Unread badge:** dock badges require a desktop that supports the Unity launcher API. The window title and tray tooltip also show unread counts.
- **Global shortcut:** shortcuts can fail on Wayland or when another application reserves the keys. The tray then shows **Shortcut unavailable**; pick another shortcut there or enter a custom one in **Preferences…**.

## Preferences and diagnostics

Open **Preferences…** from the tray (or with `Cmd+,` on macOS). Diagnostics can be selected and copied there. Resetting preferences keeps your Google session.

## Notifications

After denying notifications, enable GmailDock in your OS notification settings. The desktop notification plugin cannot reliably report OS denial.

## Downloads

Downloads finish with a notification even when WebKit omits the destination. **Troubleshooting → Open Downloads folder** reveals the latest downloaded file when available.

## Uninstalling

Before uninstalling, turn off **Start at login** (tray → Preferences). Removing a macOS app by dragging it to Trash does not run an uninstall hook. If you already removed it:

- **macOS:** delete its GmailDock LaunchAgent in `~/Library/LaunchAgents`.
- **Linux:** remove its GmailDock entry in `~/.config/autostart`.
- **Windows:** disable its entry in Startup Apps.

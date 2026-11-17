<div align="center">
  <img src="assets/gmail.svg" width="80" height="80" alt="GmailDock icon">

  # GmailDock

  Fast, lightweight Gmail on your desktop.

  [![Build and release](https://github.com/nooblk-98/gmaildock/actions/workflows/build-release.yml/badge.svg)](https://github.com/nooblk-98/gmaildock/actions/workflows/build-release.yml)
  [![Latest release](https://img.shields.io/github/v/release/nooblk-98/gmaildock?label=release)](https://github.com/nooblk-98/gmaildock/releases)
</div>

GmailDock opens [mail.google.com](https://mail.google.com/) in your operating system's webview. Its small Rust and Tauri shell is designed for fast startup and low app overhead while keeping Gmail's own interface. There is no Gmail API setup or OAuth client ID.

Inspired by [whatRust](https://github.com/karem505/whatRust).

## Features

- Gmail in a dedicated desktop window on Windows, macOS, and Linux.
- Reuses the system webview instead of bundling a browser engine, keeping the app lightweight. Windows uses WebView2. Gmail's own memory and CPU use still depends on the page and your account.
- Has a Settings panel (tray menu, Settings) for automatic update checks, close behavior, notifications, and whether non-Google links open in your browser. It always hides the Google Account shortcut and Google apps launcher.
- Closing the window keeps GmailDock running in the system tray by default. Change this in Settings; left-click the tray icon to open Gmail, or right-click for Accounts, Settings, Open in browser, Reload, Troubleshooting, and Quit.
- Opening GmailDock again focuses the running window instead of creating another tray icon.
- Keeps non-Google links out of the app window: they open in your default browser. Turn this off in Settings if a sign-in flow needs to stay inside GmailDock.
- Shows a notification when a download finishes.
- Writes a log file; open it from the tray (Troubleshooting, Open logs folder) when reporting a problem.
- Checks for updates in the background every 30 minutes when enabled. An install button appears only when an update is available.
- Shows release notes and an Update now button in a dialog when an update is available.
- Can start automatically when you sign in; this is on by default and can be changed in Settings.
- Settings includes an About section with the developer profile and source repository.
- Supports Google account popups used by Gmail's Add another account action.
- Shows your Inbox unread count as a taskbar or dock badge. Other mailboxes never change it.
- Optional desktop notifications when new mail arrives while the window is not focused.
- Press Ctrl+Shift+G (Cmd+Shift+G on macOS) to show or hide the window from anywhere.
- Zoom in or out from Settings; the level is remembered. Gmail's own theme controls light and dark mode.
- Can start minimized to the tray when launched at login.

> [!IMPORTANT]
> Google may block sign-in inside an embedded webview. If your account is not already signed in there, Gmail may be unable to open your inbox. This behavior is controlled by Google and the installed webview.

## Download

Download installers from the [Releases page](https://github.com/nooblk-98/gmaildock/releases). Builds from branch pushes and pull requests are available as artifacts on the [Actions page](https://github.com/nooblk-98/gmaildock/actions/workflows/build-release.yml).

## Run from source

Install [Rust](https://rustup.rs/), Node.js, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system. Then run:

```sh
git clone https://github.com/nooblk-98/gmaildock.git
cd gmaildock
npm ci
npm run dev
```

To build a local installer, run `npm run build`. The bundles are written under `src-tauri/target/release/bundle/`.

## Testing and contributing

GmailDock is currently tested hands-on on Windows. The release workflow builds macOS and Linux packages, but those platforms need more real-world testing. If you use macOS or Linux, please try a release and [open an issue](https://github.com/nooblk-98/gmaildock/issues) with your OS version, package type, steps to reproduce, and any error you see.

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, checks, and the release process. Issue templates ask for the details testers need to share. Contributions are welcome, especially for macOS and Linux compatibility, account sign-in behavior, accessibility, and the update experience. Please open an issue or pull request in the [repository](https://github.com/nooblk-98/gmaildock).

## Releases and updates

The [release workflow](.github/workflows/build-release.yml) builds Windows, macOS, and Linux packages. A version tag creates a signed release and `latest.json` for the in-app updater. The updater checks this repository's [latest release](https://github.com/nooblk-98/gmaildock/releases/latest).

Maintainers: run `npm run release -- X.Y.Z` to bump every version file and draft `.github/release-notes/vX.Y.Z.md` from commit messages, edit the notes, then commit and push a matching tag (or add `--push`). The notes are included in the release and in-app update dialog. The repository's `TAURI_SIGNING_PRIVATE_KEY` Actions secret is configured; keep the private key outside the repository. Forks need their own signing key, matching updater public key, and release endpoint.

## Privacy

GmailDock only wraps the Gmail website. It does not read, upload, or store your mail, and it has no servers of its own. The only data it saves locally is a small settings file (update, tray, notification, link, zoom, and startup preferences) and a log file used for troubleshooting. Your Google session lives in the system webview, as in a browser. The updater contacts GitHub to look for new releases.

## About this project

GmailDock is an independent, unofficial wrapper. It is not affiliated with or endorsed by Google. The icon adapts Google's [Gmail artwork](https://knowledge.workspace.google.com/admin/getting-started/brand-your-internal-communications-with-google-workspace). The account-menu adjustment depends on Google's page markup and may need changes when Gmail updates its interface.

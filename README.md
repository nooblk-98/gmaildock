<div align="center">
  <img src="assets/gmail.svg" width="80" height="80" alt="GmailDock icon">

  # GmailDock

  Fast, lightweight Gmail on your desktop.

  [![Build and release](https://github.com/nooblk-98/gmaildock/actions/workflows/build-release.yml/badge.svg)](https://github.com/nooblk-98/gmaildock/actions/workflows/build-release.yml)
  [![Latest release](https://img.shields.io/github/v/release/nooblk-98/gmaildock?label=release)](https://github.com/nooblk-98/gmaildock/releases)
</div>

GmailDock opens [mail.google.com](https://mail.google.com/) in your operating system's webview. Its small Rust and Tauri shell is designed for fast startup and low app overhead while keeping Gmail's own interface. There is no Gmail API setup or OAuth client ID. The current app window and installer are named **RustGmail**.

Inspired by [whatRust](https://github.com/karem505/whatRust).

## Features

- Gmail in a dedicated desktop window on Windows, macOS, and Linux.
- Reuses the system webview instead of bundling a browser engine, keeping the app lightweight. Windows uses WebView2. Gmail's own memory and CPU use still depends on the page and your account.
- Has an in-app Settings button for automatic update checks and the account-menu adjustment.
- Checks for updates in the background every 30 minutes when enabled. An install button appears only when an update is available.

> [!IMPORTANT]
> Google may block sign-in inside an embedded webview. If your account is not already signed in there, Gmail may be unable to open your inbox. This behavior is controlled by Google and the installed webview.

## Download

Installers will be available on the [Releases page](https://github.com/nooblk-98/gmaildock/releases) after the first tagged release. Builds from branch pushes and pull requests are available as artifacts on the [Actions page](https://github.com/nooblk-98/gmaildock/actions/workflows/build-release.yml).

## Run from source

Install [Rust](https://rustup.rs/), Node.js, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system. Then run:

```sh
git clone https://github.com/nooblk-98/gmaildock.git
cd gmaildock
npm ci
npm run dev
```

To build a local installer, run `npm run build`. The bundles are written under `src-tauri/target/release/bundle/`.

## Releases and updates

The [release workflow](.github/workflows/build-release.yml) builds Windows, macOS, and Linux packages. A version tag creates a signed release and `latest.json` for the in-app updater. The updater checks this repository's [latest release](https://github.com/nooblk-98/gmaildock/releases/latest); it cannot offer an update before a signed release exists.

Maintainers: update the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, then push a matching tag such as `v0.2.0`. The repository's `TAURI_SIGNING_PRIVATE_KEY` Actions secret is configured; keep the private key outside the repository. Forks need their own signing key, matching updater public key, and release endpoint.

## About this project

GmailDock is an independent, unofficial wrapper. It is not affiliated with or endorsed by Google. The icon adapts Google's [Gmail artwork](https://knowledge.workspace.google.com/admin/getting-started/brand-your-internal-communications-with-google-workspace). The account-menu adjustment depends on Google's page markup and may need changes when Gmail updates its interface.

# Contributing to GmailDock

Thanks for helping! GmailDock is tested hands-on on Windows only, so reports and fixes for macOS and Linux are especially valuable.

## Setup

Install [Rust](https://rustup.rs/), Node.js 22, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
git clone https://github.com/nooblk-98/gmaildock.git
cd gmaildock
npm ci
npm run dev
```

## Before opening a pull request

```sh
npm run check                                   # regression checks and the injected Gmail script
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks. The native side lives in `src-tauri/src` (`main.rs` wires it together, with `settings.rs`, `badge.rs`, `tray.rs`, `window.rs`, `shortcut.rs`, `updater.rs`, and `downloads.rs`). All settings are tray menu items built in `tray.rs`; there is no separate settings page. The small script injected into Gmail is `src-tauri/src/injected.js`, and it can only call `set_unread_count`.

## Signing and notarization

Release builds are unsigned unless the repository provides signing secrets. Windows installers then trigger SmartScreen, and macOS refuses to open an unsigned download until the quarantine flag is removed.

To sign:

- **macOS:** set `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID`.
- **Windows:** set `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` for the updater, and use a code-signing certificate with the Tauri `certificateThumbprint` settings.
- **Updater:** `TAURI_SIGNING_PRIVATE_KEY` is required for every release; `latest.json` is verified before the release is published.

## Cutting a release (maintainers)

```sh
npm run release -- 0.2.5          # bumps every version file and drafts release notes from commits
# edit .github/release-notes/v0.2.5.md, then:
git add -A && git commit -m "Prepare v0.2.5" && git tag v0.2.5 && git push origin HEAD v0.2.5
```

`npm run release -- 0.2.5 --push` does the commit, tag and push for you using the drafted notes. The script needs a clean working tree.

The tag push builds all platforms, creates one draft release, and publishes it when every build succeeds.

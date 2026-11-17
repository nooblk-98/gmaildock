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
npm run check                                   # regression checks (unread badge, count parsing, injected UI)
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks. The UI injected into Gmail lives in `src-tauri/src/injected.js`; the native side is split by concern in `src-tauri/src` (`main.rs` wires it together, with `settings.rs`, `badge.rs`, `tray.rs`, `window.rs`, `updater.rs`, and `downloads.rs`).

## Cutting a release (maintainers)

```sh
npm run release -- 0.2.5          # bumps every version file and drafts release notes from commits
# edit .github/release-notes/v0.2.5.md, then:
git add -A && git commit -m "Prepare v0.2.5" && git tag v0.2.5 && git push origin HEAD v0.2.5
```

`npm run release -- 0.2.5 --push` does the commit, tag and push for you using the drafted notes. The script needs a clean working tree.

The tag push builds all platforms, creates one draft release, and publishes it when every build succeeds.

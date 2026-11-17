<img src="assets/gmail.svg" width="64" height="64" alt="Gmail icon">

# RustGmail

RustGmail is a small Tauri 2 desktop shell that opens the real [Gmail website](https://mail.google.com/) in the operating system's webview. It is inspired by [whatRust](https://github.com/karem505/whatRust). There is no Gmail API integration and no OAuth client ID to configure.

This is an independent, unofficial wrapper. It is not affiliated with or endorsed by Google. The app icon uses Google's [Gmail artwork](https://knowledge.workspace.google.com/admin/getting-started/brand-your-internal-communications-with-google-workspace).

The app hides the “Manage your Google Account” entry in Gmail’s account menu. This depends on Google’s page markup and may need adjustment if Google changes that menu.

## Run

Install [Rust](https://rustup.rs/), the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), and Node.js. Then run:

```sh
npm install
npm run dev
```

To package the desktop app locally, run `npm run build`.

## Releases and updates

The [GitHub Actions workflow](.github/workflows/build-release.yml) builds Windows, macOS, and Linux installers on pull requests and pushes. Pushing a tag matching the version in `src-tauri/tauri.conf.json` (for example `v0.2.0`) creates a release with signed update artifacts and `latest.json`.

The updater endpoint points to `nooblk-98/gmaildock`. The repository has the `TAURI_SIGNING_PRIVATE_KEY` Actions secret for signed releases. The local signing key is stored outside the repository at `~/.t3/keys/gmaildock.key`; back it up securely. Its public key is embedded in the app.

For each release:

1. Update the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Push the matching `v` tag.

The app checks for releases quietly after startup and every 30 minutes. An **Update** button appears only when a newer release is available and offers to download, verify, and install it. Updates cannot work until the first signed GitHub release is published.

## Sign-in limitation

Google may reject account sign-in from an embedded webview. If that happens, the real Gmail page can be displayed but the app cannot access a new account in that webview. Google recommends using a system browser for sign-in. See [Google's desktop OAuth guidance](https://developers.google.com/identity/protocols/oauth2/native-app).

On Windows, Tauri uses the installed WebView2 runtime; it does not bundle a full browser engine. See [Tauri's webview documentation](https://v2.tauri.app/reference/webview-versions/).

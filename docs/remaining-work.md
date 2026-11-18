# Follow-up work after reliability fixes

OS signing and notarization are excluded by the maintainer's request. Keep the current app identifier to preserve existing sessions.

The following items from the audit remain open; this PR does not implement them:

- Isolated GmailDock account profiles and per-account badges. Google's existing account chooser remains available. This needs a storage migration and authentication tests on every OS.
- Sender/subject notifications and click-to-message. This requires additional access to Gmail content and a privacy documentation update.
- Save As dialogs. Download notifications and revealing the downloaded file are implemented; native destination selection needs platform tests before replacing the webview's default download flow.
- Full app translations and locale-independent Gmail account/apps selectors.
- Release notes inside the app. The tray continues to open the GitHub release notes.
- Uninstall cleanup for MSI and Linux packages. NSIS cleanup is implemented. Removing a macOS app to Trash cannot invoke cleanup; disable autostart before removal.
- OS notification denial detection: the current desktop notification plugin always reports Granted, so the UI explains how to check the OS settings.

Before merging, exercise Windows NSIS uninstall, Gmail sign-in (including country-domain redirects), downloads, shortcuts and Preferences keyboard access on Windows/macOS/Linux. Automated CI builds all six targets and executes native tests on Linux, Windows x64 and macOS ARM64; it does not replace an interactive sign-in check.

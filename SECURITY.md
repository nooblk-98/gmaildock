# Security policy

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Use GitHub's private vulnerability reporting:
https://github.com/nooblk-98/gmaildock/security/advisories/new

Include the GmailDock version (Settings, About), your platform, and the steps
needed to reproduce the issue.

## What GmailDock does with your data

GmailDock renders `mail.google.com` in your operating system's webview. It has
no servers, does not read or upload your mail, and does not store credentials.
The injected script reads only the Inbox unread count from the sidebar and
hides the Google Account and Google apps entries. Settings and logs stay on
your machine.

The updater contacts GitHub to check for new releases.

## Areas that matter for reports

- The script injected into Gmail (`src-tauri/src/injected.js`).
- Navigation and popup handling (`src-tauri/src/window.rs`).
- The updater and its signature verification.
- Local settings and log files.

## Supported versions

Only the latest release receives security fixes.

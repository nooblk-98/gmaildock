// Exercise the trusted preferences form without opening a desktop window.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';
const root = new URL('../ui/', import.meta.url);
const dom = new JSDOM(readFileSync(new URL('settings.html', root), 'utf8'), { runScripts: 'outside-only' });
const { window } = dom;
const settings = { startAtLogin: true, startMinimized: false, closeToTray: true, notifications: true,
  openExternalInBrowser: true, autoCheckUpdates: true, shortcut: 'Ctrl+G', zoom: 1, skippedVersion: '9.9.9' };
const calls = [];
window.__TAURI__ = { core: { invoke: async (command, args) => {
  calls.push({ command, args });
  if (command === 'get_preferences') return { settings, diagnostics: 'test', shortcutError: null };
  return args.settings;
} } };
window.eval(readFileSync(new URL('settings.js', root), 'utf8'));
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(window.document.querySelector('#startAtLogin').checked, true);
window.document.querySelector('#shortcut').value = 'Ctrl+Alt+X';
window.document.querySelector('#notifications').checked = false;
window.document.querySelector('form').dispatchEvent(new window.Event('submit', { cancelable: true }));
await new Promise((resolve) => setTimeout(resolve, 0));
const saved = calls.find((call) => call.command === 'save_preferences').args.settings;
assert.equal(saved.shortcut, 'Ctrl+Alt+X');
assert.equal(saved.notifications, false);
assert.equal(saved.skippedVersion, '9.9.9');
window.confirm = () => false;
const before = calls.length;
window.document.querySelector('#reset').click();
assert.equal(calls.length, before, 'cancelled reset must not invoke a command');
window.close();
console.log('preferences checks passed');

// Drives the native settings page against a mocked Tauri bridge.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { JSDOM } from 'jsdom';

const read = (name) =>
  readFileSync(fileURLToPath(new URL(`../frontend/${name}`, import.meta.url)), 'utf8');

const dom = new JSDOM(read('settings.html'), {
  url: 'http://tauri.localhost/settings.html',
  runScripts: 'outside-only',
  pretendToBeVisual: true,
});
const { window } = dom;

const defaults = {
  autoCheckUpdates: true,
  closeToTray: true,
  notifications: true,
  openExternalInBrowser: true,
  shortcut: 'CommandOrControl+Shift+G',
  skippedVersion: null,
  startAtLogin: true,
  startMinimized: false,
  zoom: 1,
};
let saved = { ...defaults };
let nextUpdate = null;
const calls = [];
const listeners = new Map();

window.__TAURI__ = {
  core: {
    invoke: async (command, args) => {
      calls.push({ command, args });
      switch (command) {
        case 'get_settings':
          return { ...saved };
        case 'save_settings':
          saved = { ...saved, ...args.settings };
          return { ...saved };
        case 'get_app_version':
          return '0.0.0-test';
        case 'get_update_state':
          return null;
        case 'check_for_updates':
          return nextUpdate;
        default:
          return undefined;
      }
    },
  },
  event: {
    listen: async (name, handler) => {
      listeners.set(name, handler);
      return () => {};
    },
  },
};

window.eval(read('settings.js'));
await new Promise((resolve) => setTimeout(resolve, 100));

const el = (id) => window.document.getElementById(id);
const settle = () => new Promise((resolve) => setTimeout(resolve, 50));

assert.equal(el('version').textContent, 'Version 0.0.0-test');
assert.equal(el('notifications').checked, true);
assert.equal(el('shortcut').value, 'CommandOrControl+Shift+G');
assert.equal(el('updateBox').hidden, true);

el('notifications').checked = false;
el('notifications').dispatchEvent(new window.Event('change'));
await settle();
assert.equal(saved.notifications, false);
assert(calls.some((call) => call.command === 'save_settings'));

el('shortcut').value = '  ';
el('shortcut').dispatchEvent(new window.Event('change'));
await settle();
assert.equal(saved.shortcut, null);

nextUpdate = { version: '9.9.9', notes: 'Big release.' };
el('checkNow').click();
await settle();
assert.equal(el('updateBox').hidden, false);
assert.equal(el('updateVersion').textContent, 'GmailDock 9.9.9');

el('skipButton').click();
await settle();
assert.equal(saved.skippedVersion, '9.9.9');
assert.equal(el('updateBox').hidden, true);
assert.equal(el('skippedNote').hidden, false);

listeners.get('update-progress')({ payload: { chunk: 512, total: 1024 } });
assert.equal(el('updateProgress').value, 50);

listeners.get('update-available')({ payload: { version: '9.9.10', notes: null } });
assert.equal(el('updateStatus').textContent, 'Version 9.9.10 is available.');

window.close();
console.log('settings checks passed');

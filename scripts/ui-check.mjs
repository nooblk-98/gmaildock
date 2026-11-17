// Mounts injected.js against a Gmail-shaped DOM to catch errors that only appear at runtime.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { JSDOM } from 'jsdom';

const source = readFileSync(
  fileURLToPath(new URL('../src-tauri/src/injected.js', import.meta.url)),
  'utf8',
);
const html = `<!doctype html>
<html>
  <body>
    <div role="navigation">
      <a href="https://mail.google.com/mail/u/0/#inbox" aria-label="Inbox 42 unread">Inbox</a>
    </div>
  </body>
</html>`;

const dom = new JSDOM(html, {
  url: 'https://mail.google.com/mail/u/0/#inbox',
  runScripts: 'outside-only',
  pretendToBeVisual: true,
});
const { window } = dom;
const invokes = [];

window.__TAURI__ = {
  core: {
    invoke: async (command, args) => {
      invokes.push({ command, args });
      switch (command) {
        case 'get_settings':
          return {
            autoCheckUpdates: true,
            closeToTray: true,
            notifications: true,
            openExternalInBrowser: true,
            startAtLogin: true,
            startMinimized: false,
            zoom: 1,
          };
        case 'get_app_version':
          return '0.0.0-test';
        case 'check_for_updates':
          return null;
        default:
          return undefined;
      }
    },
  },
};

window.eval(source);
await new Promise((resolve) => setTimeout(resolve, 100));

const host = window.document.querySelector('#gmaildock-controls');
assert(host, 'settings host was not mounted');
assert.equal(
  window.document.querySelectorAll('#gmaildock-controls').length,
  1,
  'settings host was mounted more than once',
);

const unread = invokes.find((call) => call.command === 'set_unread_count');
assert(unread, 'injected.js did not report the unread count');
assert.equal(unread.args.count, 42);

const panel = host.shadowRoot.querySelector('section');
assert.equal(panel.style.display, 'none', 'settings panel starts closed');

window.__gmaildockOpenSettings();
assert.equal(panel.style.display, 'block', 'settings panel did not open');

const labels = [...panel.querySelectorAll('label')].map((label) => label.textContent);
assert(
  labels.some((label) => label.includes('Open non-Google links in my browser')),
  'external link setting is missing from the settings panel',
);

window.close();
console.log('ui checks passed');

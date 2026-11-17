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
    <a href="https://myaccount.google.com/" aria-label="Manage your Google Account">Account</a>
    <a href="https://www.google.com/intl/en/about/products" aria-label="Google apps">Apps</a>
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
      return undefined;
    },
  },
};

window.eval(source);
await new Promise((resolve) => setTimeout(resolve, 100));

const unread = invokes.find((call) => call.command === 'set_unread_count');
assert(unread, 'injected.js did not report the unread count');
assert.equal(unread.args.count, 42);

const account = window.document.querySelector('a[href*="myaccount.google.com"]');
assert.equal(account.style.display, 'none', 'account link was not hidden');

const apps = window.document.querySelector('a[href*="about/products"]');
assert.equal(apps.style.display, 'none', 'Google apps link was not hidden');

assert.equal(window.document.querySelector('#gmaildock-controls'), null, 'no page UI should be injected');
assert.equal(window.__gmaildockOpenSettings, undefined, 'settings no longer live in the page');

window.close();
console.log('ui checks passed');

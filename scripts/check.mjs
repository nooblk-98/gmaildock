// Guards against regressions that have happened before (e.g. the unread badge being dropped).
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const injectedPath = fileURLToPath(new URL('../src-tauri/src/injected.js', import.meta.url));
const read = (name) => readFileSync(fileURLToPath(new URL(`../src-tauri/src/${name}`, import.meta.url)), 'utf8');

execFileSync(process.execPath, ['--check', injectedPath], { stdio: 'pipe' });

const injected = readFileSync(injectedPath, 'utf8');
const main = read('main.rs');
const badge = read('badge.rs');

const block = /\/\/ <unread-parse>([\s\S]*?)\/\/ <\/unread-parse>/.exec(injected);
assert(block, 'unread-parse block missing from injected.js');
const parseInboxCount = new Function(`${block[1]}; return parseInboxCount;`)();

const cases = [
  ['Inbox 60 unread', 60],
  ['Inbox 1,234 unread', 1234],
  ['Inbox', 0],
  ['', 0],
  [undefined, 0],
  ['Inbox 7', 7],
  ['البريد ١٬٢٣٤', 1234],
  ['Inbox १२३', 123],
  ['Inbox １２３', 123],
  ['Inbox 1\u202f234', 1234],
];
for (const [text, expected] of cases) {
  assert.equal(parseInboxCount(text), expected, `parseInboxCount(${JSON.stringify(text)})`);
}

assert(/fn set_unread_count/.test(badge), 'set_unread_count command missing from badge.rs');
assert(/generate_handler!\[[^\]]*set_unread_count/.test(main), 'set_unread_count not registered in invoke_handler');
assert(/set_unread_count/.test(injected), 'injected.js no longer reports the unread count');
assert(!/document\.title/.test(injected), 'badge must not read document.title (it follows the open mailbox)');

console.log('checks passed');

const capability = JSON.parse(readFileSync(new URL('../src-tauri/capabilities/gmail.json', import.meta.url)));
assert.deepEqual(capability.permissions, ['allow-set-unread-count']);
assert.deepEqual(capability.remote.urls, ['https://mail.google.com/*']);
assert.deepEqual(capability.windows, ['main']);
const build = readFileSync(new URL('../src-tauri/build.rs', import.meta.url), 'utf8');
assert(build.includes('set_unread_count'), 'app command ACL must be enabled');

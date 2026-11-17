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
];
for (const [text, expected] of cases) {
  assert.equal(parseInboxCount(text), expected, `parseInboxCount(${JSON.stringify(text)})`);
}

assert(/fn set_unread_count/.test(badge), 'set_unread_count command missing from badge.rs');
assert(/generate_handler!\[[^\]]*set_unread_count/.test(main), 'set_unread_count not registered in invoke_handler');
assert(/set_unread_count/.test(injected), 'injected.js no longer reports the unread count');
assert(!/document\.title/.test(injected), 'badge must not read document.title (it follows the open mailbox)');

console.log('checks passed');

// Guards against regressions that have happened before (e.g. the unread badge being dropped).
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';

const injected = readFileSync('src-tauri/src/injected.js', 'utf8');
const main = readFileSync('src-tauri/src/main.rs', 'utf8');

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

assert(/fn set_unread_count/.test(main), 'set_unread_count command missing from main.rs');
assert(/generate_handler!\[[^\]]*set_unread_count/.test(main), 'set_unread_count not registered in invoke_handler');
assert(/set_unread_count/.test(injected), 'injected.js no longer reports the unread count');
assert(!/document\.title/.test(injected), 'badge must not read document.title (it follows the open mailbox)');

console.log('checks passed');

#!/usr/bin/env node
// Usage: npm run release -- <version> [--push] [--dry-run]
//   npm run release -- 0.3.4            bump every version file and draft release notes
//   npm run release -- 0.3.4 --push     also commit, tag and push (starts the release workflow)
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const [version, ...flags] = process.argv.slice(2);
if (!/^\d+\.\d+\.\d+$/.test(version ?? '')) {
  console.error('Usage: npm run release -- <x.y.z> [--push] [--dry-run]');
  process.exit(1);
}
const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();

for (const flag of flags) {
  if (!['--push', '--dry-run'].includes(flag)) throw new Error(`Unknown flag: ${flag}`);
}
const dryRun = flags.includes('--dry-run');
if (!dryRun && git('status', '--porcelain')) throw new Error('Working tree is not clean. Commit or stash changes first.');
if (git('tag', '--list', `v${version}`)) throw new Error(`Tag v${version} already exists`);
if (flags.includes('--push')) {
  if (!['main', 'master'].includes(git('branch', '--show-current'))) throw new Error('Release from main or master');
  if (!/^https:\/\/github\.com\/nooblk-98\/gmaildock(?:\.git)?$|^git@github\.com:nooblk-98\/gmaildock(?:\.git)?$/.test(git('remote', 'get-url', 'origin'))) {
    throw new Error('Unexpected origin; review the repository before publishing');
  }
  if (git('ls-remote', '--tags', 'origin', `refs/tags/v${version}`).trim()) throw new Error('Remote tag already exists');
}
if (dryRun) {
  console.log(`Would bump ${JSON.parse(readFileSync('package.json', 'utf8')).version} -> ${version}, draft notes${flags.includes('--push') ? ', commit, tag and push' : ''}. No files changed.`);
  process.exit(0);
}

const edit = (file, change) => {
  const raw = readFileSync(file, 'utf8');
  const next = change(raw);
  if (next === raw) throw new Error(`${file}: version not updated`);
  writeFileSync(file, next);
};
const readJson = (file) => JSON.parse(readFileSync(file, 'utf8'));
const writeJson = (file, value) => writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);

const manifest = readJson('package.json');
const current = manifest.version;
manifest.version = version;
writeJson('package.json', manifest);

const lock = readJson('package-lock.json');
lock.version = version;
if (lock.packages?.['']) lock.packages[''].version = version;
writeJson('package-lock.json', lock);

edit('src-tauri/tauri.conf.json', (t) => t.replace(`"version": "${current}"`, `"version": "${version}"`));
edit('src-tauri/Cargo.toml', (t) => t.replace(`version = "${current}"`, `version = "${version}"`));
edit('src-tauri/Cargo.lock', (t) => t.replace(/(name = "gmaildock"\r?\nversion = )"[^"]+"/, `$1"${version}"`));

const notesFile = `.github/release-notes/v${version}.md`;
if (!existsSync(notesFile)) {
  let commits = '';
  try {
    const lastTag = git('describe', '--tags', '--abbrev=0');
    commits = git('log', `${lastTag}..HEAD`, '--no-merges', '--pretty=format:- %s');
  } catch {
    commits = git('log', '--no-merges', '-n', '10', '--pretty=format:- %s');
  }
  writeFileSync(
    notesFile,
    `GmailDock v${version}\n\n${commits || '- Maintenance release.'}\n\nWindows is the currently tested platform. macOS and Linux users are invited to test and report issues.\n`,
  );
  console.log(`Drafted ${notesFile} from commit messages. Edit it before releasing.`);
}

console.log(`Bumped ${current} -> ${version}`);
if (flags.includes('--push')) {
  git('add', '-A');
  git('commit', '-m', `Release v${version}`);
  git('tag', `v${version}`);
  git('push', 'origin', 'HEAD');
  git('push', 'origin', `v${version}`);
  console.log(`Pushed v${version}. The Build and release workflow will publish it.`);
} else {
  console.log('Review the notes, then rerun with --push, or commit, tag and push manually.');
}

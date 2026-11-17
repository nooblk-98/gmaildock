#!/usr/bin/env node
// Builds latest.json for the updater from the signed bundles attached to a draft release.
// Usage: node scripts/updater-manifest.mjs <tag> <output.json>
//
// The build jobs run in parallel, so letting each of them upload latest.json loses entries
// when two jobs rewrite the file at the same time. The publish job builds it once instead.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

const [tag, output] = process.argv.slice(2);
if (!tag || !output) {
  console.error('Usage: node scripts/updater-manifest.mjs <tag> <output.json>');
  process.exit(1);
}

// Bundle name pattern -> updater platform keys. The first key of each Windows entry prefers NSIS.
const PLATFORMS = [
  [/_x64-setup\.exe$/, ['windows-x86_64', 'windows-x86_64-nsis']],
  [/_x64_en-US\.msi$/, ['windows-x86_64-msi']],
  [/_arm64-setup\.exe$/, ['windows-aarch64', 'windows-aarch64-nsis']],
  [/_amd64\.AppImage$/, ['linux-x86_64', 'linux-x86_64-appimage']],
  [/_amd64\.deb$/, ['linux-x86_64-deb']],
  [/\.x86_64\.rpm$/, ['linux-x86_64-rpm']],
  [/_aarch64\.AppImage$/, ['linux-aarch64', 'linux-aarch64-appimage']],
  [/_arm64\.deb$/, ['linux-aarch64-deb']],
  [/\.aarch64\.rpm$/, ['linux-aarch64-rpm']],
  [/_x64\.app\.tar\.gz$/, ['darwin-x86_64', 'darwin-x86_64-app']],
  [/_aarch64\.app\.tar\.gz$/, ['darwin-aarch64', 'darwin-aarch64-app']],
];

const repo = process.env.GITHUB_REPOSITORY ?? 'nooblk-98/gmaildock';
const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });

const releases = JSON.parse(gh('api', `repos/${repo}/releases?per_page=30`));
const release = releases.find((candidate) => candidate.tag_name === tag);
if (!release) throw new Error(`No release found for ${tag}`);

const assets = new Map(release.assets.map((asset) => [asset.name, asset]));
const platforms = {};
for (const [name, asset] of assets) {
  const match = PLATFORMS.find(([pattern]) => pattern.test(name));
  if (!match) continue;
  const signature = assets.get(`${name}.sig`);
  if (!signature) throw new Error(`${name} has no .sig file`);
  const text = gh('api', '-H', 'Accept: application/octet-stream', signature.url).trim();
  for (const key of match[1]) platforms[key] = { signature: text, url: asset.url };
}

const notesFile = `.github/release-notes/${tag}.md`;
const manifest = {
  version: tag.replace(/^v/, ''),
  notes: readFileSync(notesFile, 'utf8').trim(),
  pub_date: new Date().toISOString(),
  platforms,
};
writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Wrote ${output} with ${Object.keys(platforms).length} platform entries`);

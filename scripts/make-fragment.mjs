// Build one family's manifest fragment for a release: {file, sha256, size}
// per binary, plus the tag/commit/run it was built from. Uploaded as a
// release asset next to the binaries; mStream's committed manifests are
// assembled from these fragments (its scripts/update-p2p-sidecar-manifest.mjs
// fetches them by tag).
//
// The fragment is also the family's COMPLETION MARKER: the publish job
// uploads binaries first and the fragment last, so "fragment present on the
// release" means "this family's asset set is complete and final".
//
// Usage:
//   node scripts/make-fragment.mjs \
//     --tag vX.Y.Z --built-from <sha> --run-url <url> \
//     --staging <dir> --out <file> \
//     <binary-name>...
//
// Exits non-zero on any missing/empty input — the caller treats that as
// "this run did not produce a full set".

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const args = process.argv.slice(2);
const opts = {};
const names = [];
for (let i = 0; i < args.length; i++) {
  if (args[i].startsWith('--')) { opts[args[i].slice(2)] = args[++i]; } else { names.push(args[i]); }
}

for (const k of ['tag', 'built-from', 'run-url', 'staging', 'out']) {
  if (!opts[k]) { console.error(`make-fragment: missing --${k}`); process.exit(1); }
}
if (names.length === 0) {
  console.error('make-fragment: no binary names given');
  process.exit(1);
}

const assets = {};
for (const name of names) {
  const src = path.join(opts.staging, name);
  let buf;
  try {
    buf = fs.readFileSync(src);
  } catch (err) {
    console.error(`make-fragment: cannot read ${src}: ${err.message}`);
    process.exit(1);
  }
  if (buf.length === 0) {
    console.error(`make-fragment: ${src} is empty`);
    process.exit(1);
  }
  assets[name] = {
    file: name,
    sha256: crypto.createHash('sha256').update(buf).digest('hex'),
    size: buf.length,
  };
}

const fragment = {
  family: 'p2p-sidecar',
  schema: 2,
  tag: opts.tag,
  builtFrom: opts['built-from'],
  workflowRun: opts['run-url'],
  assets,
};
fs.writeFileSync(opts.out, JSON.stringify(fragment, null, 2) + '\n');

console.log(`fragment for ${opts.tag}: ${names.length} assets`);
for (const [key, a] of Object.entries(assets)) {
  console.log(`  ${key}  sha256=${a.sha256.slice(0, 16)}…  ${a.size} bytes`);
}

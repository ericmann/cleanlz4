// Verifies docs/spec/ against docs/spec/SOURCES.json (size and SHA-256).
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, resolve } from 'node:path';

let root = '.';
const argv = process.argv.slice(2);
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === '--root') root = argv[++i];
}
const dir = join(resolve(root), 'docs', 'spec');
const failures = [];
let entries = [];
try {
  entries = JSON.parse(readFileSync(join(dir, 'SOURCES.json'), 'utf8'));
} catch (e) {
  failures.push(`SOURCES.json: ${e.message}`);
}
const known = new Set(['README.md', 'SOURCES.json']);
for (const e of entries) {
  known.add(e.file);
  let data;
  try {
    data = readFileSync(join(dir, e.file));
  } catch {
    failures.push(`${e.file}: missing`);
    continue;
  }
  if (data.length !== e.bytes) failures.push(`${e.file}: size ${data.length} != ${e.bytes}`);
  const h = createHash('sha256').update(data).digest('hex');
  if (h !== e.sha256) failures.push(`${e.file}: sha256 ${h} != ${e.sha256}`);
}
try {
  for (const f of readdirSync(dir)) {
    if (!known.has(f)) failures.push(`${f}: not listed in SOURCES.json`);
  }
} catch (e) {
  failures.push(`${dir}: ${e.message}`);
}
if (failures.length) {
  for (const f of failures) console.log(f);
  process.exit(1);
}
console.log(`spec sources: ok (${entries.length} files)`);

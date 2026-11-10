// Tests verify-spec-sources.mjs against a temporary copy of docs/spec/.
import { cpSync, mkdtempSync, readFileSync, writeFileSync, rmSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const here = dirname(fileURLToPath(import.meta.url));
const script = join(here, 'verify-spec-sources.mjs');
const repo = join(here, '..');
const tmp = mkdtempSync(join(tmpdir(), 'specsrc-'));
let bad = false;
function run() {
  return spawnSync('node', [script, '--root', tmp], { encoding: 'utf8' }).status;
}
function check(name, want) {
  const got = run();
  const ok = got === want;
  console.log(`${ok ? 'ok' : 'FAIL'}: ${name} (exit ${got})`);
  if (!ok) bad = true;
}
try {
  mkdirSync(join(tmp, 'docs'), { recursive: true });
  cpSync(join(repo, 'docs', 'spec'), join(tmp, 'docs', 'spec'), { recursive: true });
  check('clean', 0);
  const f = join(tmp, 'docs', 'spec', 'lz4_Block_format.md');
  const b = readFileSync(f);
  b[0] ^= 1;
  writeFileSync(f, b);
  check('tampered byte', 1);
  b[0] ^= 1;
  writeFileSync(f, b);
  writeFileSync(join(tmp, 'docs', 'spec', 'stray.md'), 'x');
  check('stray file', 1);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
process.exit(bad ? 1 : 0);

// Exercise the same owned-group cleanup used by the browser shutdown driver.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { ownedGroupCleanup } from '../verification-process-group.mjs';

const [python, ready, ...flags] = process.argv.slice(2);
const child = spawn(python, [fileURLToPath(new URL('./orphan-harness-child.py', import.meta.url)), ready, ...flags], {
  detached: true, stdio: 'ignore',
});
const cleanup = ownedGroupCleanup(child, 100);
for (const [sig, code] of [['SIGTERM', 143], ['SIGINT', 130]]) {
  process.on(sig, () => {
    process.exitCode = code;
    void cleanup().catch((error) => { console.error(error); process.exitCode = 1; });
  });
}
try {
  await once(child, 'exit');
  const info = JSON.parse(await readFile(ready, 'utf8'));
  assert.equal(info.group, child.pid, 'Fixture belongs to the dedicated group');
  process.kill(info.pid, 0);
  await cleanup();
  assert.throws(() => process.kill(info.pid, 0), { code: 'ESRCH' });
  await cleanup(); // Retired cleanup is idempotent.
  console.log(`PASS Node exited leader, ignore TERM=${flags.includes('--ignore-term')}: group reclaimed`);
} finally {
  await cleanup();
}

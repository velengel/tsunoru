#!/usr/bin/env node
// Regression for signals delivered to the verifier alone (as with a CI timeout).
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { access, rm } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const alive = (pid) => { try { process.kill(pid, 0); return true; } catch (e) { if (e.code === 'ESRCH') return false; throw e; } };
function descendants(parent) {
  const pairs = execFileSync('ps', ['-axo', 'pid=,ppid='], { encoding: 'utf8' }).trim().split('\n').map((line) => line.trim().split(/\s+/).map(Number));
  const found = new Set([parent]);
  for (let i = 0; i < pairs.length; i++) for (const [pid, ppid] of pairs) if (found.has(ppid)) found.add(pid);
  found.delete(parent);
  return [...found];
}
for (const stage of ['pending-browser', 'pending-process', 'asset', 'temp', 'socket', 'server', 'browser']) for (const signal of ['SIGTERM', 'SIGINT']) {
  const env = stage.startsWith('pending-') ? {
    ...process.env, TSUNORU_TEST_PLAYWRIGHT: process.env.PLAYWRIGHT_MODULE,
    PLAYWRIGHT_MODULE: resolve(root, stage === 'pending-process'
      ? 'scripts/fixtures/pending-calendar-browser-process.mjs' : 'scripts/fixtures/pending-calendar-browser.mjs'),
  } : process.env;
  const runner = spawn(process.execPath, ['scripts/verify-calendar-browser.mjs', `--shutdown-probe=${stage}`], {
    cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'],
  });
  const exit = once(runner, 'exit');
  let info, pids = [];
  try {
    info = await new Promise((done, fail) => {
      let output = '';
      const timeout = setTimeout(() => fail(new Error('shutdown probe readiness timed out')), 30000);
      runner.once('exit', () => { clearTimeout(timeout); fail(new Error('probe exited before readiness')); });
      let received = false;
      runner.stdout.on('data', (data) => {
        if (received) return;
        try {
          output += data;
          const match = output.match(/shutdown_probe_ready=(.+)\n/);
          // Pending-process waits until its real Playwright launch has started.
          const launchStarted = !stage.startsWith('pending-') || output.includes('pending_browser_launch=true');
          if (match && launchStarted) {
            const owned = JSON.parse(match[1]);
            assert.equal(typeof owned.temp, 'string', 'Probe publishes its disposable path');
            assert(owned.serverPid === null || Number.isInteger(owned.serverPid), 'Probe publishes its owned PID');
            received = true;
            clearTimeout(timeout);
            done(owned);
          }
        } catch (error) {
          received = true;
          clearTimeout(timeout);
          fail(error);
        }
      });
      runner.stderr.on('data', (data) => process.stderr.write(data));
    });
    pids = descendants(runner.pid);
    if (info.serverPid) assert(pids.includes(info.serverPid), 'Test identifies only the owned server tree');
    runner.kill(signal);
    let timer;
    const result = await Promise.race([exit, new Promise((_, fail) => { timer = setTimeout(() => fail(new Error('signal cleanup timed out')), 15000); })]).finally(() => clearTimeout(timer));
    assert.equal(result[0], stage.startsWith('pending-') ? 1 : signal === 'SIGINT' ? 130 : 143, 'Verifier exits only after signal cleanup');
    if (info.serverPid) assert(!alive(info.serverPid), `${signal}: owned server must stop`);
    assert(pids.every((pid) => !alive(pid)), `${signal}: owned browser descendants must stop`);
    await assert.rejects(access(info.temp), { code: 'ENOENT' }, `${signal}: disposable database directory must be removed`);
    console.log(`PASS ${stage} ${signal}: server, Chromium tree and temporary data removed`);
  } finally {
    // Also reclaim this test's owned resources when demonstrating the pre-fix failure.
    if (runner.exitCode === null && runner.signalCode === null) {
      // Include children created before a readiness/parse failure.
      pids = [...new Set([...pids, ...descendants(runner.pid)])];
      runner.kill('SIGKILL');
      await exit;
    }
    for (const pid of pids.reverse()) if (alive(pid)) { try { process.kill(pid, 'SIGKILL'); } catch (e) { if (e.code !== 'ESRCH') throw e; } }
    if (info) await rm(info.temp, { recursive: true, force: true });
  }
}

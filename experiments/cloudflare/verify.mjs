// Existing Miniflare installation can be supplied without downloading packages.
import assert from 'node:assert/strict';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const modulePath = process.env.MINIFLARE_MODULE;
assert.ok(modulePath, 'Set MINIFLARE_MODULE to an installed miniflare entry point');
const { Miniflare } = await import(pathToFileURL(modulePath));
const root = new URL('.', import.meta.url);
const wasmPath = new URL('target/wasm32-unknown-unknown/release/tsunoru_cloudflare_probe.wasm', root);
const temporary = await mkdtemp(join(tmpdir(), 'tsunoru-d1-probe-'));
let mf;
try {
  // Worker source intentionally absent before implementation: this must fail first.
  mf = new Miniflare({
    modules: true,
    scriptPath: new URL('worker.mjs', root).pathname,
    compatibilityDate: '2026-08-06',
    cf: false,
    d1Databases: ['DB'],
    d1Persist: temporary,
    modulesRules: [{ type: 'CompiledWasm', include: ['**/*.wasm'], fallthrough: true }],
  });
  const db = await mf.getD1Database('DB');
  const schema = await readFile(new URL('schema.sql', root), 'utf8');
  for (const statement of schema.replace(/^--.*$/gm, '').split(';').filter(s => s.trim())) {
    await db.prepare(statement).run();
  }
  const request = (path, body, origin = 'https://probe.example') => mf.dispatchFetch(`https://probe.example${path}`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', Origin: origin }, body: JSON.stringify(body),
  });
  assert.equal((await request('/create', { id: 'e1' }, 'https://other.example')).status, 403);
  assert.equal((await request('/create', { id: 'e1' })).status, 201);
  assert.equal((await request('/answer', { id: 'r1', event: 'e1', answer: 'yes' })).status, 201);
  assert.equal((await request('/answer', { id: 'r1', event: 'e1', answer: 'yes' })).status, 409);
  assert.equal((await request('/answer', { id: 'r2', event: 'missing', answer: 'yes' })).status, 409);
  assert.equal((await request('/create', { id: 'broken', fail: true })).status, 409);
  assert.equal(await db.prepare("SELECT count(*) AS n FROM events WHERE id='broken'").first('n'), 0);
  assert.equal(await db.prepare('SELECT count(*) AS n FROM responses').first('n'), 1);

  await db.prepare("INSERT INTO sessions VALUES('fixture-session',1)").run();
  await db.prepare("INSERT INTO series VALUES('s','e1')").run();
  const attempts = await Promise.all(['next-a', 'next-b'].map(id => request('/continue', { id, expected: 'e1' })));
  assert.deepEqual(attempts.map(r => r.status).sort(), [201, 409]);
  assert.equal(await db.prepare('SELECT count(*) AS n FROM continuations').first('n'), 1);
  assert.equal(await db.prepare('SELECT count(*) AS n FROM events').first('n'), 2);
  const tail = await db.prepare("SELECT tail FROM series WHERE id='s'").first('tail');
  await db.prepare("UPDATE sessions SET active=0").run();
  assert.equal((await request('/continue', { id: 'revoked', expected: tail })).status, 409);
  assert.equal(await db.prepare("SELECT count(*) AS n FROM events WHERE id='revoked'").first('n'), 0);
  // Counterexample: zero affected rows does NOT fail a D1 batch.
  await db.batch([
    db.prepare("INSERT INTO events VALUES('unguarded','synthetic')"),
    db.prepare("UPDATE series SET tail='unguarded' WHERE id='s' AND tail='stale'"),
  ]);
  assert.equal(await db.prepare("SELECT count(*) AS n FROM events WHERE id='unguarded'").first('n'), 1);
  await db.prepare("DELETE FROM events WHERE id='unguarded'").run();
  const began = performance.now();
  const probe = await (await mf.dispatchFetch('https://probe.example/probe')).json();
  const request_wall_ms = performance.now() - began;
  assert.equal(probe.domain, 7);
  assert.equal(probe.same, true);
  assert.equal(probe.different, true);
  const nativeWasm = await WebAssembly.instantiate(await readFile(wasmPath));
  assert.equal(probe.fingerprint, nativeWasm.instance.exports.argon2_probe(0) >>> 0);
  console.log(JSON.stringify({ result: 'PASS', cases: ['cross-origin', 'create-answer', 'duplicate', 'foreign-key', 'batch-rollback', 'concurrent-tail', 'session-revoked', 'zero-row-counterexample', 'shared-domain-wasm', 'argon2-repeat-wrong'], request_wall_ms, probe }));
} finally {
  if (mf) await mf.dispose();
  await rm(temporary, { recursive: true });
}

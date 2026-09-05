// Local-only diagnostic. Fixed sessions and unauthenticated probes must never be deployed.
import binary from './target/wasm32-unknown-unknown/release/tsunoru_cloudflare_probe.wasm';
const wasm = new WebAssembly.Instance(binary).exports;

export default {
  async fetch(request, env) {
    const path = new URL(request.url).pathname;
    if (path === '/probe' && request.method === 'GET') {
      const before = wasm.memory.buffer.byteLength;
      const start = performance.now();
      const fingerprint = wasm.argon2_probe(0) >>> 0;
      const elapsed = performance.now() - start;
      const same = fingerprint === (wasm.argon2_probe(0) >>> 0);
      const different = fingerprint !== (wasm.argon2_probe(1) >>> 0);
      return Response.json({ domain: wasm.domain_probe(), fingerprint, same, different, elapsed_wall_ms: elapsed,
        wasm_memory_before: before, wasm_memory_after: wasm.memory.buffer.byteLength });
    }
    if (request.method !== 'POST') return new Response('Not found', { status: 404 });
    if (request.headers.get('Origin') !== 'https://probe.example') return new Response('Forbidden', { status: 403 });
    const body = await request.json();
    const db = env.DB;
    let statements;
    if (path === '/create') {
      statements = [
        db.prepare('INSERT INTO events VALUES(?,?)').bind(body.id, 'synthetic event'),
        db.prepare('INSERT INTO candidates VALUES(?,?)').bind(`${body.id}-candidate`, body.fail ? 'missing' : body.id),
      ];
    } else if (path === '/answer') {
      statements = [db.prepare('INSERT INTO responses VALUES(?,?,?)').bind(body.id, body.event, body.answer)];
    } else if (path === '/continue') {
      // Assertion failure must abort the WHOLE batch, including later inserts.
      // A zero-row UPDATE alone is not a database error.
      statements = [
        db.prepare("INSERT INTO assertions SELECT CASE WHEN EXISTS(SELECT 1 FROM sessions WHERE id='fixture-session' AND active=1) AND EXISTS(SELECT 1 FROM series WHERE id='s' AND tail=?) THEN 1 ELSE 0 END").bind(body.expected),
        db.prepare('INSERT INTO events VALUES(?,?)').bind(body.id, 'synthetic continuation'),
        db.prepare("INSERT INTO continuations VALUES('s',?,?)").bind(body.expected, body.id),
        db.prepare("UPDATE series SET tail=? WHERE id='s' AND tail=?").bind(body.id, body.expected),
        db.prepare('DELETE FROM assertions'),
      ];
    } else return new Response('Not found', { status: 404 });
    try {
      await db.batch(statements);
      return Response.json({ saved: true }, { status: 201 });
    } catch {
      return Response.json({ saved: false }, { status: 409 });
    }
  },
};

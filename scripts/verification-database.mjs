// Private fixture schema and identity; no application or source DB is opened here.
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync } from 'node:fs';
import { Agent, request } from 'node:http';
import { dirname, join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

const migrations = join(dirname(fileURLToPath(import.meta.url)), '../migrations');

export function createIdentityDatabase(directory) {
  mkdirSync(join(directory, 'var'));
  const db = new DatabaseSync(join(directory, 'var/tsunoru.sqlite3'));
  const identity = { public_id: randomUUID(), name: 'Verification identity ' + randomUUID() };
  try {
    // SQLx 0.8 migration metadata; real-server checks reject a stale checksum.
    db.exec(`CREATE TABLE _sqlx_migrations (
      version BIGINT PRIMARY KEY, description TEXT NOT NULL,
      installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      success BOOLEAN NOT NULL, checksum BLOB NOT NULL, execution_time BIGINT NOT NULL
    ); PRAGMA foreign_keys=ON;`);
    for (const name of readdirSync(migrations).filter((name) => name.endsWith('.sql')).sort()) {
      const bytes = readFileSync(join(migrations, name));
      db.exec(bytes.toString('utf8'));
      db.prepare('INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (?, ?, 1, ?, 0)')
        .run(Number(name.split('_')[0]), name.replace(/^\d+_/, '').replace(/\.sql$/, '').replaceAll('_', ' '), createHash('sha384').update(bytes).digest());
    }
    db.prepare('INSERT INTO events (public_id, name, time_zone, organizer_capability_hash) VALUES (?, ?, ?, ?)')
      .run(identity.public_id, identity.name, 'Asia/Tokyo', createHash('sha256').update(randomUUID()).digest('hex'));
  } finally {
    db.close();
  }
  return identity;
}

export function checkDatabaseIdentity(origin, identity) {
  const body = JSON.stringify({ public_id: identity.public_id });
  return new Promise((done, fail) => {
    const req = request(new URL('/api/events/get', origin), {
      method: 'GET', headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
    }, (response) => {
      let text = '';
      response.setEncoding('utf8');
      response.on('error', fail);
      response.on('data', (chunk) => {
        text += chunk;
        if (text.length > 65536) req.destroy(new Error('database identity response too large'));
      });
      response.on('end', () => {
        try {
          const event = JSON.parse(text);
          assert(response.statusCode === 200 && event?.public_id === identity.public_id && event?.name === identity.name,
            'Owned database identity mismatch; refusing test writes');
          done();
        } catch (error) { fail(error); }
      });
    });
    req.on('error', fail);
    const deadline = setTimeout(() => req.destroy(new Error('database identity request timed out')), 2000);
    req.on('close', () => clearTimeout(deadline));
    req.end(body);
  });
}

class BoundAgent extends Agent {
  connectedOnce = false;
  createConnection(options, callback) {
    if (this.connectedOnce) {
      queueMicrotask(() => callback(new Error('Verified connection closed; refusing to reconnect a mutation')));
      return undefined;
    }
    this.connectedOnce = true;
    return super.createConnection(options, callback);
  }
}

function exchange(origin, agent, { path, method, headers = {}, body = Buffer.alloc(0), maxBytes = Infinity }) {
  return new Promise((done, fail) => {
    const outgoing = Object.fromEntries(Object.entries(headers).filter(([name]) =>
      !['host', 'connection', 'content-length', 'transfer-encoding', 'accept-encoding'].includes(name.toLowerCase())));
    const req = request(new URL(path, origin), {
      agent, method, headers: { ...outgoing, 'content-length': body.length, 'accept-encoding': 'identity' },
    }, (response) => {
      const chunks = [];
      let receivedBytes = 0;
      response.on('error', fail);
      response.on('data', (chunk) => {
        receivedBytes += chunk.length;
        if (receivedBytes > maxBytes) req.destroy(new Error('Database identity response too large'));
        else chunks.push(chunk);
      });
      response.on('end', () => {
        const headers = Object.fromEntries(Object.entries(response.headers)
          .filter(([name]) => !['connection', 'transfer-encoding'].includes(name))
          .map(([name, value]) => [name, Array.isArray(value) ? value.join(name === 'set-cookie' ? '\n' : ', ') : value]));
        done({ status: response.statusCode, headers, body: Buffer.concat(chunks) });
      });
    });
    req.on('error', fail);
    const timer = setTimeout(() => req.destroy(new Error('Verified request timed out')), 10000);
    req.on('close', () => clearTimeout(timer));
    req.end(body);
  });
}

export async function verifiedMutation(origin, identity, mutation) {
  assert.equal(new URL(mutation.path, origin).origin, new URL(origin).origin, 'Mutation must stay on the owned origin');
  const agent = new BoundAgent({ keepAlive: true, maxSockets: 1 });
  try {
    const found = await exchange(origin, agent, {
      path: '/api/events/get', method: 'GET', headers: { 'content-type': 'application/json' },
      body: Buffer.from(JSON.stringify({ public_id: identity.public_id })),
      maxBytes: 65536,
    });
    const event = JSON.parse(found.body.toString());
    assert(found.status === 200 && event?.public_id === identity.public_id && event?.name === identity.name,
      'Owned database identity mismatch; refusing test writes');
    return await exchange(origin, agent, mutation);
  } finally {
    agent.destroy();
  }
}

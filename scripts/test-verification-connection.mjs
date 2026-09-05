// A verified listener can disappear before a mutation without giving writes to its replacement.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import * as database from './verification-database.mjs';

const identity = { public_id: 'test-identity', name: 'owned database' };
let foreignWrites = 0;
const foreign = createServer((req, res) => {
  if (req.method === 'POST') foreignWrites++;
  res.end('{}');
});
const owned = createServer((req, res) => {
  req.resume();
  owned.close();
  foreign.listen(ownedPort, '127.0.0.1', () => {
    res.writeHead(200, { 'content-type': 'application/json', connection: 'close' });
    res.end(JSON.stringify(identity));
  });
});
let ownedPort;
try {
  owned.listen(0, '127.0.0.1');
  await once(owned, 'listening');
  ownedPort = owned.address().port;
  const origin = `http://127.0.0.1:${ownedPort}`;
  let failure;
  try { await database.verifiedMutation(origin, identity, { path: '/api/events/create', method: 'POST', headers: {}, body: Buffer.from('{}') }); }
  catch (error) { failure = error; }
  assert.equal(foreignWrites, 0, 'Replacement listener must receive no mutation');
  assert(failure, 'Closed verified connection must fail instead of reconnecting');
  console.log('PASS Node: listener handover receives zero writes after a valid identity response');
} finally {
  owned.closeAllConnections(); foreign.closeAllConnections();
  owned.close(); foreign.close();
}

let verifiedSocket, writes = 0, mismatch = false;
const healthy = createServer((req, res) => {
  req.resume();
  if (req.method === 'GET') {
    verifiedSocket = req.socket;
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify(mismatch ? { ...identity, name: 'another database' } : identity));
  } else {
    assert.equal(req.socket, verifiedSocket, 'Mutation uses the identity response socket');
    assert.equal(req.headers.cookie, 'existing=kept');
    writes++;
    res.writeHead(201, { 'set-cookie': ['first=1; Path=/; HttpOnly', 'second=2; Path=/'], 'content-type': 'application/octet-stream' });
    res.end(Buffer.from([0, 127, 255]));
  }
});
try {
  healthy.listen(0, '127.0.0.1');
  await once(healthy, 'listening');
  const origin = `http://127.0.0.1:${healthy.address().port}`;
  const mutation = { path: '/api/events/create', method: 'POST', headers: { cookie: 'existing=kept' }, body: Buffer.from('{}') };
  const response = await database.verifiedMutation(origin, identity, mutation);
  assert.equal(response.status, 201);
  assert.deepEqual(response.body, Buffer.from([0, 127, 255]));
  assert.equal(response.headers['set-cookie'], 'first=1; Path=/; HttpOnly\nsecond=2; Path=/');
  mismatch = true;
  await assert.rejects(database.verifiedMutation(origin, identity, mutation), /identity mismatch/);
  assert.equal(writes, 1, 'Identity mismatch sends no second mutation');
  console.log('PASS Node: one socket, status/body/cookies preserved, wrong identity rejected');
} finally {
  healthy.closeAllConnections(); healthy.close();
}

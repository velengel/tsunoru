import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  APP_ORIGIN, ORGANIZER_TOKEN, assertPublicPayload, capability, request, rowCounts,
} from "./fixtures.mjs";

const event = (id) => ({
  id,
  name: "週末の予定",
  organizer_note: "参加できる日時を教えてください。",
  time_zone: "Asia/Tokyo",
  organizer_capability: ORGANIZER_TOKEN,
  candidates: [
    { id: "c1", local_date: "2026-09-10", local_time: "10:00" },
    { id: "c2", local_date: "2026-09-11", local_time: "18:00" },
  ],
});
const answer = (respondent_name = "同じ名前") => ({
  respondent_name,
  availabilities: [
    { candidate_id: "c1", availability: "available" },
    { candidate_id: "c2", availability: "maybe" },
  ],
});
const responsePath = (id) => `/api/events/${id}/responses`;
const responseHeaders = (token) => ({ "x-response-capability": token });
const organizerHeaders = (token = ORGANIZER_TOKEN) => ({ "x-organizer-capability": token });
const canonical = (choices) => [...choices].sort((left, right) => left.candidate_id.localeCompare(right.candidate_id));

async function createEvent(fixture, id, extra = {}) {
  const created = await request(fixture, "/api/events", {
    method: "POST", payload: event(id), status: 201, ...extra,
  });
  assert.deepEqual(created.json, { id, name: event(id).name });
  assertPublicPayload(created.json);
}

async function submit(fixture, id, token, payload, status) {
  const response = await request(fixture, responsePath(id), {
    method: "POST", headers: responseHeaders(token), payload, status,
  });
  assertPublicPayload(response.json);
  if (response.status < 300) {
    assert.equal(response.json.event_id, id);
    assert.equal(typeof response.json.response_id, "string");
    assert(response.json.response_id.length > 0);
  }
  return response;
}

export async function verifyStagingApi(pool) {
  // First prove that unauthenticated API calls are rejected before DB access.
  const closed = await pool.create({ database: false });
  await request(closed, "/api/events/missing", { bearer: null, status: 401 });
  await request(closed, "/health", { bearer: null, status: 200 });
  await request(closed, "/api/events", {
    method: "POST", rawBody: "not JSON", bearer: capability(999), status: 401,
  });
  for (const origin of ["https://attacker.example", "null", `${APP_ORIGIN}:444`, `${APP_ORIGIN}, https://attacker.example`]) {
    await request(closed, "/api/events/missing", { origin, status: 403 });
  }
  await closed.close();
  console.log("PASS authentication and Origin reject before database access");

  const invalidConfigurations = [
    { omit: ["STAGING_API_TOKEN"] },
    { bindings: { STAGING_API_TOKEN: "" } },
    { bindings: { STAGING_API_TOKEN: "short" } },
    { bindings: { STAGING_API_TOKEN: "z".repeat(64) } },
    { omit: ["APP_ORIGIN"] },
    { bindings: { APP_ORIGIN: "null" } },
    { bindings: { APP_ORIGIN: `${APP_ORIGIN}/unexpected-path` } },
    { bindings: { APP_ORIGIN: "https://user:password@tsunoru-staging.example.test" } },
    { bindings: { APP_ORIGIN: "ftp://tsunoru-staging.example.test" } },
  ];
  for (const configuration of invalidConfigurations) {
    const candidate = await pool.create({ ...configuration, database: false });
    await request(candidate, "/api/events/missing", { status: 503 });
    await candidate.close();
  }
  console.log("PASS absent or malformed staging configuration fails closed");

  const fixture = await pool.create();
  const { db } = fixture;
  await createEvent(fixture, "event-one", { origin: APP_ORIGIN, contentType: "application/json; charset=utf-8" });
  await createEvent(fixture, "event-two");
  const publicEvent = await request(fixture, "/api/events/event-one", { status: 200 });
  assert.equal(publicEvent.json.id, "event-one");
  assert.equal(publicEvent.json.name, event("event-one").name);
  assert.equal(publicEvent.json.organizer_note, event("event-one").organizer_note);
  assert.equal(publicEvent.json.time_zone, "Asia/Tokyo");
  assert.deepEqual(publicEvent.json.candidates, event("event-one").candidates);
  assertPublicPayload(publicEvent.json);
  const storedEvent = await db.prepare("SELECT organizer_capability_hash FROM events WHERE id = ?1").bind("event-one").first();
  assert.equal(storedEvent.organizer_capability_hash, createHash("sha256").update(ORGANIZER_TOKEN).digest("hex"));
  const beforeDuplicate = await rowCounts(db);
  const replayedEvent = await request(fixture, "/api/events", { method: "POST", payload: event("event-one"), status: 200 });
  assert.deepEqual(replayedEvent.json, { id: "event-one", name: event("event-one").name });
  const normalizedRetry = {
    ...event("event-one"), name: `  ${event("event-one").name}\n`,
    organizer_note: `\n${event("event-one").organizer_note} `, time_zone: " Asia/Tokyo ",
    candidates: [...event("event-one").candidates].reverse().map((candidate) => ({
      ...candidate, local_date: ` ${candidate.local_date} `, local_time: ` ${candidate.local_time} `,
    })),
  };
  await request(fixture, "/api/events", { method: "POST", payload: normalizedRetry, status: 200 });
  const conflictingEvents = [
    { ...event("event-one"), organizer_capability: capability(999) },
    { ...event("event-one"), name: "別の予定" },
    { ...event("event-one"), organizer_note: "変更後のひとこと" },
    { ...event("event-one"), time_zone: "UTC" },
    { ...event("event-one"), candidates: [{ ...event("e").candidates[0], id: "injected" }] },
    { ...event("event-one"), candidates: event("e").candidates.map((candidate) => ({ ...candidate, local_time: "11:00" })) },
  ];
  for (const payload of conflictingEvents) await request(fixture, "/api/events", { method: "POST", payload, status: 409 });
  assert.deepEqual(await rowCounts(db), beforeDuplicate);
  await request(fixture, "/api/events/unknown", { status: 404 });
  console.log("PASS event creation: structured metadata, normalized authorized retries and immutable conflicts");

  const emptyNoteEvent = { ...event("empty-note"), organizer_note: " \n " };
  await request(fixture, "/api/events", { method: "POST", payload: emptyNoteEvent, status: 201 });
  await request(fixture, "/api/events", { method: "POST", payload: { ...emptyNoteEvent, organizer_note: null }, status: 200 });
  delete emptyNoteEvent.organizer_note;
  await request(fixture, "/api/events", { method: "POST", payload: emptyNoteEvent, status: 200 });
  assert.equal((await request(fixture, "/api/events/empty-note", { status: 200 })).json.organizer_note, null);

  const validDatetimeEvents = [
    { ...event("leap-day"), name: "名".repeat(100), organizer_note: "文".repeat(500), time_zone: "UTC", candidates: [{ id: "leap", local_date: "2028-02-29", local_time: "23:59" }] },
    { ...event("after-spring-gap"), organizer_note: "複数行\nでも入力できる", time_zone: "America/New_York", candidates: [{ id: "spring", local_date: "2026-03-08", local_time: "03:30" }] },
    { ...event("after-autumn-fold"), time_zone: "America/New_York", candidates: [{ id: "autumn", local_date: "2026-11-01", local_time: "02:30" }] },
  ];
  for (const payload of validDatetimeEvents) {
    await request(fixture, "/api/events", { method: "POST", payload, status: 201 });
    const actual = await request(fixture, `/api/events/${payload.id}`, { status: 200 });
    assert.equal(actual.json.organizer_note, payload.organizer_note);
    assert.equal(actual.json.time_zone, payload.time_zone);
    assert.deepEqual(actual.json.candidates, payload.candidates);
  }
  console.log("PASS optional note normalization, exact input limits, leap day and valid DST-adjacent times");

  const beforeInvalid = await rowCounts(db);
  for (const rawBody of ["{", "null", "[]", "{}", '{"id":42}']) {
    await request(fixture, "/api/events", { method: "POST", rawBody, status: 400 });
  }
  for (const contentType of ["text/plain", "application/jsonp", null]) {
    await request(fixture, "/api/events", { method: "POST", payload: event("rejected-type"), contentType, status: 415 });
  }
  const largeJson = JSON.stringify({ ...event("large-event"), name: "x".repeat(70_000) });
  const streamedBody = new ReadableStream({
    start(controller) {
      const bytes = new TextEncoder().encode(largeJson);
      for (let offset = 0; offset < bytes.length; offset += 1_024) controller.enqueue(bytes.slice(offset, offset + 1_024));
      controller.close();
    },
  });
  await request(fixture, "/api/events", { method: "POST", rawBody: streamedBody, status: 413 });
  const malformedEvents = [
    { ...event("invalid-empty"), name: " " },
    { ...event("invalid-capability"), organizer_capability: "short" },
    { ...event("invalid-candidates"), candidates: [] },
    { ...event("duplicate-candidates"), candidates: [event("e").candidates[0], event("e").candidates[0]] },
    { ...event("duplicate-ids"), candidates: event("e").candidates.map((candidate) => ({ ...candidate, id: "same" })) },
    { ...event("duplicate-datetimes"), candidates: [event("e").candidates[0], { ...event("e").candidates[0], id: "other" }] },
    { ...event("long-name"), name: "名".repeat(101) },
    { ...event("long-note"), organizer_note: "文".repeat(501) },
    { ...event("unknown-timezone"), time_zone: "Mars/Base" },
    { ...event("offset-not-timezone"), time_zone: "+09:00" },
    { ...event("empty-timezone"), time_zone: " " },
    { ...event("many-candidates"), candidates: Array.from({ length: 21 }, (_, i) => ({ id: `c${i}`, local_date: "2026-09-10", local_time: `${i.toString().padStart(2, "0")}:00` })) },
    ...["2026-02-29", "2026-04-31", "2026-13-01", "0000-01-01", "2026-9-10", "2026-09-10T00:00:00Z"].map((local_date) => ({ ...event("bad-date"), candidates: [{ ...event("e").candidates[0], local_date }] })),
    ...["24:00", "12:60", "9:00", "10:00:00"].map((local_time) => ({ ...event("bad-time"), candidates: [{ ...event("e").candidates[0], local_time }] })),
    { ...event("dst-gap"), time_zone: "America/New_York", candidates: [{ id: "gap", local_date: "2026-03-08", local_time: "02:30" }] },
    { ...event("dst-fold"), time_zone: "America/New_York", candidates: [{ id: "fold", local_date: "2026-11-01", local_time: "01:30" }] },
  ];
  for (const payload of malformedEvents) {
    await request(fixture, "/api/events", { method: "POST", payload, status: 400 });
  }
  assert.deepEqual(await rowCounts(db), beforeInvalid);
  console.log("PASS invalid dates, times, time zones and DST gaps/folds reject without writes");

  const beforeParallelEvents = await rowCounts(db);
  const repeatedCreates = await Promise.all(Array.from({ length: 6 }, () => request(fixture, "/api/events", {
    method: "POST", payload: event("parallel-event"),
  })));
  assert.deepEqual(repeatedCreates.map((result) => result.status).sort(), [200, 200, 200, 200, 200, 201]);
  const competingCreates = [event("competing-event"), {
    ...event("competing-event"), name: "競合する予定", organizer_note: "別の内容",
    candidates: event("e").candidates.map((candidate) => ({ ...candidate, local_time: "12:00" })),
  }];
  const createRace = await Promise.all(competingCreates.map((payload) => request(fixture, "/api/events", { method: "POST", payload })));
  assert.deepEqual(createRace.map((result) => result.status).sort(), [201, 409]);
  const winningCreate = competingCreates[createRace.findIndex((result) => result.status === 201)];
  const savedCreate = await request(fixture, "/api/events/competing-event", { status: 200 });
  assert.equal(savedCreate.json.name, winningCreate.name);
  assert.equal(savedCreate.json.organizer_note, winningCreate.organizer_note);
  assert.deepEqual(savedCreate.json.candidates, winningCreate.candidates);
  assert.deepEqual(await rowCounts(db), { ...beforeParallelEvents, events: beforeParallelEvents.events + 2, candidates: beforeParallelEvents.candidates + 4 });
  console.log("PASS concurrent event retries save one complete event and preserve the winning payload");

  const beforeResponses = await rowCounts(db);
  await request(fixture, responsePath("event-one"), { method: "POST", payload: answer(), status: 403 });
  await submit(fixture, "event-one", "short", answer(), 403);
  await submit(fixture, "unknown", capability(10), answer(), 404);
  const invalidAnswers = [
    { ...answer(), respondent_name: " " },
    { ...answer(), respondent_name: "名".repeat(101) },
    { ...answer(), availabilities: [] },
    { ...answer(), availabilities: [answer().availabilities[0]] },
    { ...answer(), availabilities: [...answer().availabilities, { candidate_id: "other", availability: "maybe" }] },
    { ...answer(), availabilities: [answer().availabilities[0], answer().availabilities[0]] },
    { ...answer(), availabilities: [answer().availabilities[0], { candidate_id: "other", availability: "maybe" }] },
    { ...answer(), availabilities: [{ candidate_id: "c1", availability: "unknown" }, answer().availabilities[1]] },
    { ...answer(), response_id: "guessed-response-id" },
  ];
  for (const payload of invalidAnswers) await submit(fixture, "event-one", capability(11), payload, 400);
  for (const rawBody of ["{", "null", "[]", "{}"]) {
    await request(fixture, responsePath("event-one"), { method: "POST", rawBody, headers: responseHeaders(capability(11)), status: 400 });
  }
  assert.deepEqual(await rowCounts(db), beforeResponses);
  console.log("PASS rejected response capabilities and incomplete candidate sets make no writes");

  const first = await submit(fixture, "event-one", capability(20), answer(), 201);
  const second = await submit(fixture, "event-one", capability(21), answer(), 201);
  assert.notEqual(first.json.response_id, second.json.response_id);
  const storedResponse = await db.prepare("SELECT response_capability_hash FROM responses WHERE id = ?1").bind(first.json.response_id).first();
  assert.equal(storedResponse.response_capability_hash, createHash("sha256").update(capability(20)).digest("hex"));
  const beforeReplay = await rowCounts(db);
  const retry = await submit(fixture, "event-one", capability(20), {
    ...answer(), availabilities: [...answer().availabilities].reverse(),
  }, 200);
  assert.deepEqual(retry.json, first.json);
  await submit(fixture, "event-one", capability(20), answer("別の名前"), 409);
  await submit(fixture, "event-one", capability(20), {
    ...answer(), availabilities: answer().availabilities.map((choice) => ({ ...choice, availability: "unavailable" })),
  }, 409);
  await submit(fixture, "event-two", capability(20), answer(), 409);
  assert.deepEqual(await rowCounts(db), beforeReplay);
  console.log("PASS per-response ownership, independent identical names and immutable idempotent retries");

  await request(fixture, responsePath("event-one"), { status: 403 });
  await request(fixture, responsePath("event-one"), { headers: organizerHeaders(capability(99)), status: 403 });
  const list = await request(fixture, responsePath("event-one"), { headers: organizerHeaders(), status: 200 });
  assertPublicPayload(list.json);
  assert.equal(list.json.responses.length, 2);
  for (const response of list.json.responses) {
    assert.equal(response.respondent_name, answer().respondent_name);
    assert.deepEqual(canonical(response.availabilities), canonical(answer().availabilities));
  }
  console.log("PASS organizer-only response projection excludes capabilities and hashes");

  const beforeParallel = await rowCounts(db);
  const same = await Promise.all(Array.from({ length: 6 }, () => submit(fixture, "event-one", capability(30), answer("再送"))));
  assert.deepEqual(same.map((result) => result.status).sort(), [200, 200, 200, 200, 200, 201]);
  assert.equal(new Set(same.map((result) => result.json.response_id)).size, 1);
  const competing = [
    answer("競合A"),
    { respondent_name: "競合B", availabilities: answer().availabilities.map((choice) => ({ ...choice, availability: "unavailable" })) },
  ];
  const raced = await Promise.all(competing.map((payload) => submit(fixture, "event-one", capability(31), payload)));
  assert.deepEqual(raced.map((result) => result.status).sort(), [201, 409]);
  const winnerIndex = raced.findIndex((result) => result.status === 201);
  const current = await request(fixture, responsePath("event-one"), { headers: organizerHeaders(), status: 200 });
  const storedWinner = current.json.responses.find((response) => response.response_id === raced[winnerIndex].json.response_id);
  assert.equal(storedWinner.respondent_name, competing[winnerIndex].respondent_name);
  assert.deepEqual(canonical(storedWinner.availabilities), canonical(competing[winnerIndex].availabilities));
  assert.deepEqual(await rowCounts(db), {
    ...beforeParallel, responses: beforeParallel.responses + 2, answers: beforeParallel.answers + 4,
  });
  console.log("PASS concurrent retries create one complete response and never mix conflicting payloads");

  const beforeRollback = await rowCounts(db);
  await db.prepare("CREATE TRIGGER reject_second_answer BEFORE INSERT ON answers WHEN NEW.candidate_id = 'c2' BEGIN SELECT RAISE(ABORT, 'sensitive-sql-probe'); END").run();
  try {
    const rejected = await submit(fixture, "event-one", capability(40), answer("rollback"), 500);
    assert.deepEqual(rejected.json, { error: { code: "internal_error" } });
    assert(!rejected.text.includes("sensitive-sql-probe"));
    assert.deepEqual(await rowCounts(db), beforeRollback);
  } finally {
    await db.prepare("DROP TRIGGER reject_second_answer").run();
  }
  await db.prepare("CREATE TRIGGER reject_second_candidate BEFORE INSERT ON candidates WHEN NEW.event_id = 'rollback-event' AND NEW.id = 'c2' BEGIN SELECT RAISE(ABORT, 'sensitive-event-probe'); END").run();
  try {
    const rejected = await request(fixture, "/api/events", { method: "POST", payload: event("rollback-event"), status: 500 });
    assert.deepEqual(rejected.json, { error: { code: "internal_error" } });
    assert.deepEqual(await rowCounts(db), beforeRollback);
  } finally {
    await db.prepare("DROP TRIGGER reject_second_candidate").run();
  }
  console.log("PASS failed D1 batches roll back all rows and hide internal diagnostics");

  const unconfiguredDatabase = await pool.create({ database: false });
  const failure = await request(unconfiguredDatabase, "/api/events/missing", { status: 500 });
  assert.deepEqual(failure.json, { error: { code: "internal_error" } });
  await unconfiguredDatabase.close();
  await fixture.close();
}

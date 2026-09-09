import { randomBytes as cryptoRandomBytes } from "node:crypto";

const DEFAULT_URL = "https://staging.tsunoru.velengel.com";
let activeCleanup = null;

export async function runSmoke({ baseUrl, token, fetchImpl = fetch, randomBytes = cryptoRandomBytes } = {}) {
  const origin = new URL(baseUrl || DEFAULT_URL).origin;
  const secret = requireToken(token);
  const organizerCapability = randomBytes(32).toString("hex");
  const responseCapability = randomBytes(32).toString("hex");
  const eventId = `smoke-${Date.now()}-${randomBytes(8).toString("hex")}`;
  const session = await call(fetchImpl, origin, "/api/staging/session", {
    method: "POST", origin, body: { access_code: secret }, expected: 200,
  });
  const cookie = session.headers.get("set-cookie")?.split(";", 1)[0];
  if (!cookie) throw new Error("session login did not issue a cookie");
  await call(fetchImpl, origin, "/api/staging/session", { headers: { cookie }, expected: 200 });
  await call(fetchImpl, origin, "/health", { expected: 200, checkHealth: true });

  let created = false;
  let deleted = false;
  let responseCount = 0;
  try {
    const event = {
      id: eventId, name: "staging smoke event", organizer_note: "temporary verification data",
      time_zone: "Asia/Tokyo", organizer_capability: organizerCapability,
      candidates: [{ id: "c1", local_date: "2030-01-10", local_time: "10:00" }],
    };
    // The request may commit before its response is lost; always attempt cleanup.
    created = true;
    activeCleanup = async () => {
      if (!created || deleted) return;
      try {
        const result = await call(fetchImpl, origin, `/api/events/${eventId}`, {
          method: "DELETE", headers: { cookie, "x-organizer-capability": organizerCapability }, origin, expected: 200,
        });
        deleted = result.json.deleted === true || result.json.deleted === false;
      } catch { /* the normal finally path reports cleanup failures */ }
    };
    const createdResponse = await call(fetchImpl, origin, "/api/events", {
      method: "POST", headers: { cookie }, origin, body: event, expected: 201,
    });
    if (createdResponse.json.id !== eventId) throw new Error("event create returned an unexpected id");
    const publicEvent = await call(fetchImpl, origin, `/api/events/${eventId}`, { headers: { cookie }, expected: 200 });
    if (publicEvent.json.id !== eventId || publicEvent.json.candidates?.length !== 1) throw new Error("public event projection mismatch");
    await call(fetchImpl, origin, `/api/events/${eventId}/responses`, {
      method: "POST", headers: { cookie, "x-response-capability": responseCapability }, origin,
      body: { respondent_name: "staging smoke respondent", availabilities: [{ candidate_id: "c1", availability: "available" }] }, expected: 201,
    });
    const responses = await call(fetchImpl, origin, `/api/events/${eventId}/responses`, {
      headers: { cookie, "x-organizer-capability": organizerCapability }, expected: 200,
    });
    if (responses.json.responses?.length !== 1) throw new Error("response projection mismatch");
    responseCount = responses.json.responses.length;
  } finally {
    if (created) {
      const deletedResponse = await call(fetchImpl, origin, `/api/events/${eventId}`, {
        method: "DELETE", headers: { cookie, "x-organizer-capability": organizerCapability }, origin, expected: [200, 404],
      });
      if (deletedResponse.json.deleted !== true && deletedResponse.json.deleted !== false) throw new Error("cleanup returned an invalid result");
      deleted = true;
    }
    activeCleanup = null;
  }
  return { eventId, deleted, responseCount };
}

function requireToken(token) {
  if (typeof token !== "string" || !/^[0-9a-f]{64}$/i.test(token)) throw new Error("TSUNORU_STAGING_TOKEN must be a 64-character hexadecimal value");
  return token;
}

async function call(fetchImpl, origin, path, { method = "GET", headers = {}, origin: requestOrigin, body, expected, checkHealth = false } = {}) {
  const requestHeaders = new Headers(headers);
  if (requestOrigin) requestHeaders.set("origin", requestOrigin);
  if (body !== undefined) requestHeaders.set("content-type", "application/json");
  const response = await fetchImpl(`${origin}${path}`, { method, headers: requestHeaders, ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
  const text = await response.text();
  let json;
  try { json = JSON.parse(text); } catch { throw new Error(`${method} ${path} returned non-JSON (${response.status})`); }
  const expectedStatuses = Array.isArray(expected) ? expected : [expected];
  if (!expectedStatuses.includes(response.status)) throw new Error(`${method} ${path} expected ${expectedStatuses.join(" or ")}, received ${response.status}`);
  if (response.headers.get("cache-control") !== "no-store" || response.headers.get("x-content-type-options") !== "nosniff") throw new Error(`${method} ${path} missing required security headers`);
  if (checkHealth && (json.status !== "ok" || json.runtime !== "rust-worker")) throw new Error("health response mismatch");
  return { json, headers: response.headers };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const onSignal = async () => {
    await activeCleanup?.();
    process.exitCode = 130;
  };
  process.once("SIGINT", onSignal);
  process.once("SIGTERM", onSignal);
  try {
    const result = await runSmoke({ baseUrl: process.env.TSUNORU_STAGING_URL, token: process.env.TSUNORU_STAGING_TOKEN });
    console.log(`PASS staging real-URL smoke event=${result.eventId} cleanup=verified`);
  } catch (error) {
    console.error(`FAIL staging real-URL smoke: ${error.message}`);
    process.exitCode = 1;
  } finally {
    process.removeListener("SIGINT", onSignal);
    process.removeListener("SIGTERM", onSignal);
  }
}

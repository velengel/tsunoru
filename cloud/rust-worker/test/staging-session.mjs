import assert from "node:assert/strict";
import { createHmac } from "node:crypto";
import { pathToFileURL } from "node:url";
import { APP_ORIGIN, FixturePool, STAGING_TOKEN, capability, request } from "./fixtures.mjs";

const COOKIE = "__Host-tsunoru_staging";

function signedCookie(expires, token = STAGING_TOKEN, origin = APP_ORIGIN) {
  const mac = createHmac("sha256", token)
    .update(`tsunoru-staging-session:v1\n${origin}\n${expires}`).digest("hex");
  return `${COOKIE}=v1.${expires}.${mac}`;
}

export async function verifyStagingSession(pool) {
  // A missing D1 binding makes unexpected database access fail visibly.
  const fixture = await pool.create({ database: false });
  const login = await request(fixture, "/api/staging/session", {
    method: "POST", bearer: null, origin: APP_ORIGIN,
    payload: { access_code: STAGING_TOKEN }, status: 200,
  });
  assert.deepEqual(login.json, { authenticated: true });
  const setCookie = login.headers.get("set-cookie");
  assert(setCookie?.startsWith(`${COOKIE}=v1.`), "login must issue a versioned host-only session");
  for (const attribute of ["HttpOnly", "Secure", "SameSite=Strict", "Path=/", "Max-Age=43200"]) {
    assert(setCookie.includes(attribute), `missing cookie attribute: ${attribute}`);
  }
  assert(!/\bDomain=/i.test(setCookie), "session must not span sibling apps");
  assert(!setCookie.includes(STAGING_TOKEN), "raw code must not become the cookie");
  const cookie = setCookie.split(";")[0];
  await request(fixture, "/api/staging/session", { bearer: null, headers: { cookie }, status: 200 });
  await request(fixture, "/api/staging/session", { bearer: null, status: 401 });
  await request(fixture, "/api/not-found", { bearer: null, headers: { cookie }, status: 404 });
  await request(fixture, "/api/not-found", {
    method: "POST", bearer: null, headers: { cookie }, origin: APP_ORIGIN, status: 404,
  });
  for (const origin of [undefined, "https://another.example.test", "null"]) {
    await request(fixture, "/api/staging/session", {
      method: "POST", bearer: null, origin,
      payload: { access_code: STAGING_TOKEN }, status: 403,
    });
    await request(fixture, "/api/not-found", {
      method: "POST", bearer: null, headers: { cookie }, origin, status: 403,
    });
  }
  await request(fixture, "/api/staging/session", {
    bearer: null, headers: { cookie }, origin: "https://another.example.test", status: 403,
  });
  await request(fixture, "/api/staging/session", {
    method: "POST", bearer: null, origin: APP_ORIGIN,
    payload: { access_code: capability(9) }, status: 401,
  });
  await request(fixture, "/api/staging/session", {
    method: "POST", bearer: null, origin: APP_ORIGIN,
    payload: { access_code: STAGING_TOKEN, unknown: true }, status: 400,
  });
  const now = Math.floor(Date.now() / 1000);
  const invalidCookies = [
    `${cookie.slice(0, -1)}${cookie.endsWith("0") ? "1" : "0"}`,
    `${COOKIE}=${STAGING_TOKEN}`,
    `${cookie}; ${COOKIE}=duplicate`,
    signedCookie(now - 10),
    signedCookie(now + 86_400),
    signedCookie(now + 100, STAGING_TOKEN, "https://another.example.test"),
    `${COOKIE}=v1.not-a-time.00`,
  ];
  for (const invalid of invalidCookies) {
    await request(fixture, "/api/staging/session", {
      bearer: null, headers: { cookie: invalid }, status: 401,
    });
  }
  // The independent Node HMAC implementation must produce an accepted session.
  await request(fixture, "/api/staging/session", {
    bearer: null, headers: { cookie: signedCookie(now + 100) }, status: 200,
  });
  await request(fixture, "/api/staging/session", {
    bearer: capability(9), headers: { cookie }, status: 401,
  });
  await request(fixture, "/api/not-found", { status: 404 });
  const rotated = await pool.create({ database: false, bindings: { STAGING_API_TOKEN: capability(9) } });
  await request(rotated, "/api/staging/session", { bearer: null, headers: { cookie }, status: 401 });
  const missing = await pool.create({ database: false, omit: ["STAGING_API_TOKEN"] });
  await request(missing, "/api/staging/session", {
    method: "POST", bearer: null, origin: APP_ORIGIN,
    payload: { access_code: STAGING_TOKEN }, status: 503,
  });
  await request(fixture, "/api/staging/session", {
    method: "DELETE", bearer: null, headers: { cookie }, status: 403,
  });
  const logout = await request(fixture, "/api/staging/session", {
    method: "DELETE", bearer: null, headers: { cookie }, origin: APP_ORIGIN, status: 200,
  });
  assert(logout.headers.get("set-cookie")?.includes("Max-Age=0"), "logout must clear this browser cookie");
  assert.deepEqual(logout.json, { authenticated: false });
  console.log("PASS staging session: login, cookie attributes, origin/CSRF, MAC, expiry, rotation, logout; no D1 binding");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const pool = new FixturePool();
  const interrupted = (signal) => pool.interrupt(new Error(`interrupted by ${signal}`));
  const onInt = () => interrupted("SIGINT");
  const onTerm = () => interrupted("SIGTERM");
  process.once("SIGINT", onInt);
  process.once("SIGTERM", onTerm);
  try {
    await verifyStagingSession(pool);
  } finally {
    await pool.dispose();
    process.removeListener("SIGINT", onInt);
    process.removeListener("SIGTERM", onTerm);
  }
}

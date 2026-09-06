import assert from "node:assert/strict";
import { fileURLToPath, pathToFileURL } from "node:url";
import { FixturePool, request } from "./fixtures.mjs";

export async function verifyStagingAssets(pool) {
  const fixture = await pool.create({
    database: false,
    assetsDirectory: fileURLToPath(new URL("./assets", import.meta.url)),
  });
  for (const path of ["/", "/events/shared-example", "/organizer/shared-example"]) {
    const response = await fixture.fetch(path, { headers: { "Sec-Fetch-Mode": "navigate" } });
    const html = await response.text();
    assert.equal(response.status, 200, `direct navigation ${path}: ${html}`);
    assert.match(html, /tsunoru-static-routing-fixture/);
    assert.match(response.headers.get("content-type"), /text\/html/);
    assert.equal(response.headers.get("referrer-policy"), "no-referrer");
    assert.equal(response.headers.get("x-frame-options"), "DENY");
    assert.equal(response.headers.get("x-content-type-options"), "nosniff");
    const csp = response.headers.get("content-security-policy");
    assert(csp?.includes("frame-ancestors 'none'"));
    assert(!csp.includes("'unsafe-eval'"), "general JavaScript evaluation must not be enabled");
  }
  // Navigation mode must never let SPA fallback hide authentication failures.
  await request(fixture, "/api/events/private", {
    bearer: null, headers: { "Sec-Fetch-Mode": "navigate" }, status: 401,
  });
  await request(fixture, "/api/not-found", {
    headers: { "Sec-Fetch-Mode": "navigate" }, status: 404,
  });
  await request(fixture, "/health", { bearer: null, status: 200 });
  console.log("PASS static app/direct routes, CSP and API-before-SPA authentication");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const pool = new FixturePool();
  const onInt = () => pool.interrupt(new Error("interrupted by SIGINT"));
  const onTerm = () => pool.interrupt(new Error("interrupted by SIGTERM"));
  process.once("SIGINT", onInt);
  process.once("SIGTERM", onTerm);
  try { await verifyStagingAssets(pool); }
  finally {
    await pool.dispose();
    process.removeListener("SIGINT", onInt);
    process.removeListener("SIGTERM", onTerm);
  }
}

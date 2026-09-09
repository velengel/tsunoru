import assert from "node:assert/strict";
import { test } from "node:test";
import { runSmoke } from "../../../scripts/verify-staging-real-url.mjs";

test("smoke verifier completes the staging journey and cleans up", async () => {
  const calls = [];
  const fetchImpl = async (url, init = {}) => {
    calls.push({ url, init });
    const path = new URL(url).pathname;
    if (path === "/health") return response(200, { status: "ok", runtime: "rust-worker" });
    if (path === "/api/staging/session" && init.method === "POST") return response(200, { authenticated: true }, { "set-cookie": "__Host-tsunoru_staging=v1.test; HttpOnly" });
    if (path === "/api/staging/session") return response(200, { authenticated: true });
    if (path === "/api/events" && init.method === "POST") return response(201, { id: JSON.parse(init.body).id, name: "Smoke event" });
    if (/^\/api\/events\/[^/]+$/.test(path) && init.method === "GET") return response(200, { id: path.split("/").at(-1), candidates: [{ id: "c1" }] });
    if (/\/responses$/.test(path) && init.method === "POST") return response(201, { event_id: path.split("/")[3], response_id: "response" });
    if (/\/responses$/.test(path)) return response(200, { responses: [{ response_id: "response" }] });
    if (/^\/api\/events\/[^/]+$/.test(path) && init.method === "DELETE") return response(200, { deleted: true });
    throw new Error(`unexpected ${init.method ?? "GET"} ${path}`);
  };
  const result = await runSmoke({ baseUrl: "https://staging.example.test", token: "a".repeat(64), fetchImpl, randomBytes: () => Buffer.alloc(32, 7) });
  assert.equal(result.deleted, true);
  assert.equal(calls.at(-1).init.method, "DELETE");
  assert(!calls.some(({ init }) => String(init.body ?? "").includes("a".repeat(64)) && init.method !== "POST"));
});

function response(status, body, headers = {}) {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", "cache-control": "no-store", "x-content-type-options": "nosniff", ...headers } });
}


test("smoke verifier cleans up after a response failure", async () => {
  const methods = [];
  const fetchImpl = async (url, init = {}) => {
    const path = new URL(url).pathname; methods.push(init.method ?? "GET");
    if (path === "/health") return response(200, { status: "ok", runtime: "rust-worker" });
    if (path === "/api/staging/session" && init.method === "POST") return response(200, { authenticated: true }, { "set-cookie": "__Host-tsunoru_staging=v1.test" });
    if (path === "/api/staging/session") return response(200, { authenticated: true });
    if (path === "/api/events" && init.method === "POST") return response(201, { id: JSON.parse(init.body).id, name: "Smoke event" });
    if (/^\/api\/events\/[^/]+$/.test(path) && init.method === "GET") return response(200, { id: path.split("/").at(-1), candidates: [{ id: "c1" }] });
    if (/\/responses$/.test(path) && init.method === "POST") return response(500, { error: "temporary" });
    if (/^\/api\/events\/[^/]+$/.test(path) && init.method === "DELETE") return response(200, { deleted: true });
    throw new Error(`unexpected ${init.method ?? "GET"} ${path}`);
  };
  await assert.rejects(runSmoke({ baseUrl: "https://staging.example.test", token: "a".repeat(64), fetchImpl, randomBytes: () => Buffer.alloc(32, 8) }));
  assert.equal(methods.at(-1), "DELETE");
});

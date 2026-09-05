import { Miniflare } from "/Users/velengel/Developer/active/koji-todo/node_modules/miniflare/dist/src/index.js";
import { readFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
const persist = await mkdtemp(join(tmpdir(), "tsunoru-rust-worker-"));
let mf;
const cleanup = async (code = 0) => { if (mf) await mf.dispose(); await rm(persist, { recursive: true, force: true }); process.exit(code); };
process.once("SIGINT", () => void cleanup(130)); process.once("SIGTERM", () => void cleanup(143));
mf = new Miniflare({ scriptPath: "build/index.js", modules: true, modulesRules: [{ type: "ESModule", include: ["**/*.js"], fallthrough: true }, { type: "CompiledWasm", include: ["**/*.wasm"], fallthrough: true }], d1Databases: ["DB"], d1Persist: persist });
try {
  const db = await mf.getD1Database("DB"); await db.exec(await readFile("schema.sql", "utf8"));
  const health = await (await mf.dispatchFetch("http://localhost/health")).json(); if (health.status !== "ok" || health.runtime !== "rust-worker") throw new Error("health failed");
  const created = await mf.dispatchFetch("http://localhost/api/events", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ id: "local-event", name: "Local event" }) }); if (created.status !== 200 || (await created.json()).id !== "local-event") throw new Error("create failed");
  const invalid = await mf.dispatchFetch("http://localhost/api/events", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ id: "", name: "invalid" }) }); if (invalid.status !== 400) throw new Error(`invalid status ${invalid.status}`);
  console.log("PASS rust-worker health + D1 event create + validation"); await cleanup(0);
} catch (error) { console.error(error); await cleanup(1); }

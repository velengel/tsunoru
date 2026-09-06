import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Miniflare, convertV4MiniflareOptions } from "miniflare";

export const APP_ORIGIN = "https://tsunoru-staging.example.test";
// Synthetic values generated solely for this disposable, in-memory fixture.
export const capability = (number) => number.toString(16).padStart(64, "0");
export const STAGING_TOKEN = capability(1);
export const ORGANIZER_TOKEN = capability(2);
const workerPath = fileURLToPath(new URL("../build/index.js", import.meta.url));
const schemaPath = new URL("../schema.sql", import.meta.url);

export class FixturePool {
  #workers = new Set();
  #closing = new WeakMap();
  #disposed;
  #interrupted;

  check() {
    if (this.#interrupted) throw this.#interrupted;
  }

  interrupt(reason) {
    this.#interrupted ||= reason;
    // The main finally also awaits this same promise before allowing exit.
    void this.dispose().catch(() => {});
  }

  async operation(promise, label) {
    this.check();
    let timeout;
    try {
      const result = await Promise.race([
        promise,
        new Promise((_, reject) => {
          timeout = setTimeout(() => reject(new Error(`${label} timed out`)), 30_000);
        }),
      ]);
      this.check();
      return result;
    } finally {
      clearTimeout(timeout);
    }
  }

  async create({ bindings = {}, omit = [], database = true, initialize = true, assetsDirectory } = {}) {
    this.check();
    if (this.#disposed) throw new Error("fixture pool is already closed");
    const values = { STAGING_API_TOKEN: STAGING_TOKEN, APP_ORIGIN, ...bindings };
    for (const key of omit) delete values[key];
    // Register ownership synchronously, before any initialization can yield.
    const mf = new Miniflare(convertV4MiniflareOptions({
      modulesRoot: fileURLToPath(new URL("../build/", import.meta.url)),
      modules: [
        { type: "ESModule", path: workerPath },
        { type: "CompiledWasm", path: fileURLToPath(new URL("../build/index_bg.wasm", import.meta.url)) },
      ],
      compatibilityDate: "2026-09-03",
      cf: false,
      telemetry: { enabled: false },
      bindings: values,
      d1Databases: database ? ["DB"] : [],
      d1Persist: false,
      ...(assetsDirectory ? { assets: {
        directory: assetsDirectory,
        binding: "ASSETS",
        run_worker_first: true,
        routerConfig: { has_user_worker: true },
        assetConfig: { not_found_handling: "single-page-application" },
      } } : {}),
    }));
    this.#workers.add(mf);
    let db;
    if (database) {
      db = await this.operation(mf.getD1Database("DB"), "D1 initialization");
      if (initialize) {
        const schema = await readFile(schemaPath, "utf8");
        // exec splits its input at newlines; complete statements may be multiline.
        for (const statement of schema.split(";").map((part) => part.trim()).filter(Boolean)) {
          await this.operation(db.prepare(statement).run(), "schema initialization");
        }
      }
    }
    return {
      db,
      fetch: async (path, init = {}) => this.operation(
        mf.dispatchFetch(`${APP_ORIGIN}${path}`, init),
        `${init.method || "GET"} ${path}`,
      ),
      close: () => this.#close(mf),
    };
  }

  #close(mf) {
    if (!this.#closing.has(mf)) {
      this.#closing.set(mf, mf.dispose().then(() => this.#workers.delete(mf)));
    }
    return this.#closing.get(mf);
  }

  dispose() {
    this.#disposed ||= (async () => {
      const results = await Promise.allSettled([...this.#workers].map((mf) => this.#close(mf)));
      this.#workers.clear();
      const errors = results.filter((result) => result.status === "rejected").map((result) => result.reason);
      if (errors.length) throw new AggregateError(errors, "failed to dispose owned Miniflare instances");
    })();
    return this.#disposed;
  }
}

export async function request(fixture, path, {
  method = "GET", payload, rawBody, headers = {}, bearer = STAGING_TOKEN,
  origin, status, contentType = "application/json", checkHeaders = true,
} = {}) {
  const requestHeaders = new Headers(headers);
  if (bearer !== null) requestHeaders.set("authorization", `Bearer ${bearer}`);
  if (origin !== undefined) requestHeaders.set("origin", origin);
  const body = rawBody === undefined && payload !== undefined ? JSON.stringify(payload) : rawBody;
  if (body !== undefined && contentType !== null) requestHeaders.set("content-type", contentType);
  const response = await fixture.fetch(path, {
    method,
    headers: requestHeaders,
    ...(body === undefined ? {} : { body }),
    ...(body instanceof ReadableStream ? { duplex: "half" } : {}),
  });
  const text = await response.text();
  if (status !== undefined) assert.equal(response.status, status, `${method} ${path}: ${text}`);
  if (checkHeaders) {
    assert.equal(response.headers.get("cache-control"), "no-store", `${path}: private data must not be cached`);
    assert.equal(response.headers.get("x-content-type-options"), "nosniff", `${path}: nosniff header`);
    assert.notEqual(response.headers.get("access-control-allow-origin"), "*", `${path}: no wildcard CORS`);
  }
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    assert.fail(`${method} ${path}: expected JSON, received ${text.slice(0, 200)}`);
  }
  return { status: response.status, json, text, headers: response.headers };
}

export async function rowCounts(db) {
  const counts = {};
  for (const table of ["events", "candidates", "responses", "answers"]) {
    counts[table] = (await db.prepare(`SELECT COUNT(*) AS count FROM ${table}`).first()).count;
  }
  return counts;
}

export function assertPublicPayload(value) {
  const inspect = (node) => {
    if (!node || typeof node !== "object") return;
    for (const [key, entry] of Object.entries(node)) {
      assert(!/capability|token|hash/i.test(key), `private field exposed: ${key}`);
      inspect(entry);
    }
  };
  inspect(value);
  const serialized = JSON.stringify(value);
  for (const token of [STAGING_TOKEN, ORGANIZER_TOKEN]) {
    assert(!serialized.includes(token), "synthetic secret leaked in response");
  }
}

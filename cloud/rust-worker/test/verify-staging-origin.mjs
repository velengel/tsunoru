import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => readFile(path.join(root, name), "utf8");
const config = JSON.parse(await read("staging-origin.json"));
assert.equal(config.fallbackStatus, "migration-only");
assert.match(config.canonical, /^https:\/\/staging\.tsunoru\.velengel\.com$/);
assert.match(config.legacyFallback, /^https:\/\/tsunoru-staging\.kounakadora528\.workers\.dev$/);

const wrangler = await read("wrangler.toml");
const staging = wrangler.slice(wrangler.indexOf("[env.staging]"));
assert.ok(staging, "wrangler.toml must define env.staging");
assert.match(staging, new RegExp(`APP_ORIGIN = "${config.canonical.replaceAll(".", "\\.")}"`));
assert.match(staging, new RegExp(`pattern = "${new URL(config.canonical).hostname.replaceAll(".", "\\.")}"`));

const readme = await read("README.md");
assert.match(readme, new RegExp("canonical app origin `" + config.canonical.replaceAll(".", "\\.") + "`"));
assert.match(readme, new RegExp("`" + config.legacyFallback.replaceAll(".", "\\.") + "`"));
assert.match(readme, /migration-only fallback/);

console.log(`PASS: canonical staging origin ${config.canonical} is aligned`);

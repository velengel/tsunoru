import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
const expectations = {
  "route.rs": /pub\s+async\s+fn\s+route|fn\s+route/,
  "policy.rs": /capability|authorize|session/,
  "events.rs": /event|Event/,
  "responses.rs": /response|Response/,
  "cleanup.rs": /delete|cleanup|Delete/,
};
for (const [module, pattern] of Object.entries(expectations)) {
  const source = await readFile(join(root, module), "utf8");
  assert.match(source, pattern, `${module} should own implementation code`);
}

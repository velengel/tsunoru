import assert from "node:assert/strict";
import { access } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
for (const module of ["route.rs", "policy.rs", "events.rs", "responses.rs", "cleanup.rs"]) {
  await assert.doesNotReject(access(join(root, module)), `${module} should define one Worker responsibility`);
}

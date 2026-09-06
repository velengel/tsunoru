#!/usr/bin/env node
// Synthetic build commands for failure/interrupt tests; no compiler or network.
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, join, resolve, sep } from "node:path";

const root = process.env.TSUNORU_BUILD_TEST_ROOT;
if (!root || await readFile(join(root, "fixture.marker"), "utf8") !== "tsunoru-build-test") {
  throw new Error("isolated fixture root required");
}
if (process.argv.includes("--child")) {
  setInterval(() => {}, 1000);
} else if (basename(process.argv[1]) === "cargo") {
  console.log(JSON.stringify({ target_directory: join(root, "target") }));
} else if (basename(process.argv[1]) === "dx") {
  const output = join(root, "target/dx/tsunoru/release/web/public");
  await mkdir(output, { recursive: true });
  await writeFile(join(output, "index.html"), '<html><script type="module" src="/app.js"></script></html>');
  await writeFile(join(output, "app.js"), "// synthetic new app\n");
} else if (basename(process.argv[1]) === "worker-build") {
  const position = process.argv.indexOf("--out-dir");
  const output = resolve(process.cwd(), position < 0 ? "build" : process.argv[position + 1]);
  if (!output.startsWith(resolve(root) + sep)) throw new Error("output escapes fixture");
  await mkdir(output, { recursive: true });
  await writeFile(join(output, "index.js"), "// synthetic new worker\n");
  console.log("BUILD_TEST_WORKER_STARTED");
  if (process.env.TSUNORU_BUILD_TEST_MODE === "hold") {
    const child = spawn(process.execPath, [process.argv[1], "--child"], { stdio: "ignore" });
    console.log(`BUILD_TEST_READY ${process.pid} ${child.pid}`);
    setInterval(() => {}, 1000);
  } else if (process.env.TSUNORU_BUILD_TEST_MODE === "fail") process.exitCode = 23;
} else throw new Error("unexpected fixture command");

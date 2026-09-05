import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const workerRoot = fileURLToPath(new URL("../", import.meta.url));
const liveGroup = (groupId) => execFileSync("ps", ["-axo", "pid=,pgid=,stat=,comm="], { encoding: "utf8" })
  .split("\n")
  .map((line) => /^\s*(\d+)\s+(\d+)\s+(\S+)\s+(.+)$/.exec(line))
  .filter((row) => row && Number(row[2]) === groupId && !row[3].startsWith("Z"))
  .map((row) => ({ pid: Number(row[1]), command: row[4] }));

let active;
let interrupted;
const interrupt = (signal) => {
  interrupted = signal;
  process.exitCode = signal === "SIGINT" ? 130 : 143;
  active?.kill(signal);
};
const onInterrupt = () => interrupt("SIGINT");
const onTerminate = () => interrupt("SIGTERM");
process.once("SIGINT", onInterrupt);
process.once("SIGTERM", onTerminate);

try {
for (const { signal, ready } of [
  { signal: "SIGINT", ready: false },
  { signal: "SIGTERM", ready: true },
]) {
  if (interrupted) break;
  const temporary = await mkdtemp(join(tmpdir(), "tsunoru-worker-cleanup-"));
  if (interrupted) {
    await rm(temporary, { recursive: true, force: true });
    break;
  }
  const child = spawn(process.execPath, ["verify-local.mjs"], {
    cwd: workerRoot,
    detached: true,
    env: { ...process.env, TMPDIR: temporary },
    stdio: ["ignore", "pipe", "pipe"],
  });
  active = child;
  let output = "";
  let finished;
  const closed = new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code, exitSignal) => {
      finished = { code, signal: exitSignal };
      resolve(finished);
    });
  });
  // Observe this promise even if process discovery throws before it is awaited.
  void closed.catch(() => {});
  child.stdout.on("data", (chunk) => { output = (output + chunk).slice(-16_384); });
  child.stderr.on("data", (chunk) => { output = (output + chunk).slice(-16_384); });
  try {
    const deadline = Date.now() + 30_000;
    let foundWorker = false;
    while (Date.now() < deadline && !finished && !interrupted) {
      foundWorker ||= liveGroup(child.pid).some((process) => /(?:^|\/)workerd$/.test(process.command));
      if (foundWorker && (!ready || output.includes("PASS event creation"))) break;
      await delay(20);
    }
    if (interrupted) throw new Error(`cleanup verification interrupted by ${interrupted}`);
    assert(foundWorker && !finished, `no owned Worker available for ${signal}: ${output}`);
    if (ready) assert(output.includes("PASS event creation"), `verification never reached active D1 operations: ${output}`);
    child.kill(signal);
    let deadlineTimer;
    const result = await Promise.race([
      closed,
      new Promise((_, reject) => {
        deadlineTimer = setTimeout(() => reject(new Error(`${signal} cleanup timed out`)), 15_000);
      }),
    ]).finally(() => clearTimeout(deadlineTimer));
    assert.deepEqual(result, { code: signal === "SIGINT" ? 130 : 143, signal: null }, output);
    assert.deepEqual(liveGroup(child.pid), [], `owned subprocess survived ${signal}`);
    assert.deepEqual(await readdir(temporary), [], `owned temporary data survived ${signal}`);
    console.log(`PASS ${signal} during ${ready ? "D1 verification" : "Worker initialization"}: no owned subprocess or temporary data remains`);
  } finally {
    if (child.pid && liveGroup(child.pid).length) {
      // This detached process group was created solely by this test invocation.
      try { process.kill(-child.pid, "SIGKILL"); } catch (error) { if (error.code !== "ESRCH") throw error; }
    }
    await closed.catch(() => {});
    active = undefined;
    await rm(temporary, { recursive: true, force: true });
  }
}
} catch (error) {
  process.exitCode ||= 1;
  console.error(error);
} finally {
  process.removeListener("SIGINT", onInterrupt);
  process.removeListener("SIGTERM", onTerminate);
}

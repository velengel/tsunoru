import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { chmod, copyFile, mkdir, mkdtemp, readFile, readdir, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

let active;
let interrupted;
const interrupt = (signal) => { interrupted = signal; active?.kill(signal); };
const onInt = () => interrupt("SIGINT");
const onTerm = () => interrupt("SIGTERM");
process.once("SIGINT", onInt);
process.once("SIGTERM", onTerm);
const live = (pids) => execFileSync("ps", ["-axo", "pid=,stat="], { encoding: "utf8" })
  .split("\n").map(line => /^\s*(\d+)\s+(\S+)/.exec(line))
  .filter(row => row && pids.includes(Number(row[1])) && !row[2].startsWith("Z"));

try {
  for (const mode of ["fail", "SIGINT", "SIGTERM", "success"]) {
    if (interrupted) throw new Error(`interrupted by ${interrupted}`);
    const temporary = await realpath(await mkdtemp(join(tmpdir(), "tsunoru-staging-build-")));
    let child, closed, finished, ownedPids = [];
    try {
      const worker = join(temporary, "cloud/rust-worker");
      const bin = join(temporary, "bin");
      await mkdir(bin, { recursive: true });
      await mkdir(join(worker, "build/public"), { recursive: true });
      await writeFile(join(temporary, "fixture.marker"), "tsunoru-build-test");
      await writeFile(join(temporary, "package.json"), '{"type":"module"}');
      await writeFile(join(worker, "build/public/last-success.txt"), "old app");
      await writeFile(join(worker, "build/last-success.txt"), "old worker");
      await copyFile(fileURLToPath(new URL("../build-staging.mjs", import.meta.url)), join(worker, "build-staging.mjs"));
      for (const command of ["cargo", "dx", "worker-build"]) {
        await copyFile(fileURLToPath(new URL("./build-cli-fixture.mjs", import.meta.url)), join(bin, command));
        await chmod(join(bin, command), 0o700);
      }
      child = spawn(process.execPath, [join(worker, "build-staging.mjs")], {
        cwd: worker, detached: true, stdio: ["ignore", "pipe", "pipe"],
        env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, TSUNORU_BUILD_TEST_ROOT: temporary,
          TSUNORU_BUILD_TEST_MODE: mode.startsWith("SIG") ? "hold" : mode },
      });
      active = child;
      let output = "";
      closed = new Promise((resolve, reject) => {
        child.once("error", reject);
        child.once("close", (code, signal) => { finished = { code, signal }; resolve(finished); });
      });
      void closed.catch(() => {});
      const append = (chunk) => {
        output = (output + chunk).slice(-16_384);
        const ready = /BUILD_TEST_READY (\d+) (\d+)/.exec(output);
        if (ready) ownedPids = ready.slice(1).map(Number);
      };
      child.stdout.on("data", append); child.stderr.on("data", append);
      if (mode.startsWith("SIG")) {
        const deadline = Date.now() + 15_000;
        let ready;
        while (!(ready = /BUILD_TEST_READY (\d+) (\d+)/.exec(output)) && !finished && !interrupted && Date.now() < deadline) await delay(20);
        assert(ready && !finished && !interrupted, `fixture never reached partial Worker build: ${output}`);
        ownedPids = ready.slice(1).map(Number);
        child.kill(mode);
      }
      let timer;
      const result = await Promise.race([closed, new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`build cleanup timeout: ${output}`)), 15_000);
      })]).finally(() => clearTimeout(timer));
      const code = mode === "success" ? 0 : mode === "SIGINT" ? 130 : mode === "SIGTERM" ? 143 : 1;
      assert.deepEqual(result, { code, signal: null }, output);
      assert.match(output, /BUILD_TEST_WORKER_STARTED/, "fixture must reach the partial Worker build");
      if (mode === "success") {
        assert.match(await readFile(join(worker, "build/public/index.html"), "utf8"), /app.js/);
        assert.match(await readFile(join(worker, "build/index.js"), "utf8"), /new worker/);
        assert.match(await readFile(join(worker, "build/asset-sha256.txt"), "utf8"), /index.html/);
      } else {
        assert.equal(await readFile(join(worker, "build/public/last-success.txt"), "utf8"), "old app", "failed build must preserve the last successful app");
        assert.deepEqual(await readdir(join(worker, "build/public")), ["last-success.txt"]);
        assert.deepEqual((await readdir(join(worker, "build"))).sort(), ["last-success.txt", "public"]);
      }
      assert.deepEqual((await readdir(worker)).filter(name => name.startsWith(".staging-build-")), []);
      assert.deepEqual(live(ownedPids), [], "owned compiler descendants survived");
      console.log(`PASS staging build ${mode}: published output and owned artifacts/processes`);
    } finally {
      if (child && !finished) child.kill("SIGTERM");
      if (closed && !finished) await Promise.race([closed.catch(() => {}), delay(1000)]);
      if (ownedPids.length && live(ownedPids).length) {
        try { process.kill(-ownedPids[0], "SIGKILL"); } catch (error) { if (error.code !== "ESRCH") throw error; }
      }
      if (child && !finished) { try { process.kill(-child.pid, "SIGKILL"); } catch (error) { if (error.code !== "ESRCH") throw error; } }
      await closed?.catch(() => {});
      active = undefined;
      await rm(temporary, { recursive: true, force: true });
    }
  }
} catch (error) {
  process.exitCode = interrupted === "SIGINT" ? 130 : interrupted === "SIGTERM" ? 143 : 1;
  console.error(error);
} finally {
  process.removeListener("SIGINT", onInt);
  process.removeListener("SIGTERM", onTerm);
}

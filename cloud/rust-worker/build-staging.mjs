import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { cp, mkdtemp, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const directory = dirname(fileURLToPath(import.meta.url));
const root = fileURLToPath(new URL("../../", import.meta.url));
const destination = join(directory, "build");
let child;
let interrupted;
let temporary;
let previous;
let promoted = false;
let retainRecovery = false;
const checkInterrupted = () => {
  if (interrupted) throw new Error(`build interrupted by ${interrupted}`);
};
const interrupt = (signal) => {
  interrupted ||= signal;
  // Each invocation owns one child process group; never stop another dev server.
  if (child?.pid) {
    try { process.kill(-child.pid, signal); }
    catch (error) { if (error.code !== "ESRCH") throw error; }
  }
};
const onInt = () => interrupt("SIGINT");
const onTerm = () => interrupt("SIGTERM");
process.once("SIGINT", onInt);
process.once("SIGTERM", onTerm);

function signalGroup(pid, signal) {
  try { process.kill(-pid, signal); return true; }
  catch (error) { if (error.code === "ESRCH") return false; throw error; }
}

async function stopGroup(pid) {
  if (!pid) return;
  // The command may have exited while its detached group's helpers remain.
  for (const signal of ["SIGTERM", "SIGKILL"]) {
    if (!signalGroup(pid, signal)) return;
    const deadline = Date.now() + 1500;
    while (Date.now() < deadline) {
      if (!signalGroup(pid, 0)) return;
      await delay(20);
    }
  }
  throw new Error(`owned compiler process group ${pid} did not exit`);
}

async function run(command, args, cwd, capture = false) {
  checkInterrupted();
  child = spawn(command, args, {
    cwd, detached: true, stdio: capture ? ["ignore", "pipe", "inherit"] : "inherit",
  });
  const processGroup = child.pid;
  let output = "";
  if (capture) child.stdout.setEncoding("utf8").on("data", (chunk) => { output += chunk; });
  const closed = new Promise((resolve) => child.once("close", resolve));
  try {
    const { code, signal } = await new Promise((resolve, reject) => {
      child.once("error", reject);
      child.once("exit", (code, signal) => resolve({ code, signal }));
    });
    if (code !== 0 || interrupted) throw new Error(`${command} failed (${signal || code})`);
    await closed;
    checkInterrupted();
    return output;
  } catch (error) {
    // Reap helpers before waiting for inherited pipes to close or deleting output.
    await stopGroup(processGroup);
    await closed;
    throw error;
  } finally { child = undefined; }
}

try {
  temporary = await mkdtemp(join(directory, ".staging-build-"));
  const assets = join(temporary, "assets");
  const bundle = join(temporary, "bundle");
  const metadata = JSON.parse(await run("cargo", ["metadata", "--no-deps", "--format-version", "1", "--locked"], root, true));
  const source = join(metadata.target_directory, "dx", "tsunoru", "release", "web", "public");
  // Dioxus can retain old hashed assets across builds. Publish only this build.
  await rm(source, { recursive: true, force: true });
  await run("dx", ["build", "--web", "--release", "--no-default-features", "--features", "cloud-web", "--debug-symbols=false", "--locked"], root);
  const html = await readFile(join(source, "index.html"), "utf8");
  // Assemble privately; a failed copy or compiler must not replace a good bundle.
  await cp(source, assets, { recursive: true });
  const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)];
  if (scripts.length !== 1 || !/\bsrc\s*=/.test(scripts[0][1]) || scripts[0][2].trim()) {
    throw new Error("expected one external Dioxus CSR loader compatible with script-src self");
  }
  const digests = [];
  async function inspect(path, prefix = "") {
    for (const entry of await readdir(path, { withFileTypes: true })) {
      const relative = `${prefix}${entry.name}`;
      const absolute = join(path, entry.name);
      if (entry.isDirectory()) await inspect(absolute, `${relative}/`);
      else if (/\.(br|gz|map)$/.test(entry.name)) await rm(absolute);
      else {
        const bytes = await readFile(absolute);
        digests.push(`${createHash("sha256").update(bytes).digest("hex")}  ${relative}`);
      }
    }
  }
  await inspect(assets);
  await run("worker-build", ["--release", "--out-dir", relative(directory, bundle)], directory);
  await rename(assets, join(bundle, "public"));
  await writeFile(join(bundle, "asset-sha256.txt"), `${digests.sort().join("\n")}\n`);
  checkInterrupted();
  try {
    await rename(destination, join(temporary, "previous-build"));
    previous = join(temporary, "previous-build");
  } catch (error) { if (error.code !== "ENOENT") throw error; }
  checkInterrupted();
  await rename(bundle, destination);
  promoted = true;
  checkInterrupted();
  console.log(`PASS staging build: ${digests.length} assets, external CSR loader, Rust Worker`);
} catch (error) {
  process.exitCode = interrupted === "SIGINT" ? 130 : interrupted === "SIGTERM" ? 143 : 1;
  console.error(error.message);
  try {
    if (promoted) await rm(destination, { recursive: true, force: true });
    if (previous) await rename(previous, destination);
  } catch (recoveryError) {
    // Do not erase the last good bundle if the filesystem prevents restoration.
    retainRecovery = true;
    console.error(`Could not restore the previous build: ${recoveryError.message}. Recovery files: ${temporary}`);
  }
} finally {
  if (temporary && !retainRecovery) {
    try { await rm(temporary, { recursive: true, force: true }); }
    catch (error) { process.exitCode ||= 1; console.error(`Could not remove build staging directory ${temporary}: ${error.message}`); }
  }
  process.removeListener("SIGINT", onInt);
  process.removeListener("SIGTERM", onTerm);
}

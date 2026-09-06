import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const directory = dirname(fileURLToPath(import.meta.url));
const root = fileURLToPath(new URL("../../", import.meta.url));
const destination = join(directory, "public");
let child;
let interrupted;
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

async function run(command, args, cwd, capture = false) {
  if (interrupted) throw new Error(`build interrupted by ${interrupted}`);
  return new Promise((resolve, reject) => {
    child = spawn(command, args, {
      cwd, detached: true, stdio: capture ? ["ignore", "pipe", "inherit"] : "inherit",
    });
    let output = "";
    if (capture) child.stdout.setEncoding("utf8").on("data", (chunk) => { output += chunk; });
    child.once("error", reject);
    child.once("close", (code, signal) => {
      child = undefined;
      if (code === 0 && !interrupted) resolve(output);
      else reject(new Error(`${command} failed (${signal || code})`));
    });
  });
}

try {
  const metadata = JSON.parse(await run("cargo", ["metadata", "--no-deps", "--format-version", "1", "--locked"], root, true));
  const source = join(metadata.target_directory, "dx", "tsunoru", "release", "web", "public");
  // Dioxus can retain old hashed assets across builds. Publish only this build.
  await rm(source, { recursive: true, force: true });
  await run("dx", ["build", "--web", "--release", "--no-default-features", "--features", "cloud-web", "--debug-symbols=false", "--locked"], root);
  const html = await readFile(join(source, "index.html"), "utf8");
  // Work only on this script's ignored, generated staging output.
  await rm(destination, { recursive: true, force: true });
  await cp(source, destination, { recursive: true });
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
  await inspect(destination);
  await run("worker-build", ["--release"], directory);
  await mkdir(join(directory, "build"), { recursive: true });
  await writeFile(join(directory, "build", "asset-sha256.txt"), `${digests.sort().join("\n")}\n`);
  console.log(`PASS staging build: ${digests.length} assets, external CSR loader, Rust Worker`);
} catch (error) {
  process.exitCode = interrupted === "SIGINT" ? 130 : interrupted === "SIGTERM" ? 143 : 1;
  console.error(error.message);
} finally {
  process.removeListener("SIGINT", onInt);
  process.removeListener("SIGTERM", onTerm);
}

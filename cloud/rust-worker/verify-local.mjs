import { FixturePool } from "./test/fixtures.mjs";
import { verifyStagingApi } from "./test/staging-api.mjs";

const pool = new FixturePool();
const keepAlive = setInterval(() => {}, 1_000);
const interrupt = (signal) => {
  process.exitCode = signal === "SIGINT" ? 130 : 143;
  pool.interrupt(new Error(`verification interrupted by ${signal}`));
};
const onInterrupt = () => interrupt("SIGINT");
const onTerminate = () => interrupt("SIGTERM");
process.once("SIGINT", onInterrupt);
process.once("SIGTERM", onTerminate);

try {
  await verifyStagingApi(pool);
  console.log("PASS Rust Worker staging API contract");
} catch (error) {
  process.exitCode ||= 1;
  console.error(error);
} finally {
  try {
    await pool.dispose();
  } catch (error) {
    process.exitCode ||= 1;
    console.error("Miniflare cleanup failed:", error);
  } finally {
    clearInterval(keepAlive);
    process.removeListener("SIGINT", onInterrupt);
    process.removeListener("SIGTERM", onTerminate);
  }
}

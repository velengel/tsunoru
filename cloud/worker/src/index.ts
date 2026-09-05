import coreModule from "../../core/target/wasm32-unknown-unknown/release/tsunoru_worker_core.wasm";

type CoreExports = WebAssembly.Exports & {
  domain_probe: () => number;
  argon2_probe: (wrong: number) => number;
};

const core = new WebAssembly.Instance(coreModule, {}).exports as CoreExports;

function json(value: unknown, status = 200): Response {
  return Response.json(value, { status, headers: { "Cache-Control": "no-store" } });
}

export default {
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === "/api/cloudflare-probe" && request.method === "GET") {
      return json({
        domain_validation: core.domain_probe(),
        argon2_repeatable: core.argon2_probe(0) === core.argon2_probe(0),
        argon2_changes_for_other_input: core.argon2_probe(0) !== core.argon2_probe(1),
      });
    }
    return json({ error: "not found" }, 404);
  },
};

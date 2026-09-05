# Local Cloudflare compatibility probe

This is a synthetic, local-only experiment, not a deployable application.
There is deliberately no Wrangler deployment configuration.
`worker.mjs` uses a fixed fixture session, unauthenticated diagnostic endpoints and a fixed Argon2 salt. Never expose it publicly.

The Rust module includes the real `src/domain.rs` with its server timezone validation enabled. JavaScript owns D1 access in this experiment; this does not verify workers-rs bindings or Dioxus server functions.

Prerequisites: Rust with `wasm32-unknown-unknown`, cached Cargo dependencies, Node, and an existing Miniflare installation.
Tested with Miniflare `5.20260730.0-alpha`, workerd package `1.20260730.1`, compatibility date `2026-08-06`. The runtime rejected `2026-09-05` as newer than supported. No tools were downloaded/installed for this fallback.

From the repository root:

```sh
cargo build --offline --locked --manifest-path experiments/cloudflare/Cargo.toml --target wasm32-unknown-unknown --release
MINIFLARE_MODULE=/absolute/path/to/node_modules/miniflare/dist/src/index.js node experiments/cloudflare/verify.mjs
cargo clippy --offline --locked --manifest-path experiments/cloudflare/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path experiments/cloudflare/Cargo.toml --check
```

The verifier creates a fresh temporary D1 directory, disables external CF metadata fetching, and disposes Miniflare/removes its directory on success or exception.
The database schema is a reduced model, not the seven production migrations. Event/answer creation, batch rollback, concurrent expected-tail changes and session revocation are checked. An unguarded zero-row UPDATE is also shown to commit a preceding insert.

Argon2id uses the production parameter values (19,456 KiB, t=2, p=1, output=32 bytes) with synthetic input. The 32-bit fingerprint is a diagnostic comparison only, never an authentication check. Measurements are local wall-clock latency and Wasm linear memory, not Cloudflare CPU accounting or total isolate memory. Full PHC parsing, salt generation, cookies, rate limiting and authentication are not verified here.

The HTTP assertions were run before `worker.mjs` existed and failed with ENOENT; after implementation all ten groups passed. `cargo test` contains zero unit tests; the meaningful assertions are the HTTP/runtime ones in `verify.mjs`.

Fullstack counterexample:

```sh
cargo check --offline --locked --manifest-path experiments/dioxus-worker-probe/Cargo.toml --target wasm32-unknown-unknown
```

This currently fails in mio through Dioxus/Tokio network features, even without SQLx and Argon2. Preserve that result rather than treating the raw Wasm experiment as a Fullstack success.

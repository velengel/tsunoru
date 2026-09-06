# Limited staging API

This Worker prepares isolated API testing before Dioxus UI integration. It does not implement the full native product. Use a new disposable D1 database: `schema.sql` is a fresh baseline, not a migration for the #8/#9 experiment. Existing tables must cause schema application to fail rather than silently accepting incompatible columns.

## Build and verify

```sh
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.8.5 --locked
npm ci --ignore-scripts
npm run check
cargo check --locked --target wasm32-unknown-unknown
cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings
cargo fmt --check
npm run deploy:check
```

Run from `cloud/rust-worker`. Node tests use a fresh in-memory Miniflare D1 and dispose their runtime on success, error, SIGINT and SIGTERM. They do not use Wrangler login, remote resources, the native SQLite database, or another repository's packages. `strip = "debuginfo"` preserves the information wasm-bindgen needs; `strip = true` does not.

## Request boundary

`GET /health` returns runtime status. Every `/api/*` route requires `Authorization: Bearer <STAGING_API_TOKEN>`. The staging token is 64 hex characters generated from 32 cryptographically random bytes; it is distinct from organizer and response capabilities. Missing/invalid settings fail closed with 503. Incorrect authentication returns 401 before reading a body or opening D1.

Configure one exact `APP_ORIGIN`, for example `https://tsunoru-staging.example.test`. If a request includes `Origin`, it must match exactly. `null`, other ports, other schemes, credentials, paths, and origin lists are rejected. Requests without `Origin` can be made by authenticated CLI clients. This API does not authenticate with cookies and does not enable cross-origin browser access. Origin checking alone is not authentication.

POST bodies must be JSON, at most 64 KiB while streaming. Malformed JSON or unknown fields return 400, other media types 415, oversized bodies 413. IDs use 1–64 ASCII letters, digits, `_`, or `-`. Names use 1–100 characters after trimming, candidate labels 1–100 characters, and events have 1–20 distinct candidates (matching native count/name limits). Date/time parsing and full native DTO parity belong to UI integration.

All responses use `Cache-Control: no-store` and `X-Content-Type-Options: nosniff`. Internal errors expose only `{ "error": { "code": "internal_error" } }`, never SQL, credentials or exception text.

## Routes

| Method/path | Additional credential | Request and behavior |
| --- | --- | --- |
| `POST /api/events` | `organizer_capability` in JSON, 64 hex characters | `{id,name,organizer_capability,candidates:[{id,label}]}`; 201 `{id,name}`; duplicate ID 409 |
| `GET /api/events/:id` | none | `{id,name,candidates:[{id,label}]}`; no capability hashes or private response data |
| `POST /api/events/:id/responses` | `x-response-capability`, unique 64-hex secret for this response | `{respondent_name,availabilities:[{candidate_id,availability}]}`; 201 `{event_id,response_id}`, identical replay 200, changed replay 409 |
| `GET /api/events/:id/responses` | `x-organizer-capability` | `{responses:[{response_id,respondent_name,availabilities:[{candidate_id,availability}]}]}`; invalid capability 403 |

The caller generates and retains a response capability **before** the first submission, so a lost HTTP response can be retried. Generate secrets with Web Crypto or an OS CSPRNG. D1 stores only SHA-256 hashes. A display name can be reused by different participants; public response IDs never authorize a write. The event-wide response credential and `/api/answers` from #9 are removed. Every candidate must occur exactly once, and availability is `available`, `maybe`, or `unavailable`.

The Worker normalizes names and candidate order, then compares a hash of the whole payload. D1 batch conditions check the candidate set, response capability, event and payload on the writes themselves. An identical concurrent submission creates one response; a competing changed payload receives 409 and cannot modify the first response. Any statement failure rolls back the batch. Response editing and per-response revocation are not implemented.

## Staging deployment preparation

The committed config has `workers_dev = false`, `preview_urls = false`, no routes, and a placeholder D1 ID. `npm run deploy:check` bundles locally without deploying. An actual staging deployment requires selecting a dedicated account/database and origin and completing these steps:

1. Create a **new** `tsunoru-staging` D1 (`npx wrangler d1 create tsunoru-staging`) and record its ID in `env.staging.d1_databases`. Do not use the native or old experiment database.
2. Apply the fresh schema once with `npx wrangler d1 execute tsunoru-staging --env staging --remote --file schema.sql`. Inspect the target account and DB before remote execution. Do not reset an existing DB.
3. Install a new 32-byte random hex token with the interactive `npx wrangler secret put STAGING_API_TOKEN --env staging`. Keep it in ignored local secret storage or a password manager; never use it as a command-line argument or commit it.
4. Set `APP_ORIGIN` to the chosen HTTPS origin. Enable only the chosen staging route/hostname. Keep preview URLs off.
5. Deploy with `npx wrangler deploy --env staging`, then check unauthenticated 401, wrong-origin 403, authorized create/read/answer, and identical retry against **synthetic** data.

The staging token can be rotated by replacing the Worker secret. Rate limiting, individual tester identity/expiry, account sessions, Dioxus UI integration and production backup/migration are follow-ups before wider use. Local tests and dry-run are not evidence of a live deployment.

References: [D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch), [Workers secrets](https://developers.cloudflare.com/workers/configuration/secrets/), [Wrangler configuration](https://developers.cloudflare.com/workers/wrangler/configuration/), ADR 0051–0053.

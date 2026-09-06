# Limited staging app

The Dioxus CSR app and Rust Worker share one origin. A tester enters the shared trial code, creates an event, shares its URL, submits availability, and opens the organizer matrix in the browser that created the event. Native Fullstack remains the default root build; `cloud-web` selects this smaller journey.

Use a **new** dedicated D1 database. `schema.sql` is a fresh baseline, not a migration for the native app or earlier experiments. Existing tables intentionally fail schema application. Accounts, response editing, comments, final decisions, browser-to-browser recovery, and existing-data migration are outside this pilot ([#12](https://github.com/velengel/tsunoru/issues/12)).

## Build and verify

Run from `cloud/rust-worker`, with the repository's Dioxus CLI and Rust toolchain installed:

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

`npm run check` tests the API, sessions, static routing and cleanup against disposable Miniflare instances, plus isolated build failure and signal fixtures. It uses neither remote resources nor the native SQLite database. `npm run deploy:check` builds the CSR app and Worker, verifies the external script loader, records asset SHA-256 hashes under `build/`, and bundles without deploying. The Worker and `build/public/` assets are assembled together in an ignored temporary directory and replace `build/` only after both builds succeed. Failure or SIGINT/SIGTERM removes that invocation's staging files and preserves the previous complete bundle. Compiler caches remain reusable; a filesystem recovery failure reports the retained backup path. App changes also require the root checks in `AGENTS.md`.

For browser development, create an ignored `.dev.vars.staging` with a **synthetic** 64-hex `STAGING_API_TOKEN`, initialize only the local database with `npx wrangler d1 execute tsunoru-staging --env staging --local --file schema.sql`, then run `npm run dev:app`. Open `http://localhost:8791` so the configured Origin matches; localhost allows the Secure cookie in supporting browsers. This local database belongs to this worktree. Never reset an existing database to rerun initialization.

## Authentication and storage

`POST /api/staging/session` exchanges `{access_code}` for a 12-hour HMAC-SHA256 signed `__Host-tsunoru_staging` cookie. It is HttpOnly, Secure, SameSite=Strict, host-only and bound to `APP_ORIGIN`. `GET` checks the session; `DELETE` clears this browser's cookie. Logout does not individually revoke a copied cookie. Rotating `STAGING_API_TOKEN` invalidates every session and the old trial code. The code is never embedded in assets or URLs and is not persisted in browser storage.

Other `/api/*` routes require a valid cookie or explicit `Authorization: Bearer <STAGING_API_TOKEN>` for CLI checks. Login, logout and cookie mutations require an exact Origin. Any supplied wrong Origin is rejected, including for Bearer calls. Missing or malformed settings return 503; bad credentials return 401 before D1 access. API responses are `no-store`; SQL, capabilities and raw exception messages never appear in responses or logs. Static files and `/health` are public; Worker-first routing keeps API authentication ahead of SPA fallback.

Organizer and response capabilities are separate random 64-hex secrets. The browser retains each operation's exact payload and capability **before** submission, allowing a lost response to be retried after reloading. Storage failure blocks the write. Event-creation 400 errors happen before D1 access, so the UI can restore the input for correction; uncertain failures retain the pending request. Browser storage must remain available to manage the event. A shared URL contains only the event ID and never transfers organizer rights.

### Google organizer session

`POST /api/organizer/session` accepts `{id_token}` from Google Identity Services. The Worker verifies the RS256 signature against Google's JWKS and checks `iss`, `aud`, `sub`, and `exp` before issuing a host-only HttpOnly session cookie. Configure `GOOGLE_CLIENT_ID` as a non-secret var and a random 64-hex `ORGANIZER_SESSION_SECRET` as a Worker secret. Until those settings exist, event creation/deletion keeps the legacy staging session so local fixtures remain runnable; production rollout must configure both before treating Google sign-in as enabled. ID tokens are never logged or returned.

## Event API

JSON bodies are limited to 64 KiB while streaming. IDs use 1–64 ASCII letters, digits, `_`, or `-`; names use 1–100 trimmed characters. Events accept 1–20 candidates and an optional 500-character organizer note. The Worker validates calendar dates, clock times, IANA zones and unique local datetimes. DST gaps and folds are rejected because a candidate must identify one instant.

| Method/path | Additional credential | Request and result |
| --- | --- | --- |
| `POST /api/events` | `organizer_capability` in JSON | `{id,name,time_zone,organizer_note,candidates:[{id,local_date,local_time}],organizer_capability}`; first create 201 `{id,name}`, identical authorized retry 200, changed/wrong-owner retry 409 |
| `GET /api/events/:id` | none | `{id,name,time_zone,organizer_note,candidates:[{id,local_date,local_time}]}`; no capabilities or private answers |
| `POST /api/events/:id/responses` | `x-response-capability` | `{respondent_name,availabilities:[{candidate_id,availability}]}`; 201 `{event_id,response_id}`, identical retry 200, changed retry 409 |
| `GET /api/events/:id/responses` | `x-organizer-capability` | `{responses:[{response_id,respondent_name,availabilities:[{candidate_id,availability}]}]}`; wrong capability 403 |
| `DELETE /api/events/:id` | `x-organizer-capability` | Deletes the event, candidates, responses and answers atomically; success 200 `{deleted:true}`, wrong capability 403 |

Every candidate must be answered exactly once with `available`, `maybe`, or `unavailable`. Display names may repeat; they never identify ownership. D1 keeps capability and normalized payload hashes. Authorization, complete candidate sets and writes are checked within one batch. Competing retries cannot mix answers; a statement failure rolls the whole batch back.

## Staging deployment

The selected account is the existing Koji Todo / Voice Workbench account. Only `env.staging` enables workers.dev; preview URLs stay disabled and no custom routes are changed. The intended Worker and new D1 are both named `tsunoru-staging`, with app origin `https://tsunoru-staging.kounakadora528.workers.dev`. The D1 ID remains a placeholder until creation is authorized and succeeds.

After approval for the new remote resources:

1. Create the dedicated empty DB: `npx wrangler d1 create tsunoru-staging --env staging --location apac --update-config=false`. Record the returned ID in `env.staging.d1_databases`.
2. Inspect the target, then apply `npx wrangler d1 execute tsunoru-staging --env staging --remote --file schema.sql` **once**. Never drop/reset existing tables.
3. Save `{ "STAGING_API_TOKEN": "<new 32-byte random hex value>" }` in ignored `secrets/staging.json`, owner-readable only. Keep the actual code in private local storage or a password manager. Do not put it in command arguments or commits.
4. Add `GOOGLE_CLIENT_ID` to the staging vars and `ORGANIZER_SESSION_SECRET` (a new random 64-hex value) to ignored `secrets/staging.json` when enabling Google organizer auth.
5. Run `npm run deploy:check`, then `npx wrangler deploy --env staging --secrets-file secrets/staging.json`. A new Worker accepts its first secret through `--secrets-file`; `secret put` requires the Worker to exist.
5. Record the deployed version and check health, assets, unauthenticated 401, wrong Origin 403, cookie login, create/read/answer/retry and organizer-only results using synthetic data. Check the same browser journey at 320px and desktop separately.

This pilot is for a few trusted testers with disposable data. Rate limits, retention/deletion policy for continued use, individual revocation, backup/restore and general-public readiness need the decisions in #12. Local tests and dry-run do not establish a deployed app or physical-phone behavior. Current evidence is in [report 0028](../../docs/reports/0028-staging-browser-app.md).

References: [Static Assets routing](https://developers.cloudflare.com/workers/static-assets/routing/worker-script/), [D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch), [Workers secrets](https://developers.cloudflare.com/workers/configuration/secrets/), [Wrangler deploy](https://developers.cloudflare.com/workers/wrangler/commands/deployments/), ADR 0055–0058.

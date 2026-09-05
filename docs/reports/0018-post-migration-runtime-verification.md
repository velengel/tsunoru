# Post-migration runtime verification

Date: 2026-09-05

## Outcome

The canonical checkout starts successfully and its built HTTP server supports the anonymous event lifecycle.
The four migrated events remain readable. Runtime verification changed only a disposable database copy.
Browser interaction remains UNVERIFIED because browser-client bootstrap rejected a plugin dependency's trusted code path.

## Evidence

| Layer | Result | Evidence |
| --- | --- | --- |
| Local files | PASS | `zsh scripts/verify-local-git-materialization.zsh`: both dataless counts zero |
| Toolchain | PASS | Rust/Cargo 1.98.0; Dioxus CLI 0.7.10 |
| Original SQLite | PASS | `PRAGMA integrity_check`: `ok`; `foreign_key_check`: no rows; four events |
| Default tests | PASS | `cargo test --all-targets`: 118 passed, zero failed, exit 0 |
| Server tests | PASS | `cargo test --all-targets --features server`: 226 passed, zero failed, exit 0 |
| Build/start | PASS | `dx serve --web --addr 127.0.0.1 --port 8081 --open false`: successful build in 137.15 seconds, server launched |
| Live HTTP | PASS | Root HTML, CSS, JavaScript, favicon, WASM and one migrated event returned HTTP 200 on port 8081 |
| Isolated HTTP lifecycle | PASS | `python3 scripts/verify-runtime.py`: `runtime_verification=PASS`, exit 0 |
| Original data preservation | PASS | Script compared complete SQL dumps and SHA-256 of the original DB before/after; both unchanged |
| Cleanup | PASS | Script-owned child server stopped and disposable directory removed; earlier port 8082 candidate stopped |
| Browser interaction | UNVERIFIED | `Trusted RPC dependency must resolve within a configured trusted code path` for browser-service.mjs during bootstrap |
| External deployment / physical device | UNVERIFIED | Not exercised |

No application Rust code changed. Clippy, formatting and a separate `dx build --web` were not repeated; the serving build itself succeeded.
The port 8081 development server was retained for local use at the end of verification.

## Runtime coverage

The repository script starts the existing built binary on a dedicated loopback port with a SQLite backup under ignored `var/`.
It verifies all four migrated events, then creates a new event and reads it back.
It submits a participant answer and comment, checks organizer counts and the full response matrix, and confirms wrong organizer authority returns the documented 404.
It decides one candidate, reads back the decision, downloads an iCalendar event and confirms a late answer receives 409.
Capability values remain in process memory and are not printed or saved by the script.

## Approval and request-format findings

The first lifecycle action was rejected before execution because it combined an ad-hoc `/tmp` script with test writes.
The reviewer explicitly recommended repository-scoped scripts.
After the user's follow-up, Story 0021 and ADR 0026 recorded a constrained implementation: a repository-owned script with a fixed server binary, loopback-only requests to its own child, disposable data and original-data checks.
That action passed automatic review without changing global permissions.
This is a verified outcome for this session, not a guarantee that later reviews will approve every script.

An initial manual GET with `?public_id=...` returned HTTP 500 with a missing-field decoding error.
The installed Dioxus 0.7.10 JSON extractor reads arguments from the request body; using a GET JSON body returned HTTP 200.
The first script run also expected 403 for wrong organizer authority; `src/server.rs` specifies 404, so the test expectation was corrected.
Neither observation required an application change.

## Reproduction

```sh
dx build --web
python3 scripts/verify-runtime.py
```

The script requires local listener permission. Run without concurrent writers to the original DB so its preservation assertions are meaningful.
This verifies HTTP behavior and persistence; it does not replace a browser hydration or interaction check.

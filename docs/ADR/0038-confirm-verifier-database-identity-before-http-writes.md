# ADR 0038: Confirm verifier database identity before HTTP writes

## context

Review R010 identifies that a free-port reservation can race with another server.
Matching HTML and CSS proves asset provenance but not which database receives a mutation.
Both browser and HTTP verifiers need an identity check and child-liveness monitoring.

## decision

Verify a fresh marker seeded directly into the disposable database through a read-only HTTP request before permitting test mutations.

## rejected options

- Rely on HTTP 200 and matching assets: another TSUNORU instance can return both.
- Use a fixed marker: another fixture could contain the same value.
- Add a production-only-for-tests identity endpoint: the existing public event read can establish the disposable database identity.

## consequences

Seed an unpredictable event ID and marker in the disposable database before the server starts, and verify both via the existing event read endpoint.
Keep checking the owned child around requests and stop browser traffic if it exits.
The browser fixture needs current migrations and SQLx migration metadata; stale schema or checksum assumptions must fail verification rather than bypass identity.
The source database and unrelated servers must never receive marker insertion or HTTP test writes.

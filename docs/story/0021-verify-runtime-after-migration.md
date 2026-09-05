# Story 0021: Verify runtime after migration

## context

The user wants to confirm TSUNORU starts and works after leaving iCloud.
Automatic approval rejected an ad-hoc `/tmp` script and recommended repository-scoped verification.

## definition of done

- [x] Check local materialization, original SQLite integrity, and both test configurations.
- [x] Verify the built server's event lifecycle against an isolated database copy.
- [x] Record HTTP, browser, data-preservation, and tooling evidence separately.

## to do

- [x] Add a reviewable repository script that owns its loopback server and database copy.
- [x] Run creation, response, summary, decision, and calendar checks.
- [x] Preserve the original database and stop the script-owned server.

Evidence: [verification report](../reports/0018-post-migration-runtime-verification.md).

## concern

Browser bootstrap currently fails with a trusted-code-path error.
HTTP verification cannot establish browser hydration or physical-device behavior.
Do not loosen global command permissions to make one verification pass.

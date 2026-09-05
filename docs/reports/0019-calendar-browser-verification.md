# Calendar browser verification

Date: 2026-09-05

## Outcome

The preserved calendar repair now has browser evidence at 320px and 1440px.
Two-digit dates stay on one line, all seven columns remain aligned, and calendar selection through event creation and the post-answer matrix succeeds.
Development resumed in the existing `fix/calendar-layout-verification` worktree. PR #6 is the delivery PR; the redundant starter PR #5 was closed.

## Measurements

| Case | Page width / scroll width | Smallest day target | Maximum numeric label height | Result |
| --- | --- | --- | --- | --- |
| Pre-repair 320px | 320 / 320 | 24.98 × 44 px | 32 px | FAIL: date digits wrap |
| Repaired 320px | 320 / 320 | 28.05 × 44 px | 16 px | PASS |
| Repaired 1440px | 1440 / 1440 | 70.38 × 44 px | 16 px | PASS |

All cases realized seven grid tracks. The repaired toolbar stayed on one row.
The narrow baseline screenshot shows dates such as 20 and 22 split vertically; both repaired screenshots show single-line numbers.
The selected state has a checkmark as well as color, and the focus outline is visible.
The answer matrix intentionally scrolls within its component at 320px; ArrowRight moves it without page-level overflow.

## Evidence layers

- Source tests: `cargo test --all-targets`, 118 passed, exit 0.
- Server tests: `cargo test --all-targets --features server`, 226 passed, exit 0.
- Static checks: `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check`, exit 0.
- Build: `dx build --web`, server and client succeeded, exit 0.
- Repository boundary: `zsh scripts/test-public-snapshot-boundary.zsh`, PASS.
- Live asset: the verifier calls `sh scripts/verify_served_calendar_assets.sh` against its owned server; PASS.
- Browser: `PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node scripts/verify-calendar-browser.mjs`, PASS at both widths.
- Negative fixture: the same command with `--stale-css` detects missing calendar selectors despite HTTP 200; PASS.
- Before/after: the same geometry assertions against the pre-repair build failed on numeric-label wrapping before the repaired build passed.
- Data: browser writes use a fresh disposable database, not the user's migrated database. Runner-owned Chromium and server are stopped on completion.
- External deployment, physical iPhone, and screen-reader speech: UNVERIFIED.

The actual HTML-linked CSS was `/assets/main-dxh4149cb41d3ca4f8b.css`, returned as `text/css`, and matched this server bundle's file exactly.
Playwright 1.62.0 used installed Chromium 151.0.7922.34 with a fresh profile and reduced motion.
No uncaught application exceptions occurred. The directly launched debug binary lacks the dev CLI websocket endpoint; expected hot-reload connection failures are not application-test failures.

Local reproducible evidence is under ignored `var/browser-evidence/`:

- `baseline/calendar-320.png` and `baseline/measurements.json`.
- `verified/calendar-320.png`, `calendar-1440.png`, `selected-320.png`, `selected-1440.png`.
- `verified/answers-320.png`, `answers-1440.png`, and `measurements.json`.

See the README for commands. Screenshots and database state are not committed.

## Self-review convergence

Round 1 reviewed CSS behavior, served-file provenance, narrow and wide screenshots, focus and selection, and all changed source tests.
The visible radio label replaced a test click on the hidden input, and the response test now waits for Dioxus listener attachment rather than interacting with SSR markup prematurely.
Both-width creation and answer flows passed after those test corrections.

Round 2 reviewed runner ownership, failure cleanup, and gaps in asset and keyboard evidence.
The runner now checks byte-for-byte served CSS against the bundle, uses a bounded readiness request, verifies keyboard scrolling within the answer matrix, and still stops the server if browser shutdown fails.
It recognizes signal-terminated children to avoid waiting forever during cleanup and removes a previous failure screenshot before a new run.
Affected browser and stale-CSS tests were rerun after these corrections.

Final review found no remaining merge-blocking issue within the calendar repair scope.
The test deliberately verifies the hydrated response path; accepting input before hydration is outside this repair and is not claimed as verified.

## Codex review disposition

The first user-facing PR-ready claim preceded the asynchronous Codex review and did not satisfy the user's clarified completion condition.
Local runtime evidence above remains valid; external-review convergence is now separately tracked by Story 0022 and ADR 0030.

| Comment | Decision | Reason and response |
| --- | --- | --- |
| [discussion_r3939384154](https://github.com/velengel/tsunoru/pull/6#discussion_r3939384154), ADR 0021 mixes independent decisions | Change required | ADR 0021 is first introduced to main by this PR, so its older number is not an accepted-history exemption. Limit it to build-output isolation; move date-label policy to ADR 0028 and served-browser evidence to ADR 0029. Refer to existing ADR 0019 for native-button semantics and ADR 0027 for browser-runner/data isolation. |

The ADR correction changed documentation and instructions only. A subsequent review also required the verifier signal-handling correction below; application code remains unchanged.
Document structure, relative links, ID uniqueness, staged diff and secret checks are run for this correction.
The final-head Codex review must complete before another PR-ready claim; elapsed time or absence of new comments is not a substitute.

### Termination-signal follow-up

[discussion_r3939426271](https://github.com/velengel/tsunoru/pull/6#discussion_r3939426271) requires a change: normal `finally` cleanup does not handle direct SIGINT/SIGTERM termination reliably.
The pre-fix shutdown probe failed because SIGTERM did not complete cleanup within 15 seconds; its test reclaimed its own remaining processes and directory.
The verifier now owns SIGINT/SIGTERM handling, shares idempotent cleanup with normal exit, and waits for any in-flight Chromium launch before closing it.
Playwright's own signal handling is disabled for these two signals so the verifier controls its full resource lifetime.
`test-calendar-browser-shutdown.mjs` checks both signals, including the absence of the owned server, Chromium descendants and temporary directory.
The normal both-width flow and stale-CSS check are rerun after this change. SIGKILL and machine failure cannot be cleaned up by an in-process handler.

### Additional workflow and Python-verifier findings

- [discussion_r3939444599](https://github.com/velengel/tsunoru/pull/6#discussion_r3939444599): change required. Worktree reuse and opening an early draft are independent user-requested operating policies. ADR 0031 and ADR 0032 now adopt them separately; AGENTS.md references both.
- [discussion_r3939444603](https://github.com/velengel/tsunoru/pull/6#discussion_r3939444603): change required. The Python verifier had the same direct-SIGTERM gap. Its new regression reproduced `SIGTERM: server leaked` before implementation.

The Python verifier now turns SIGTERM/SIGINT into an exit that unwinds its existing server cleanup and TemporaryDirectory context, and ignores repeated termination signals during cleanup.
The signal regression uses a fresh database initialized by the built server and checks server removal, temporary-data removal and signal-specific exit status for both signals.
It also runs the normal HTTP lifecycle and confirms unchanged source-fixture contents and hash. All three paths passed.
No application Rust or CSS changed during these comment corrections.


### Initialization-order finding

[discussion_r3939464122](https://github.com/velengel/tsunoru/pull/6#discussion_r3939464122): change required. The first signal fix registered handlers after acquiring resources, leaving an early termination window.
The new temporary-directory-stage test failed before the fix because that directory remained after SIGTERM.
Handlers are now registered before acquisition; cleanup awaits an in-flight mkdtemp, socket setup or browser launch, and checkpoints prevent acquiring the next resource after shutdown starts.
The signal regression covers temporary-directory creation, socket setup, server spawn and completed browser launch for both SIGTERM and SIGINT, including signal-specific exit codes and removal of owned processes/data.


### Asynchronous asset-check finding

[discussion_r3939494231](https://github.com/velengel/tsunoru/pull/6#discussion_r3939494231): change required. The synchronous asset-check subprocess could block Node's signal handler while a checker stalled.
A fixed repository fixture reproduced the missed shutdown deadline before implementation.
The checker now runs asynchronously in its own process group, and cleanup terminates both its shell and descendants, escalating only that group after a bounded wait.
The regression now covers five phases, including the stalled checker, with both SIGTERM and SIGINT (10 cases).
Normal browser flow and the stale-CSS negative case are rerun after this correction.

## Pending browser launch follow-up

[Codex discussion_r3939552354](https://github.com/velengel/tsunoru/pull/6#discussion_r3939552354) requires a change: awaiting an unresolved browser launch prevented server and disposable-directory cleanup after a termination signal.
A never-settling launch fixture reproduced the 15-second regression deadline failure before the fix.
Browser cleanup now has a 6-second deadline, and real Playwright launch uses its documented 5-second timeout.
Timeouts exit with failure status 1 after the remaining cleanup; completed cleanup on ordinary signals retains status 130/143.
A slow machine taking over 5 seconds to launch Chromium now fails the verification explicitly and may require revisiting this deadline.

All 14 shutdown cases passed: never-settling launch, an actual Playwright-owned process that never establishes its browser connection, asset checker, temporary-directory acquisition, socket acquisition, server startup and launched browser, each with SIGTERM and SIGINT.
The process fixture uses a dedicated executable that sleeps, and the test observes only descendants of its verifier and the owned server's cwd.
The installed Playwright 1.62.0 process launcher also registers an exit handler for its owned browser processes; this was inspected locally, while the stalled-process cases verify actual process removal.
The API timeout is documented in [Playwright BrowserType.launch](https://playwright.dev/docs/api/class-browsertype#browser-type-launch-option-timeout).

The normal 320px/1440px browser flow, stale-CSS negative control, syntax checks and public-snapshot boundary check passed again after this change.

## Python child publication follow-up

[Codex discussion_r3939580165](https://github.com/velengel/tsunoru/pull/6#discussion_r3939580165) requires a change: SIGINT/SIGTERM could raise SystemExit after Popen created a child but before the verifier assigned its cleanup handle.
The regression wraps the real Popen constructor, announces the owned PID before returning, and waits until a signal is received; the pre-fix run failed with `SIGTERM: server leaked`.
The verifier now defers the Python termination action across child construction and assignment, then processes the recorded signal while the child is reachable by finally.
This avoids changing OS signal masks that a new server could inherit.

All four Python cases passed (publication and ready phases, each with SIGTERM and SIGINT), including server exit, disposable-directory removal and the expected signal exit status.
The same isolated fixture suite passed the normal HTTP lifecycle and unchanged source-database checks; syntax and diff checks also passed.
Self-review confirmed that deferred termination is released on both normal and exceptional exits and that signal handlers are restored when main returns.

## Portable process discovery follow-up

[Codex discussion_r3939600856](https://github.com/velengel/tsunoru/pull/6#discussion_r3939600856) requires a change: the regression inferred ownership from a macOS ps comm path and lsof, and exceptions in its stdout callback escaped promise cleanup.
The verifier now publishes the owned PID and disposable directory at the pending-launch checkpoint, as it already does at other startup checkpoints.
The regression waits for both that notification and the fixture's launch-start marker, parses inside try/catch, and rejects its awaited promise on a notification error.
The failure path collects still-owned descendants before terminating the runner.

The fixed fixtures under `scripts/fixtures/portable-process-tools` allow numeric PID/PPID discovery but reject process-name queries and lsof.
The old ps invocation was confirmed to exit 2 against that fixture; the updated full regression passed all 14 cases under the constrained PATH.
An initial temporary-script reproduction was rejected by automatic approval review; the accepted verification uses these inspectable repository fixtures instead.
This proves removal of those command dependencies on macOS, not execution in a Linux environment, which remains unverified.

Reproduction command from the worktree root:

```sh
PATH="$PWD/scripts/fixtures/portable-process-tools:$PATH" PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node scripts/test-calendar-browser-shutdown.mjs
```

The normal 320px/1440px browser flow, syntax and diff checks passed after the notification change.

## Batched database isolation follow-up

The completed review of 4bd1676 reported [wrong-server writes](https://github.com/velengel/tsunoru/pull/6#discussion_r3939636006) and [source WAL side effects](https://github.com/velengel/tsunoru/pull/6#discussion_r3939636008).
Both require changes because matching assets and mode=ro did not prove the intended data boundary.
They were handled in one batch, including a local cross-check of both browser and HTTP verifiers before another external review.

The wrong-database fixture deliberately runs the same real application bundle against another disposable database, without relying on a timing-sensitive port race.
Before the fixes, browser verification wrote two events to that database and HTTP verification wrote one.
After the fixes, both reject its identity before writing any events.
Each verifier seeds an unpredictable marker directly in its own disposable database, reads it through the existing event API before test writes, and checks child liveness around traffic.
The browser fixture applies current migration SQL and SQLx 0.8 metadata using node:sqlite; normal real-server verification validates the resulting checksums and schema.
See [ADR 0038](../ADR/0038-confirm-verifier-database-identity-before-http-writes.md).

The source snapshot fixture leaves a committed WAL without SHM, including an event that exists only in WAL.
Before the fix, the source file set changed despite successful HTTP verification.
After the fix, the WAL-only event remains readable through the copied database and the original main/WAL/SHM bytes and file presence remain unchanged.
The verifier reads source files as bytes, checks repeated file sets, opens only the disposable snapshot with SQLite, and compares source files again after verification.
This requires a quiescent source and is not an arbitrary live-backup guarantee; see [ADR 0037](../ADR/0037-inspect-source-database-through-byte-snapshots.md) and [SQLite WAL read-only behavior](https://www.sqlite.org/wal.html#read_only_databases).

Batch verification passed: Node shutdown 14 cases under the portable-tool fixture, Python shutdown four cases plus normal HTTP lifecycle, both wrong-database tests, WAL snapshot preservation, 320px/1440px browser flow, stale-CSS negative control, syntax and public-snapshot boundary checks.
Application Rust/CSS did not change in this batch; the previously recorded Rust tests, clippy, fmt and web build remain the application-code evidence.
Self-review checked related failure phases in both verifiers, the source sidecars and WAL-only data, request deadlines, child-liveness guards, migration metadata, and the new documentation's scope.

## Temporary-directory publication follow-up

[Review 3939721392](https://github.com/velengel/tsunoru/pull/6#discussion_r3939721392) requires extending Python termination deferral to temporary-directory creation.
TemporaryDirectory installs its finalizer after mkdtemp returns, so the pre-fix signal test reproduced a leftover directory before the with block could own it.
An ExitStack now owns the cleanup registration before deferred termination is released.
The same termination guard covers both directory and child publication; later snapshot, file and database work stays inside the registered directory cleanup scope.

The expanded Python regression passed six cases: directory creation, child publication and ready state, each with SIGTERM and SIGINT.
Normal HTTP operation, wrong-database rejection and WAL-only source preservation also passed after this change.
The unchanged Node regression remains the separately recorded 14-case evidence.

## Outer-driver termination batch

[Review 3939750034](https://github.com/velengel/tsunoru/pull/6#discussion_r3939750034) reproduced an outer Python driver's SIGTERM leaving its seed server alive. The fix covers the shutdown, identity and snapshot drivers in one batch, including the Node shutdown driver.

Python drivers share a signal-aware resource scope: directory and process publication defer termination until cleanup is registered, children get their own process group, and directory teardown stops the scope's children first. The outer regression runner uses the same scope. The Node driver registers each iteration's cleanup synchronously after spawn, and shares that idempotent cleanup between normal failure and signal handlers. Both request graceful shutdown before bounded force cleanup.

The outer regression validates reported PIDs against the runner's descendants and paths against the owned fixture locations before checking both SIGTERM and SIGINT. All ten cases passed. The existing six Python and fourteen Node inner-interruption cases, normal HTTP lifecycle, both wrong-database controls and WAL-only preservation also passed after the driver changes. Local review checked acquisition publication, imported fixture ownership, process-before-directory ordering, timeout cleanup and the new regression runner itself. SIGKILL and host failure remain outside executable cleanup guarantees; see [ADR 0039](../ADR/0039-scope-regression-harness-resources.md).

## Asset-checker acquisition follow-up

[Review 3939819165](https://github.com/velengel/tsunoru/pull/6#discussion_r3939819165) requires addressing the real shell checker's mktemp/assignment/trap interval. A fixed mktemp fixture pauses after directory creation; terminating the real checker reproduced a leftover directory before the fix. The former stalled-asset fixture did not exercise that implementation.

The existing shell entrypoint now execs a Python implementation that checks HTML, CSS and content type in memory. It creates neither temporary directories nor child processes, so this acquisition gap is removed instead of adding another cleanup protocol. Python 3 is required; connection-refused retries and a five-second socket timeout bound ordinary connection failures. Response memory use scales with the local assets; see [ADR 0040](../ADR/0040-check-calendar-assets-in-memory.md).

The real-checker regression passes six cases: SIGTERM, SIGINT, valid CSS, stale CSS, wrong content type and missing HTML markers. The full 320px/1440px browser flow, browser stale-CSS negative control and fourteen browser shutdown cases also pass. Local review inspected all remaining temporary-directory creation sites in scripts and checked entrypoint compatibility, signal exit status, HTTP errors and the new test driver's owned scope.

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

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

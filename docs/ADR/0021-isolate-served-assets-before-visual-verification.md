# ADR 0021: Isolate candidate build output

## context

The original calendar investigation found current markup served with a stylesheet missing calendar selectors.
Two `dx serve` processes used different ports but shared the same worktree and `target/dx` output.
A different port separates HTTP listeners, but does not separate the artifacts they serve.

This record was preserved on the calendar feature branch and is first introduced to main by PR #6.
Following its Codex review, its independent evidence and layout choices are recorded in ADR 0029 and ADR 0028.
Native calendar-button semantics were already adopted in ADR 0019; the isolated browser runner and disposable test data are covered by ADR 0027.

## decision

Give each concurrently built candidate server its own build-output directory.

## rejected options

- Separate only ports: both build processes can still overwrite the same assets.
- Delete shared old assets before every run: the same shared-writer failure can recur.
- Inline the calendar CSS: hides one symptom without isolating other served artifacts.

## consequences

A dedicated worktree with its default target directory, or an explicit distinct `CARGO_TARGET_DIR`, keeps candidate artifacts separate.
When using concurrent `dx serve` sessions, session caches must also be separate so they cannot restore another candidate's state.
Independent builds consume more disk space and can take longer on first build.
Isolation alone does not prove that a browser used the intended artifacts; ADR 0029 covers that evidence.

References: [Dioxus assets](https://dioxuslabs.com/learn/0.7/essentials/ui/assets/), [Dioxus hot reload](https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload/), [ADR 0019](0019-use-an-inline-month-calendar-with-an-editable-base-time.md), [ADR 0027](0027-run-isolated-browser-regressions.md), [ADR 0028](0028-keep-calendar-date-labels-on-one-line.md), [ADR 0029](0029-verify-ui-through-served-browser-evidence.md).

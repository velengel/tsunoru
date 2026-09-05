# Codex review configuration

Checked against official documentation on 2026-09-05.

## Repository configuration

The root [AGENTS.md](../../AGENTS.md#code-review-rules) contains the review guidance for this repository.
The three rules cover existing authorization/data boundaries, mobile and keyboard scheduling interaction, and ownership of verification resources.
Each names a failure condition and an allowed path; they add no new product behavior.
Formatting and deterministic checks remain in the verification workflow rather than these review rules.
The instruction placement is recorded in [ADR 0033](../ADR/0033-keep-code-review-rules-in-root-agents.md).

## Hosted settings

[OpenAI's setup instructions](https://learn.chatgpt.com/docs/third-party/github#set-up-codex-code-review) place the Code review toggle and Automatic reviews preferences in [Codex code review settings](https://chatgpt.com/codex/settings/code-review).
For a connected repository with Codex cloud configured, a maintainer with the required GitHub permission enables review there and checks the desired automatic trigger preferences.
Adding AGENTS.md supplies review guidance; it does not enable those toggles.
A manual review can be requested with a PR comment containing `@codex review`.

PR #6 has received Codex reviews after draft-to-ready transitions, which establishes that this trigger worked for this PR.
The complete account settings and automatic behavior for every push were not inspected or changed.
If a future review does not start, check the repository toggle and trigger preferences against [official troubleshooting](https://learn.chatgpt.com/docs/third-party/github#troubleshoot-code-review).

## Why these sources and files

[Customize what Codex reviews](https://learn.chatgpt.com/docs/third-party/github#customize-what-codex-reviews) directly documents AGENTS.md discovery, applicable root/nested scope, the Code Review Rules heading, safe exceptions, and starting with two or three rules.
It also states that guidance does not replace tests, branch protections or required approvals.
This is the primary source for the implementation; no undocumented review-specific YAML or TOML file is introduced.

Existing repository references supply the rule content:

- [Ubiquitous language](../ubiquitous-language.md) and [server boundary](../../src/server.rs): public event/answer views are distinct from organizer capabilities, response capabilities and account sessions.
- [Calendar browser verification](0019-calendar-browser-verification.md): 320px date labels, keyboard interaction, intentional matrix scrolling and verifier signal cleanup have concrete regression evidence.
- [Post-migration runtime verification](0018-post-migration-runtime-verification.md): source data is preserved while an isolated copy is used for verification writes.

## Verification boundary

This change only adds instructions and documentation.
Self-review confirmed rule scope and safe exceptions against the references above.
Required Story/ADR sections, unique document IDs, relative links and diff whitespace checks passed.
Model compliance cannot be proven by a text-matching test; final-head review completion and any findings are checked live on PR #6 under [Story 0022](../story/0022-converge-codex-pr-review.md).
A completed review with no findings does not prove that every rule was exercised.

## Local review and grouped follow-up

The user's follow-up adds [the judgment log](../review-judge-logs.md) as a local-review reference.
AGENTS.md requires evidence-backed judgments, commit-linked completion replies and one follow-up review after a batch of related fixes and local checks.
The review instructions ask Codex to report all actionable findings discovered in its pass together and group duplicate causes without dropping independent findings.
This can reduce avoidable serial requests; it cannot guarantee exhaustive model findings in one pass.
See [ADR 0034](../ADR/0034-record-review-judgments-in-repository.md), [ADR 0035](../ADR/0035-reply-to-addressed-review-comments.md) and [ADR 0036](../ADR/0036-batch-review-follow-up.md).

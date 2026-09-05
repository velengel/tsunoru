# ADR 0036: Batch review follow-up

## context

PR #6 repeatedly received one lifecycle finding, fixed it and requested another review before inspecting adjacent lifecycle windows.
The user wants to reduce this serial exchange.
Official [Codex review guidance](https://learn.chatgpt.com/docs/third-party/github#customize-what-codex-reviews) supports repository review instructions, but does not promise exhaustive findings in one pass.

## decision

Process each completed review as one batch of findings and related local checks before requesting another review.

## rejected options

- Request a fresh review after each individual fix: repeats avoidable external waits.
- Assume one batch proves exhaustive coverage: later review may still find defects.
- Suppress repeated topics regardless of evidence: related symptoms may have different causes.

## consequences

Collect summaries and all threads, group shared causes, judge every finding and inspect related files and failure paths locally.
Consult the judgment log, implement and verify the batch, reply with commit links and request one review of the resulting head.
Ask reviewers to report all actionable findings found in their pass together and group duplicates without suppressing independent issues.
Later findings enter the next batch; the final-head completion gate remains unchanged.

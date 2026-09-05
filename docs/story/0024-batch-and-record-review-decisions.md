# Story 0024: Batch and record review decisions

## context

The user requests docs/review-judge-logs.md for this PR and future review decisions, with rationale and links, and requires local reviewers to consult it.
They also request a completion reply with a commit link for each addressed PR comment and fewer one-finding/fix/review cycles.
The latest completed review adds two isolation findings to assess together.

## definition of done

- [ ] Record every PR #6 finding, disposition, reason, verification and commit link in the judgment log.
- [ ] Make local review consult the log while still evaluating current evidence.
- [ ] Reply to addressed review comments with verified pushed commit links.
- [ ] Record and apply a batch review workflow.
- [ ] Fix and verify both new isolation findings and inspect related paths before one new review.
- Final-head Codex review completion and unresolved findings are checked live on PR #6.

## to do

- [ ] Record separate decisions for the judgment log, completion replies and batch workflow.
- [ ] Backfill previous findings and replies; retain later reassessments as history.
- [ ] Add negative isolation tests before implementation and inspect both verifiers.
- [ ] Converge self-review, push the complete batch, then request one review of that head.

## concern

A review prompt cannot guarantee that Codex discovers all defects in one pass.
Previously rejected or fixed findings may need reassessment when new evidence appears.
The log must not be used to suppress fresh findings or to misrepresent an old head's review as current.

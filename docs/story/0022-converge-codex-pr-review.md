# Story 0022: Converge Codex PR review

## context

The user requires Codex review comments to be evaluated and necessary fixes completed before declaring a PR ready.
PR #6's first Codex review identified multiple independent decisions in newly introduced ADR 0021.
Its older number does not make it an accepted main-branch historical record.

## definition of done

- [x] Triage the received Codex comment with evidence and a required-fix or no-change rationale.
- [x] Separate ADR 0021's independent decisions without changing accepted main-branch ADRs.
- [x] Record the complete PR-ready workflow in repository instructions.
- Completion requires the final head's Codex review to finish and necessary feedback to be resolved; verify this live on PR #6.

## to do

- [x] Read the full review, inline comment and applicable ADR policy.
- [x] Decide to address discussion_r3939384154.
- [x] Split decision records, check links and document structure, then self-review.
- After pushing fixes, inspect the subsequent Codex review and converge further findings before delivery. Keep final review status on the PR so recording it does not itself create an unreviewed commit.

## concern

A GitHub ready flag triggers review; it is not proof that asynchronous review has finished.
Reviewing only an old commit or treating silence as approval can miss new comments.
Rejected suggestions need a reason; comments should not be applied mechanically.

## Follow-up finding

Codex discussion_r3939426271 identifies leaked child processes and disposable data when the browser verifier receives SIGINT or SIGTERM.
Change required: reproduce both termination paths, then route signals through shared idempotent cleanup and review the resulting head.

## Third-review findings

- discussion_r3939444599 requires separate ADRs for the user's worktree-reuse and early-draft policies; record ADR 0031 and ADR 0032 before retaining those enforcement rules.
- discussion_r3939444603 identifies the same SIGTERM cleanup gap in the Python HTTP verifier; add a signal regression and unwind its existing cleanup on termination.


## Initialization-order follow-up

Codex discussion_r3939464122 requires signal handlers before the first owned resource is acquired.
The expanded regression reproduced a leftover temporary directory when SIGTERM arrived immediately after mkdtemp.
The handler now precedes resource acquisition, and cleanup awaits pending acquisition before removing it.
Test all four startup phases with SIGTERM and SIGINT, then rerun normal browser and stale-CSS checks before the next review.


## Asset-subprocess follow-up

Codex discussion_r3939494231 requires asynchronous asset checking so a stalled checker cannot block signal cleanup.
Reproduce termination during a stalled checker, then own and terminate its subprocess group from shared cleanup.

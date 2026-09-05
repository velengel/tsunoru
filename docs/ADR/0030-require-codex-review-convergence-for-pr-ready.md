# ADR 0030: Require Codex review convergence for PR ready

## context

The user defines PR ready as a development PR, planning, implementation, self-review and fixes, followed by delivery of a mergeable PR link.
PR #6 was reported ready before its asynchronous Codex review completed, and that review found an actionable documentation issue.
The user explicitly adds review-comment triage and necessary fixes to the completion condition.

## decision

Declare PR ready only after Codex review of the final head has completed and every finding has a verified fix or an evidence-backed no-change disposition.

## rejected options

- Use GitHub's ready flag alone: changing that flag can start a review that has not completed yet.
- Fix every suggestion automatically: findings can be inaccurate or out of scope.
- Accept an older head's clean review: later changes have not been reviewed.

## consequences

Completion can require waiting for external review and another fix-and-review cycle.
The GitHub ready flag may be used to trigger review before the user-facing PR-ready claim.
If review is unavailable or incomplete, report that boundary and do not claim completion.
The disposition record links comments to fixes or reasons, and actionable threads are resolved only after their fixes are pushed and verified.

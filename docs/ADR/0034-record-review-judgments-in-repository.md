# ADR 0034: Record review judgments in the repository

## context

The user wants review judgments, reasons and links retained in this PR and consulted during local review.
Existing verification reports contain useful dispositions but lack a dedicated cross-review entry point.

## decision

Maintain review disposition history in docs/review-judge-logs.md and consult relevant entries during local code review.

## rejected options

- Rely only on GitHub threads: local review cannot reliably discover prior reasoning.
- Treat old judgments as permanent exclusions: new evidence can invalidate them.

## consequences

Each entry records the source finding, decision, rationale, evidence and relevant commits.
Reassessments append history instead of silently overwriting earlier reasoning.
The log adds maintenance work and can become stale, so current code and evidence remain decisive.

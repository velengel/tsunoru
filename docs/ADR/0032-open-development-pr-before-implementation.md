# ADR 0032: Open a development PR before implementation

## context

The user explicitly requires a development PR before planning and implementation, followed by self-review and fix convergence.
The draft provides a stable delivery thread while work is still incomplete; ADR 0030 separately governs final readiness.

## decision

Open a draft development PR before implementing the requested change.

## rejected options

- Open the PR only after implementation: does not satisfy the requested development workflow.
- Open a ready PR immediately: presents incomplete work as ready for review.

## consequences

A feature branch and initial push are needed earlier, and the PR title and description must be updated as the implementation settles.
The draft remains an incomplete-work record until checks and reviews satisfy ADR 0030.
This does not authorize merging or publishing the application.

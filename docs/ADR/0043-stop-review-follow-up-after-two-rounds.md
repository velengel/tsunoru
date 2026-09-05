# ADR 0043: Stop review follow-up after two rounds

## context

PR #6 accumulated sixteen findings and repeated follow-up cycles. Batching related fixes did not bound the total work. The user now prioritizes ending this work for a merge decision and explicitly requires stopping after two rounds.

## decision

Limit autonomous review follow-up to two rounds per PR, then stop and report remaining findings and merge readiness.

## rejected options

- Continue until every new review is clean: allows review work to expand indefinitely.
- Treat every finding as requiring a fix: ignores scope, demonstrated impact and cost.
- Reset the count for each commit or new finding: defeats the limit.

## consequences

One round comprises a received review batch, necessity judgment, any required fixes and verification. At the second round's completion, stop; do not request another review or start a third round without explicit user instruction. Report severe findings immediately without silently overriding the limit. Judge findings by the original goal, concrete impact, reproducibility and correction cost, and record fix, defer or no-change decisions with reasons.

This supersedes the unbounded continuation requirements in ADR 0030 and ADR 0036. Report completed, pending and unreviewed evidence honestly; stopping is not a claim that review passed. The current PR already exceeds the limit and stops now. Merging remains a separate user decision.

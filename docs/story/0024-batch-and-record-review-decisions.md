# Story 0024: Batch and record review decisions

## context

The user requests docs/review-judge-logs.md for this PR and future review decisions, with rationale and links, and requires local reviewers to consult it.
They also request a completion reply with a commit link for each addressed PR comment and fewer one-finding/fix/review cycles.
The latest completed review adds two isolation findings to assess together.

## definition of done

- [x] Record every PR #6 finding, disposition, reason, verification and commit link in the judgment log.
- [x] Make local review consult the log while still evaluating current evidence.
- [x] Reply to addressed review comments with verified pushed commit links.
- [x] Record and apply a batch review workflow.
- [x] Fix and verify both new isolation findings and inspect related paths before one new review.
- Final-head Codex review completion and unresolved findings are checked live on PR #6.

## to do

- [x] Record separate decisions for the judgment log, completion replies and batch workflow.
- [x] Backfill previous findings and replies; retain later reassessments as history.
- [x] Add negative isolation tests before implementation and inspect both verifiers.
- [x] Converge local self-review and verify the full isolation batch.
- Push the completed documentation batch and request one final-head review; keep its live completion state on PR #6.

## concern

A review prompt cannot guarantee that Codex discovers all defects in one pass.
Previously rejected or fixed findings may need reassessment when new evidence appears.
The log must not be used to suppress fresh findings or to misrepresent an old head's review as current.

## Temporary-directory publication follow-up

Review 3939721392 identifies the same ownership-publication gap in TemporaryDirectory construction.
Add directory creation to the signal regression, reuse termination deferral while registering cleanup, and record its judgment and commit-linked reply in the next batch.

## Outer harness termination batch

Review 3939750034 covers the Python shutdown harness, the Node shutdown harness and the three identity/snapshot drivers.
Reproduce cancellation of an outer harness, then centralize owned process/path cleanup in both languages and test both signals across all five drivers before one follow-up review.

- [x] Reproduce an outer SIGTERM leaving the seed server alive.
- [x] Apply signal-aware ownership scopes to all five drivers and to the outer regression runner itself.
- [x] Verify ten outer interruption cases, six Python inner cases, fourteen Node inner cases, normal HTTP, both database identity guards and WAL preservation.
- The commit-linked reply and the resulting final-head review are recorded in the judgment log and checked live on PR #6.

## Asset-checker acquisition follow-up

Review 3939819165 identifies a shell mktemp/assignment/trap gap that the stalled asset fixture does not exercise. Reproduce interruption of the real checker during acquisition, then eliminate its temporary files by checking HTTP responses in memory. Preserve the shell entrypoint and validate normal, stale and interrupted responses before another batched review.

- [x] Reproduce a leftover directory during the real shell checker's acquisition.
- [x] Remove temporary files and subprocesses from the checker implementation.
- [x] Verify both signals, valid/stale CSS, wrong content type, missing HTML markers, the normal browser flow, stale-CSS browser control and fourteen browser shutdown cases.
- Commit and reply evidence is retained in R014; final-head review remains a live PR-ready condition.

## Mutation connection affinity batch

Review 3939865260 identifies a check-to-use gap in both HTTP and browser verification: the port can be rebound after identity verification but before a mutation. Reproduce a listener handover after a valid identity response, bind that response and mutation to one non-reconnecting TCP connection, and validate both clients and the complete browser/HTTP flows before one review.

- [x] Reproduce writes to a replacement listener with separate requests and ordinary reconnecting HTTPConnection behavior.
- [x] Bind each mutation to its verified connection in both clients and reject reconnect attempts.
- [x] Verify zero replacement writes, same-socket success, response/cookie preservation, wrong identity, complete browser and HTTP flows, source preservation and thirty inner/outer interruption cases.
- Record the pushed fix and completion reply in R015, then check the final-head review live.

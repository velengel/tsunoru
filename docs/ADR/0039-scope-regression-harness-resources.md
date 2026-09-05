# ADR 0039: Scope regression harness resources

## context

PR review 3939750034 found that terminating an outer regression driver bypasses its cleanup, even when the inner verifier handles termination. Five drivers share this ownership boundary. The regression reproduces a surviving seed server after SIGTERM.

## decision

Manage regression-driver resources in a signal-aware cleanup scope shared by the Python drivers and applied to each Node test iteration.

## rejected options

- Fix only the reported Python line: the related identity, snapshot and browser drivers have the same failure mode.
- Kill processes by executable name: this could terminate unrelated development servers.

## consequences

The drivers register owned processes and temporary directories before delivering deferred termination, request graceful shutdown before force, and retain bounded cleanup waits. Both signals are tested at the outer-driver boundary. SIGKILL and machine failure cannot execute application cleanup; that limitation remains accepted.

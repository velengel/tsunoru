# ADR 0026: Isolate runtime verification

## context

Migration verification needs writes to establish that event creation and decisions work.
The existing database contains user data, and an automatic reviewer rejected execution of an ad-hoc temporary script while recommending repository-scoped scripts.

## decision

Run runtime verification through a repository-owned script that starts its own loopback server against a disposable SQLite backup.

## rejected options

- Broad command permission changes: scope exceeds this verification task.
- Writes against the original database: creates avoidable cleanup and data risks.
- Tests alone: do not establish that the built HTTP server works.

## consequences

The script and its fixed executable are reviewable before execution; temporary directories contain data only.
The script requires an existing Dioxus web build and local listener permission.
It cannot verify browser interaction, and ephemeral port selection has a small bind race that must abort if the child exits.

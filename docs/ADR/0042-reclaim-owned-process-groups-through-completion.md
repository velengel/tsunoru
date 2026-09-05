# ADR 0042: Reclaim owned process groups through completion

## context

Review 3939917795 reproduces an owned descendant surviving after its group leader exits. A completed Popen or ChildProcess does not establish that its process group has no members. Both regression drivers create dedicated process groups.

## decision

Keep each owned process group's cleanup active until the group has no remaining members, independently of leader exit.

## rejected options

- Check only the leader: descendants can outlive it.
- Search by executable name: that could include unrelated servers.

## consequences

Cleanup signals only groups created for the invocation, waits for graceful completion, then uses bounded KILL fallback. The cleanup registration retires once completed and cannot signal a later group through repeated cleanup. A group that remains after the deadline produces failure. Forced cleanup can discard disposable test state and is limited to owned groups.

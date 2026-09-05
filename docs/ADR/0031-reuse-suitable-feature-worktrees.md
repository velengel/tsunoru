# ADR 0031: Reuse suitable feature worktrees

## context

Calendar work already had a dedicated branch and worktree.
Creating another worktree split the continuation from its existing context without a technical need, and the user explicitly corrected that choice.

## decision

Continue feature work in its existing suitable worktree.

## rejected options

- Always create a new worktree on resumption: duplicates context and creates unnecessary branch and PR cleanup.
- Always reuse regardless of state: can interfere with dirty, in-use or unrelated work.

## consequences

Suitability requires checking the worktree's branch, changes and ownership first.
When a new worktree is necessary, explain the concrete reason and preserve existing work.
This may require extra inspection before implementation resumes.

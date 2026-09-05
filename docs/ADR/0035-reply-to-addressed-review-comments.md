# ADR 0035: Reply to addressed review comments

## context

The user requests an explicit completion reply with a commit link whenever a PR review comment is addressed.
Resolving a thread alone does not show the reviewer where the fix lives.

## decision

Reply to each addressed PR review comment with the verified pushed commit link and a concise description of the fix.

## rejected options

- Resolve without replying: loses the requested fix-to-comment trace.
- Reply before verification or push: the link or completion claim may be premature.

## consequences

Check for an existing reply before posting to avoid duplicates.
Include relevant verification in the reply, then resolve addressed threads.
This adds one acknowledgment per addressed thread but does not trigger a new review per finding.
For a no-change disposition, reply with the evidence-backed reason and judgment-log link rather than claiming a fix.

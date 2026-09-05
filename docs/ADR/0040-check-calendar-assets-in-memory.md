# ADR 0040: Check calendar assets in memory

## context

Review 3939819165 found a signal window between shell mktemp and cleanup registration. The checker only needs the HTTP response text and content type; persisting either response is unnecessary. Python is already required by the verification workflow.

## decision

Check served calendar HTML and CSS in memory through a Python implementation behind the existing shell entrypoint.

## rejected options

- Add another temporary-directory supervisor: this retains disk resources that the check does not require.
- Add shell signal deferral around command substitution: handling signals across the shell and mktemp child is more complex than removing the allocation.

## consequences

The checker creates no temporary directories or child processes. The shell command remains compatible, while its implementation now requires Python 3. Connection-refused retries and marker/content-type checks remain; HTTP reads use a socket timeout. Responses are held in memory, so memory use scales with the local HTML/CSS response size.

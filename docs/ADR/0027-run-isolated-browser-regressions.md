# ADR 0027: Run isolated browser regressions

## context

Story 0015 requires measured browser layout and real interaction evidence.
In-app browser bootstrap fails its trusted-code-path check before browser selection.
Playwright and Chromium are already installed locally; a dedicated test profile avoids the user's browser state.
ADR 0021 requires isolated build outputs; ADR 0029 defines served-browser evidence.

## decision

Verify the calendar through a repository-owned Playwright runner against its own built server and disposable database.

## rejected options

- Change plugin trust or broadly allow commands: exceeds this product task.
- Treat source tests and HTTP checks as browser evidence: cannot prove hydration or geometry.
- Reuse the user's browser profile or original database: unnecessary state coupling.

## consequences

The runner requires process/listener permission and an installed Playwright module supplied explicitly by path.
It launches the fixed built server and fresh Chromium profile, stores screenshots under ignored var and closes owned processes.
It cannot establish screen-reader speech or physical-iPhone behavior.

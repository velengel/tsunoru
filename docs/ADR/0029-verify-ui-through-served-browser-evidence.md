# ADR 0029: Verify UI through served browser evidence

## context

Source CSS assertions passed while an earlier served stylesheet lacked the calendar's selectors.
A screenshot alone cannot prove track geometry, target dimensions or keyboard behavior.
Story 0015 already requires 320px and 1440px layout and calendar-to-answer interaction evidence.

## decision

Base UI verification on the assets and behavior observed in the running browser.

## rejected options

- Source assertions alone: cannot identify a stale served file.
- A successful build alone: does not establish what the browser received or applied.
- Screenshots alone: cannot establish exact geometry or completed interaction.

## consequences

The verification links the live document's stylesheet URL to its response, content type, calendar selectors and corresponding bundle file.
Computed style and geometry provide track, overflow and target measurements; screenshots provide visual review evidence.
Pointer and keyboard actions establish the changed interaction path, including calendar-to-answer completion for Story 0015.
Source tests and builds remain useful separate evidence, and the browser runner is selected in ADR 0027.
Browser automation costs more time than static checks and does not establish physical-device or screen-reader behavior.

References: [Story 0015](../story/0015-repair-and-prove-the-served-calendar-layout.md), [ADR 0027](0027-run-isolated-browser-regressions.md), [browser evidence](../reports/0019-calendar-browser-verification.md).

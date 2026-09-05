# ADR 0028: Keep calendar date labels on one line

## context

The pre-repair 320px calendar had seven columns and no page overflow, but two-digit labels wrapped onto two lines.
Preserving the current seven-column calendar needs enough usable space within each date button.
The measured repair reduces narrow-layout inner padding and gaps, producing 28.05px-wide targets with single-line labels.

## decision

Keep each calendar date label on one line at every supported viewport width.

## rejected options

- Allow numeric labels to wrap: splits a single date into visually separate digits.
- Shrink the entire page: makes controls and text smaller to conceal the spacing problem.
- Change the calendar to fewer columns: breaks the established week representation.

## consequences

At the existing 320px lower bound, inner padding and gaps must leave each day at least 24px wide and high.
Dense calendar days remain narrower than the preferred 44px width; their measured height is 44px.
Viewports below 320px are not covered.
The CSS uses non-wrapping labels and narrow-screen spacing; these remain subject to the browser geometry checks in ADR 0029.

References: [browser evidence](../reports/0019-calendar-browser-verification.md), [WCAG target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html).

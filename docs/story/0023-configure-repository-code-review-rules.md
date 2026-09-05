# Story 0023: Configure repository code review rules

## context

The user requests official-documentation-backed Codex review configuration in PR #6.
The root AGENTS.md already describes development and PR-ready workflow, but has no dedicated Code Review Rules section.

## definition of done

- [x] Identify the supported repository instruction file and separate it from hosted trigger settings.
- [x] Add concise TSUNORU review rules with concrete failure conditions and safe paths.
- [x] Verify references, document structure and consistency with existing product boundaries.
- Final-head Codex review must complete with findings converged under Story 0022; record the live result on PR #6.

## to do

- [x] Fetch the official GitHub review documentation using OpenAI Docs MCP.
- [x] Record the instruction-placement decision before editing AGENTS.md.
- [x] Add rules and an operational reference, then self-review the documentation diff.
- Push and inspect the final-head review; retain its live completion state on PR #6 so recording it does not create an unreviewed commit.

## concern

Repository guidance cannot itself enable hosted review triggers or guarantee findings.
Rules that duplicate lint checks or omit safe exceptions can create noise.
No product behavior or hosted account settings need to change for this task.

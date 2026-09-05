# ADR 0033: Keep code review rules in root AGENTS.md

## context

The user wants repository-level Codex review configuration grounded in official documentation.
[OpenAI's GitHub review documentation](https://learn.chatgpt.com/docs/third-party/github#customize-what-codex-reviews), retrieved on 2026-09-05, supports a Code Review Rules section in applicable AGENTS.md files and recommends starting with two or three concise, consequential checks that include safe paths.
TSUNORU has one application and shared verification tools, so root guidance covers the current scope without duplicated nested files.

## decision

Maintain TSUNORU-specific Codex review guidance in a Code Review Rules section of the root AGENTS.md.

## rejected options

- Add an unreferenced review configuration file: the documented discovery mechanism is AGENTS.md, so an invented filename would not establish that Codex reads it.
- Split these initial rules into nested files: the current shared boundaries do not require separate service-specific guidance.
- Put hosted review toggles in a repository config file: the documented controls live in Codex code review settings.

## consequences

Review guidance is versioned with the code and visible in PR diffs.
The initial rules summarize existing data, interaction and verifier ownership boundaries; they do not introduce new product requirements.
The rules need maintenance when those boundaries change, and guidance does not guarantee detection or replace required checks and approvals.
Hosted trigger preferences remain external state; the operational reference distinguishes configuration instructions from observed PR behavior.

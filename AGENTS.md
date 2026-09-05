# Repository instructions

## Development workflow

- Before implementation, create or update a Story under `docs/story/`.
- Every Story must contain `context`, `definition of done`, `to do`, and `concern` sections derived from the request at hand.
- Before implementing a product, architecture, dependency, tooling, or operating-policy decision, record it under `docs/ADR/`.
- Every ADR must contain `context`, `decision`, `rejected options`, and `consequences`, including accepted risks and disadvantages.
- For ADRs created after ADR 0022, write `decision` as one line that names one adopted decision. Add a supplement only for conditions without which that line would be misread; keep reasons in `context`, alternatives in `rejected options`, and risks in `consequences`.
- Split independent decisions into separate ADRs, and do not rewrite accepted historical ADRs only to match the newer format.
- Add every shared project term to `docs/ubiquitous-language.md` with its meaning, usage, unavoidable system synonym when one exists, and a reference link when available.
- Write the user-observable test first, run it, and confirm the expected failure before writing the implementation that makes it pass.
- Keep Story checkboxes and documentation aligned with the verified implementation state.

## PR ready workflow

- Continue in an existing suitable feature worktree; create a new one only when needed, and explain the reason.
- Open a development draft PR before implementation, then record the plan, implement, and converge self-review and fixes.
- Inspect Codex review summaries, reviews, and inline threads for the final PR head. For every finding, record whether a change is needed and why.
- Apply necessary fixes, verify them, push, and repeat review inspection for the new head until findings converge. Resolve addressed threads only after the fixes are verified and pushed; record evidence-backed reasons for no-change decisions.
- GitHub's ready flag can trigger review; it is not the user-facing completion criterion. Require an explicit completed Codex review for the final head, no pending necessary fixes, successful required checks, and a mergeable PR before delivering the PR-ready link.
- If Codex review is unavailable or still running, report the pending boundary instead of declaring PR ready. Do not infer approval from silence or an older head's review.

## Git and secrets

- On macOS, run `zsh scripts/verify-local-git-materialization.zsh` before `git status`, whole-repository inspection, or worktree creation. If it reports `dataless_files_present`, stop Git operations and use Finder's **Keep Downloaded** on the repository folder.
- Create linked worktrees through `zsh scripts/create-feature-worktree.zsh <feature-branch> <absolute-worktree-path> [start-point]`; choose a destination outside iCloud Drive and every File Provider domain, and do not overlap direct `git worktree add` commands across sessions. When the canonical repository is outside a File Provider domain, prefer its ignored `.codex/worktree/<slug>` directory.
- A command runner session ID means the child process is still running. Poll that session until an exit code is returned. Treat `locked initializing` as worktree creation in progress, and do not unlock, prune, remove, or retry while the creating Git process or file updates continue.
- Before opening Finder for a task, record the existing Finder window IDs, targets, bounds, and detail panels. Prefer command-line inspection when Finder is not required.
- After Finder work, close every Finder window, information panel, detail panel, and sheet created for the task. Restore any reused pre-existing Finder window to its recorded target and bounds, then re-inventory Finder UI after cleanup. Do not quit Finder or close unrelated pre-existing windows.
- Commit at coherent work boundaries and stage only files belonging to that boundary.
- Use a one-line `<prefix>: <summary>` subject followed by `why` and `what` sections in the commit body.
- Use `docs`, `test`, `feat`, `fix`, `refactor`, `chore`, or another short noun that identifies the change type as the prefix.
- Never commit tokens, API keys, passwords, private keys, environment files, service-account files, or real credentials.
- Use placeholders in examples, keep local values in ignored files, and check ignore rules before handling any secret-shaped file.
- Before each commit, inspect `git diff --cached --name-only`, run `git diff --cached --check`, and scan the staged content for common secret patterns.

## Verification

- Run `cargo test --all-targets`, `cargo test --all-targets --features server`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, and `dx build --web` after changing application code.
- Keep one verified server reachable while replacing a running Fullstack candidate. Start the candidate on another port, verify it, then switch.
- Report local tests, build status, Git state, external deployment, and physical-device evidence as separate layers.
- Support completion claims with commands, output summaries, and file paths or line numbers.

## Documentation style and discoveries

- Before writing explanatory Japanese prose, read the [Japanese technical-writing rules](https://gist.github.com/k16shikano/fd287c3133457c4fd8f5601d34aa817d) and the [cognitive-rhythm writing rules](https://gist.github.com/k16shikano/eb2929f13ed19c97188393d297be8432).
- Keep one topic per paragraph, preserve real uncertainty, and remove prose that only announces the document's own progression.
- At the end of implementation work, record previously unknown findings in `docs/reports/surprise-and-discovery.md` and surface them as `Surprise & Discovery` in the final report.
- Keep repository-specific knowledge in this repository.
- When memory feedback is authorized, add only a short search hook and the authoritative repository path to global memory.

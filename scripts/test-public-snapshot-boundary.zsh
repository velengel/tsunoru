#!/bin/zsh
set -euo pipefail

repository_root="${0:A:h:h}"
cd "$repository_root"

fail() {
  print -u2 "public_snapshot_boundary=FAIL reason=$1"
  exit 1
}

required_files=(
  docs/story/0016-show-tsunoru-identity-in-the-browser-tab.md
  docs/story/0017-keep-future-adr-decisions-concise.md
  docs/story/0018-prevent-dataless-git-worktree-stalls.md
  docs/story/0019-migrate-repository-out-of-file-provider.md
  docs/ADR/0022-use-a-gathering-mark-as-the-favicon.md
  docs/ADR/0023-state-one-decision-in-one-line.md
  docs/ADR/0024-require-local-git-materialization-before-worktree-creation.md
  docs/ADR/0025-publish-history-free-repository-snapshot.md
  scripts/verify-local-git-materialization.zsh
  scripts/create-feature-worktree.zsh
  scripts/test-worktree-creation-guard.zsh
)

for required_file in $required_files; do
  [[ -f "$required_file" ]] || fail "missing_file:$required_file"
done

for executable_script in \
  scripts/verify-local-git-materialization.zsh \
  scripts/create-feature-worktree.zsh \
  scripts/test-worktree-creation-guard.zsh; do
  [[ -x "$executable_script" ]] || fail "script_not_executable:$executable_script"
done

for record_directory in docs/story docs/ADR; do
  duplicate_ids="$(find "$record_directory" -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]-*.md' -exec basename {} \; | cut -c1-4 | sort | uniq -d)"
  [[ -z "$duplicate_ids" ]] || fail "duplicate_record_id:$record_directory:$duplicate_ids"
done

grep -Fqx '/.codex/worktree/' .gitignore || fail "missing_repository_worktree_ignore"

tracked_local_state="$(git ls-files -- target var .mydocs '*.sqlite3' '*.sqlite3-wal' '*.sqlite3-shm')"
[[ -z "$tracked_local_state" ]] || fail "local_state_tracked:$tracked_local_state"

print "public_snapshot_boundary=PASS"
print "story_ids=unique"
print "adr_ids=unique"
print "repository_worktree_ignore=present"
print "local_state_tracked=0"

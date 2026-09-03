#!/bin/zsh
set -euo pipefail

(( $# <= 1 )) || {
  print -u2 "usage: zsh scripts/verify-local-git-materialization.zsh [repository-path]"
  exit 2
}

requested_root="${1:-$PWD}"
repository_root="$(git -C "$requested_root" rev-parse --show-toplevel)"
common_git_dir="$(git -C "$repository_root" rev-parse --path-format=absolute --git-common-dir)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  print "local_git_materialization=PASS"
  print "platform=not_applicable"
  print "repository=$repository_root"
  exit 0
fi

object_paths="$(find "$common_git_dir/objects" -type f -flags +dataless -print 2>/dev/null || true)"
worktree_paths="$(find "$repository_root" \
  \( -path "$repository_root/.git" \
    -o -path "$repository_root/target" \
    -o -path "$repository_root/var" \) -prune \
  -o -type f -flags +dataless -print 2>/dev/null || true)"

if [[ -n "$object_paths" || -n "$worktree_paths" ]]; then
  object_count="$(print -r -- "$object_paths" | awk 'NF { count++ } END { print count + 0 }')"
  worktree_count="$(print -r -- "$worktree_paths" | awk 'NF { count++ } END { print count + 0 }')"
  print -u2 "local_git_materialization=BLOCKED"
  print -u2 "reason=dataless_files_present"
  print -u2 "dataless_git_objects=$object_count"
  print -u2 "dataless_worktree_files=$worktree_count"
  print -u2 "recovery=keep_downloaded_required"
  print -u2 "finder_action=Control-click the repository folder in Finder and choose Keep Downloaded"
  if [[ -n "$object_paths" ]]; then
    print -u2 "sample_git_object=${${(f)object_paths}[1]}"
  fi
  if [[ -n "$worktree_paths" ]]; then
    print -u2 "sample_worktree_file=${${(f)worktree_paths}[1]}"
  fi
  exit 75
fi

print "local_git_materialization=PASS"
print "platform=macOS"
print "repository=$repository_root"
print "common_git_dir=$common_git_dir"
print "dataless_git_objects=0"
print "dataless_worktree_files=0"

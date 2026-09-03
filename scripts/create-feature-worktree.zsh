#!/bin/zsh
set -euo pipefail

(( $# == 2 || $# == 3 )) || {
  print -u2 "usage: zsh scripts/create-feature-worktree.zsh <feature-branch> <absolute-worktree-path> [start-point]"
  exit 2
}

branch="$1"
destination="$2"
start_point="${3:-main}"

[[ "$branch" != "main" && "$branch" != "master" && "$branch" != refs/heads/* ]] || {
  print -u2 "worktree_creation=BLOCKED reason=invalid_feature_branch branch=$branch"
  exit 2
}
[[ "$destination" == /* ]] || {
  print -u2 "worktree_creation=BLOCKED reason=destination_must_be_absolute destination=$destination"
  exit 2
}
[[ ! -e "$destination" ]] || {
  print -u2 "worktree_creation=BLOCKED reason=destination_already_exists destination=$destination"
  exit 2
}

repository_root="$(git rev-parse --show-toplevel)"
common_git_dir="$(git -C "$repository_root" rev-parse --path-format=absolute --git-common-dir)"
destination_key="${destination:A}"

is_file_provider_managed() {
  local candidate="$1"
  while [[ ! -e "$candidate" && "$candidate" != "/" ]]; do
    candidate="${candidate:h}"
  done
  while true; do
    if xattr -p com.apple.file-provider-domain-id "$candidate" >/dev/null 2>&1; then
      return 0
    fi
    [[ "$candidate" == "/" ]] && break
    candidate="${candidate:h}"
  done
  return 1
}

if [[ "$(uname -s)" == "Darwin" ]] && is_file_provider_managed "$destination_key"; then
  print -u2 "worktree_creation=BLOCKED reason=destination_file_provider_managed"
  print -u2 "destination=$destination_key"
  print -u2 "recovery=choose an absolute worktree path outside iCloud Drive or another File Provider domain"
  exit 75
fi

lock_dir="$common_git_dir/tsunoru-worktree-create.lock"
owner_file="$lock_dir/owner"
owner_token="$$:$(date +%s):$RANDOM"
lock_owned=no

cleanup_creation_lock() {
  [[ "$lock_owned" == yes ]] || return 0
  local recorded_token=""
  if [[ -r "$owner_file" ]]; then
    recorded_token="$(sed -n 's/^token=//p' "$owner_file" | head -1)"
  fi
  if [[ "$recorded_token" == "$owner_token" ]]; then
    rm -f -- "$owner_file"
    rmdir "$lock_dir" 2>/dev/null || true
  fi
}
trap cleanup_creation_lock EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if ! mkdir "$lock_dir" 2>/dev/null; then
  print -u2 "worktree_creation=BLOCKED reason=another_creation_active"
  print -u2 "lock=$lock_dir"
  if [[ -r "$owner_file" ]]; then
    sed 's/^/active_/' "$owner_file" >&2
  else
    print -u2 "active_owner=initializing"
  fi
  print -u2 "recovery=inspect the owner PID and git worktree list; do not remove the lock while its process is active"
  exit 75
fi
lock_owned=yes
printf '%s\n' \
  "token=$owner_token" \
  "pid=$$" \
  "started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  "cwd=$PWD" \
  "branch=$branch" \
  "destination=$destination" \
  "start_point=$start_point" > "$owner_file"

zsh "${0:A:h}/verify-local-git-materialization.zsh" "$repository_root"

git -C "$repository_root" show-ref --verify --quiet "refs/heads/$branch" && {
  print -u2 "worktree_creation=BLOCKED reason=branch_already_exists branch=$branch"
  exit 2
}

git -C "$repository_root" worktree list --porcelain | awk -v target="$destination_key" '
  /^worktree / && substr($0, 10) == target { found = 1 }
  END { exit(found ? 0 : 1) }
' && {
  print -u2 "worktree_creation=BLOCKED reason=destination_already_registered destination=$destination_key"
  exit 2
}

expected_head="$(git -C "$repository_root" rev-parse --verify "$start_point^{commit}")"
if ! git -C "$repository_root" worktree add --quiet -b "$branch" "$destination" "$expected_head"; then
  print -u2 "worktree_creation=FAIL reason=git_worktree_add_failed"
  print -u2 "branch=$branch"
  print -u2 "destination=$destination"
  print -u2 "expected_HEAD=$expected_head"
  print -u2 "recovery=inspect the destination, branch, git process, and git worktree list before retrying"
  exit 1
fi

created_branch="$(git -C "$destination" branch --show-current)"
created_head="$(git -C "$destination" rev-parse HEAD)"
created_status="$(git -C "$destination" status --porcelain --untracked-files=all)"
created_git_dir="$(git -C "$destination" rev-parse --path-format=absolute --git-dir)"

if [[ "$created_branch" != "$branch" || "$created_head" != "$expected_head" || -n "$created_status" || -e "$created_git_dir/locked" ]]; then
  print -u2 "worktree_creation=FAIL reason=post_create_verification"
  print -u2 "expected_branch=$branch"
  print -u2 "actual_branch=${created_branch:-unavailable}"
  print -u2 "expected_HEAD=$expected_head"
  print -u2 "actual_HEAD=${created_head:-unavailable}"
  print -u2 "status=${created_status:-clean}"
  print -u2 "git_internal_lock=$created_git_dir/locked"
  exit 1
fi

print "worktree_creation=PASS"
print "cwd=$destination"
print "branch=$created_branch"
print "HEAD=$created_head"
print "status=clean"
print "git_internal_lock=absent"

#!/bin/zsh
set -euo pipefail
unsetopt BGNICE

script_dir="${0:A:h}"
verify_script="$script_dir/verify-local-git-materialization.zsh"
create_script="$script_dir/create-feature-worktree.zsh"
agents_file="$script_dir/../AGENTS.md"

fail() {
  print -u2 "worktree_guard_test=FAIL reason=$1"
  exit 1
}

[[ -x "$verify_script" ]] || fail "missing_materialization_preflight"
[[ -x "$create_script" ]] || fail "missing_worktree_creator"
[[ -r "$agents_file" ]] || fail "missing_agents_guidance"

verify_source="$(<"$verify_script")"
[[ "$verify_source" != *'$repository_root/.codex/worktree'* ]] || fail "in_repository_worktree_excluded_from_preflight"

agents_guidance="$(<"$agents_file")"
[[ "$agents_guidance" == *"record the existing Finder window IDs, targets, bounds, and detail panels"* ]] || fail "missing_finder_baseline_rule"
[[ "$agents_guidance" == *"close every Finder window, information panel, detail panel, and sheet created for the task"* ]] || fail "missing_finder_cleanup_rule"
[[ "$agents_guidance" == *"Restore any reused pre-existing Finder window"* ]] || fail "missing_finder_restore_rule"
[[ "$agents_guidance" == *"re-inventory Finder UI after cleanup"* ]] || fail "missing_finder_verification_rule"

fixture_root="$(mktemp -d "/private/tmp/tsunoru-worktree-guard.XXXXXX")"
trap 'rm -rf -- "$fixture_root"' EXIT

repository="$fixture_root/repository"
git init -q -b main "$repository"
git -C "$repository" config user.name "TSUNORU Test"
git -C "$repository" config user.email "tsunoru-test@example.invalid"
printf '%s\n' "fixture" > "$repository/README.md"
git -C "$repository" add README.md
git -C "$repository" commit -q -m "test: create fixture"

clean_output="$(zsh "$verify_script" "$repository")"
[[ "$clean_output" == *"local_git_materialization=PASS"* ]] || fail "local_fixture_did_not_pass"

fake_bin="$fixture_root/fake-bin"
mkdir -p "$fake_bin"
printf '%s\n' '#!/bin/zsh' 'print "/synthetic/dataless-object"' > "$fake_bin/find"
chmod +x "$fake_bin/find"

set +e
blocked_output="$(PATH="$fake_bin:$PATH" zsh "$verify_script" "$repository" 2>&1)"
blocked_status=$?
set -e
[[ $blocked_status -eq 75 ]] || fail "dataless_probe_exit=$blocked_status"
[[ "$blocked_output" == *"local_git_materialization=BLOCKED"* ]] || fail "dataless_probe_message"
[[ "$blocked_output" == *"keep_downloaded_required"* ]] || fail "missing_finder_recovery"

fake_provider_bin="$fixture_root/fake-provider-bin"
mkdir -p "$fake_provider_bin"
printf '%s\n' '#!/bin/zsh' 'exit 0' > "$fake_provider_bin/xattr"
chmod +x "$fake_provider_bin/xattr"

provider_managed_worktree="$fixture_root/provider-managed-worktree"
set +e
provider_output="$(cd "$repository" && PATH="$fake_provider_bin:$PATH" zsh "$create_script" feature/provider-managed "$provider_managed_worktree" main 2>&1)"
provider_status=$?
set -e
[[ $provider_status -eq 75 ]] || fail "provider_managed_destination_exit=$provider_status"
[[ "$provider_output" == *"destination_file_provider_managed"* ]] || fail "provider_managed_destination_message"
[[ ! -e "$provider_managed_worktree" ]] || fail "provider_managed_destination_created"
git -C "$repository" show-ref --verify --quiet refs/heads/feature/provider-managed && fail "provider_managed_branch_created"

printf '%s\n' '#!/bin/zsh' 'sleep 2' > "$repository/.git/hooks/post-checkout"
chmod +x "$repository/.git/hooks/post-checkout"

first_worktree="$fixture_root/first-worktree"
second_worktree="$fixture_root/second-worktree"
first_output="$fixture_root/first.out"
(
  cd "$repository"
  zsh "$create_script" feature/first "$first_worktree" main > "$first_output" 2>&1
) &
first_pid=$!

common_git_dir="$(git -C "$repository" rev-parse --path-format=absolute --git-common-dir)"
creation_lock="$common_git_dir/tsunoru-worktree-create.lock"
for _ in {1..100}; do
  [[ -d "$creation_lock" ]] && break
  sleep 0.05
done
if [[ ! -d "$creation_lock" ]]; then
  wait "$first_pid" 2>/dev/null || true
  [[ -r "$first_output" ]] && print -u2 -- "$(<"$first_output")"
  fail "first_creator_did_not_acquire_lock"
fi

set +e
second_output="$(cd "$repository" && zsh "$create_script" feature/second "$second_worktree" main 2>&1)"
second_status=$?
set -e
[[ $second_status -eq 75 ]] || fail "concurrent_creator_exit=$second_status"
[[ "$second_output" == *"worktree_creation=BLOCKED"* ]] || fail "concurrent_creator_message"
[[ "$second_output" == *"another_creation_active"* ]] || fail "concurrent_creator_reason"

if ! wait "$first_pid"; then
  print -u2 -- "$(<"$first_output")"
  fail "first_creator_failed"
fi
first_result="$(<"$first_output")"
[[ "$first_result" == *"worktree_creation=PASS"* ]] || fail "first_creator_missing_pass"
[[ "$(git -C "$first_worktree" branch --show-current)" == "feature/first" ]] || fail "first_branch_mismatch"
[[ -z "$(git -C "$first_worktree" status --porcelain)" ]] || fail "first_worktree_dirty"
[[ ! -e "$second_worktree" ]] || fail "second_worktree_was_created"
git -C "$repository" show-ref --verify --quiet refs/heads/feature/second && fail "second_branch_was_created"
[[ ! -d "$creation_lock" ]] || fail "creation_lock_remained"

print "worktree_guard_test=PASS"

#!/bin/zsh
set -euo pipefail
pr="${1:?usage: $0 <pr-number-or-url> [interval-seconds] [timeout-seconds>}"
interval="${2:-15}"
timeout="${3:-1800}"
repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
number="$(gh pr view "$pr" --json number --jq .number)"
started="$(date +%s)"
while true; do
  payload="$(gh pr view "$number" --json headRefOid,comments)"
  head="$(print -r -- "$payload" | python3 -c 'import json,sys; print(json.load(sys.stdin)["headRefOid"])')"
  summary="$(print -r -- "$payload" | python3 -c '
import json,sys
for c in json.load(sys.stdin)["comments"]:
    body=c.get("body","")
    if "codex-pull-request-review-summary" in body:
        print(body)
')"
  reviewed="$(print -r -- "$summary" | python3 -c 'import re,sys; m=re.search(r"Commit \| `([0-9a-f]+)`",sys.stdin.read()); print(m.group(1) if m else "")')"
  if [[ "$reviewed" == "$head" ]] && print -r -- "$summary" | rg -q 'Status \| .*Completed'; then
    print -r -- "Codex review completed for head $head"
    print -r -- "$summary"
    print -r -- "\nInline findings and review comments:"
    gh api "repos/$repo/pulls/$number/comments" --paginate --jq '.[] | "- [\(.html_url)] \(.body)"'
    exit 0
  fi
  if (( $(date +%s) - started >= timeout )); then
    print -u2 -- "Timed out waiting for Codex review completion (head $head)"
    exit 2
  fi
  print -r -- "Codex review still running or stale for head $head; next check in ${interval}s"
  sleep "$interval"
done

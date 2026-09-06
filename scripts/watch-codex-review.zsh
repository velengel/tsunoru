#!/bin/zsh
set -euo pipefail

pr="${1:?usage: $0 <pr-number-or-url> [interval-seconds] [timeout-seconds]}"
interval="${2:-15}"
timeout="${3:-1800}"
started="$(date +%s)"

while true; do
  payload="$(gh pr view "$pr" --json headRefOid,comments,reviews)"
  head="$(print -r -- "$payload" | python3 -c 'import json,sys; print(json.load(sys.stdin)["headRefOid"])')"
  summary="$(print -r -- "$payload" | python3 -c '
import json,sys
data=json.load(sys.stdin)
for c in data["comments"]:
    body=c.get("body", "")
    if "codex-pull-request-review-summary" in body:
        print(body)
        break
')"
  if print -r -- "$summary" | rg -q 'Status \| ✅? ?\*\*Completed\*\*|Status.*Completed'; then
    print -r -- "Codex review completed for head $head"
    print -r -- "$summary"
    print -r -- "\nFindings (reviews and comments):"
    print -r -- "$payload" | python3 -c '
import json,sys
data=json.load(sys.stdin)
for review in data["reviews"]:
    print(f"- review {review.get(\"state\")}: {review.get(\"body\", \"\").strip()}")
for comment in data["comments"]:
    body=comment.get("body", "")
    if "codex-pull-request-review-summary" not in body:
        print(f"- comment: {body.strip()}")
'
    exit 0
  fi
  now="$(date +%s)"
  if (( now - started >= timeout )); then
    print -u2 -- "Timed out waiting for Codex review completion (head $head)"
    exit 2
  fi
  print -r -- "Codex review still running for head $head; next check in ${interval}s"
  sleep "$interval"
done

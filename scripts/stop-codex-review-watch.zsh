#!/bin/zsh
set -euo pipefail
pr="${1:?usage: $0 <pr-number-or-url>}"
root="$(git rev-parse --show-toplevel)"
pidfile="$root/.codex-log/codex-review-watch/$pr.pid"
[[ -f "$pidfile" ]] || { print -r -- "not running"; exit 0; }
pid="$(<$pidfile)"
if kill -0 "$pid" 2>/dev/null; then
  command="$(ps -p "$pid" -o command=)"
  [[ "$command" == *"watch-codex-review.zsh $pr"* ]] || { print -u2 -- "refusing to stop unrelated process $pid"; exit 1; }
  kill "$pid"
fi
rm -f "$pidfile"
print -r -- "stopped $pid"

#!/bin/zsh
set -euo pipefail
pr="${1:?usage: $0 <pr-number-or-url> [interval-seconds] [timeout-seconds]}"
interval="${2:-15}"
timeout="${3:-1800}"
root="$(git rev-parse --show-toplevel)"
state="$root/.codex-log/codex-review-watch"
mkdir -p "$state"
pidfile="$state/$pr.pid"
logfile="$state/$pr.log"
if [[ -f "$pidfile" ]] && kill -0 "$(<"$pidfile")" 2>/dev/null; then
  print -r -- "already running pid $(<"$pidfile")"
  exit 0
fi
nohup "$root/scripts/watch-codex-review.zsh" "$pr" "$interval" "$timeout" >>"$logfile" 2>&1 </dev/null &
print $! >| "$pidfile"
print -r -- "started pid $(<$pidfile)"
print -r -- "log $logfile"

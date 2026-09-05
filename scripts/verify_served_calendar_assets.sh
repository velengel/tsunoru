#!/bin/sh
# Keep the existing command while avoiding shell-owned temporary resources.
exec python3 "$(dirname "$0")/verify_calendar_assets.py" "$@"

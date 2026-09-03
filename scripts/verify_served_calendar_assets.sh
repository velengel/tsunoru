#!/bin/sh

set -eu

origin="${1:-http://127.0.0.1:8083}"
origin="${origin%/}"
temporary_directory="$(mktemp -d "${TMPDIR:-/tmp}/tsunoru-calendar-assets.XXXXXX")"

cleanup() {
  rm -rf "$temporary_directory"
}
trap cleanup EXIT HUP INT TERM

html_file="$temporary_directory/root.html"
css_file="$temporary_directory/calendar.css"
headers_file="$temporary_directory/calendar.headers"

curl --retry 15 --retry-connrefused --retry-delay 1 -fsS "$origin/" -o "$html_file"

stylesheet_path="$({
  rg -o 'href="[^"]+\.css"' "$html_file" || true
} | sed -n '1s/^href="\([^"]*\)"$/\1/p')"

if [ -z "$stylesheet_path" ]; then
  echo "FAIL: live HTML did not link a stylesheet" >&2
  exit 1
fi

case "$stylesheet_path" in
  /*) stylesheet_url="${origin}${stylesheet_path}" ;;
  *) stylesheet_url="${origin}/${stylesheet_path}" ;;
esac

curl --retry 15 --retry-connrefused --retry-delay 1 -fsS -D "$headers_file" "$stylesheet_url" -o "$css_file"

if ! rg -iq '^content-type:[[:space:]]*text/css' "$headers_file"; then
  echo "FAIL: linked asset was not served as text/css" >&2
  exit 1
fi

for html_marker in \
  'カレンダーの日を押すと' \
  'candidate-direct-entry'
do
  if ! rg -q "$html_marker" "$html_file"; then
    echo "FAIL: live HTML is missing current marker: $html_marker" >&2
    exit 1
  fi
done

for css_marker in \
  '.candidate-calendar-toolbar' \
  '.candidate-calendar-grid' \
  '.candidate-calendar-day' \
  'grid-template-columns:repeat(7,minmax(0,1fr))'
do
  if ! rg -Fq "$css_marker" "$css_file"; then
    echo "FAIL: linked stylesheet is missing current marker: $css_marker" >&2
    exit 1
  fi
done

echo "PASS: $origin serves current calendar markup and stylesheet"

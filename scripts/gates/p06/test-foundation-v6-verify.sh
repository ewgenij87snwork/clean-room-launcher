#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-gate.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
cp -R "$root/." "$scratch"

expect_refusal() {
  label=$1
  shift
  "$@"
  if P06_GATE_ROOT="$scratch" "$root/scripts/gates/p06/foundation-v6-verify.sh" >/dev/null 2>&1; then
    echo "P06_FOUNDATION_NEGATIVE_ACCEPTED:$label" >&2
    exit 1
  fi
}

expect_refusal subject sh -c 'sed -i.bak "s/\"subject\":\"[0-9a-f]*\"/\"subject\":\"0000000000000000000000000000000000000000\"/" "$1/reports/gates/p06/foundation-v6.json" && rm "$1/reports/gates/p06/foundation-v6.json.bak"' sh "$scratch"
cp -R "$root/." "$scratch"
expect_refusal receipt sh -c 'printf "x" >> "$1/reports/gates/p06/task-3.json"' sh "$scratch"
cp -R "$root/." "$scratch"
expect_refusal output sh -c 'printf "x" >> "$1/reports/gates/p06/outputs/foundation-v6-task-4.txt"' sh "$scratch"
cp -R "$root/." "$scratch"
expect_refusal privacy sh -c 'printf "secret=forbidden\n" >> "$1/reports/gates/p06/outputs/foundation-v6-task-3.txt"' sh "$scratch"

echo P06_FOUNDATION_NEGATIVE_PASS

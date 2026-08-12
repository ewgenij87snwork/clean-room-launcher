#!/bin/sh
set -eu
root=$(cd "$(dirname "$0")/../../.." && pwd -P)
cd "$root"
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --all-targets --locked --offline
scripts/check-public-boundary.sh --root "$root"
evidence=$(mktemp "${TMPDIR:-/tmp}/taskseal-p04-evidence.XXXXXX")
trap 'rm -f "$evidence"' EXIT HUP INT TERM
scripts/gates/p04/emit-acceptance-evidence.sh "$root" > "$evidence"
cmp "$evidence" reports/gates/p04/acceptance-evidence.json
scripts/gates/p04/validate-receipts.sh "$root"
echo P04_CATALOG_GATE_PASS

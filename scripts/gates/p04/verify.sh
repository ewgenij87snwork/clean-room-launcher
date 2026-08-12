#!/bin/sh
set -eu
root=$(cd "$(dirname "$0")/../../.." && pwd -P)
cd "$root"
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --all-targets --locked --offline
scripts/check-public-boundary.sh --root "$root"
for n in 1 2 3 4 5 6 7; do test -f "reports/gates/p04/task-$n.json"; done
echo P04_CATALOG_GATE_PASS

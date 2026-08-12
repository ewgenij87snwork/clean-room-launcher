#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
timeout 120 cargo test --all-targets --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
scripts/check-public-boundary.sh --root .
if rg -n 'Command::new|std::process::Command' src/adapters/{placement,qualification}.rs; then
  echo 'P06_FOUNDATION_PROVIDER_BIRTH_FORBIDDEN' >&2
  exit 2
fi
if ! rg -n 'P06_REQUIRED: provider tuple is not qualified' src/cli/dispatch.rs >/dev/null; then
  echo 'P06_FOUNDATION_PROVIDER_ROUTE_OPEN' >&2
  exit 2
fi
echo P06_FOUNDATION_V6_PASS

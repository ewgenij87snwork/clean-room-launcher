#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd -P)
cd "$root"

jq -e '
  keys == ["command","executable","result","schema_version","status"] and
  .schema_version == "taskseal.local-codex-launch-smoke.v1" and
  .command == "codex -- --help" and .executable == "codex" and
  .result == "accepted" and (.status|test("^exit-[0-9]+$|^unavailable$"))
' reports/gates/p06/local-codex-launch-v1/smoke.json >/dev/null

rustfmt --edition 2024 --check src/cli/mod.rs src/cli/process.rs tests/cli.rs tests/cli/argv_passthrough.rs tests/cli/local_codex_launch.rs fixtures/cli/fake-provider.rs
cargo clippy --all-targets --all-features --locked --offline -- -D warnings
cargo test --lib --test cli --test adapters --locked --offline

rg -n 'Command::new\([^"`]*\)|login|browser|Keychain|credential|access[_-]?token' src/cli/process.rs >/dev/null && {
  echo P06_LOCAL_CODEX_SOURCE_POLICY_FAIL >&2
  exit 2
} || true
rg -n 'Command::new\("codex"\)|LOCAL_CODEX_BOUNDARY_REQUIRED|LOCAL_CODEX_UNAVAILABLE' src/cli/process.rs src/cli/mod.rs >/dev/null

echo P06_LOCAL_CODEX_LAUNCH_GATE_PASS

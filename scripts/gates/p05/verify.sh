#!/bin/sh
set -eu

root=$(cd "$(dirname "$0")/../../.." && pwd -P)
cd "$root"
cargo fmt --check
cargo clippy --all-targets --all-features --locked --offline -- -D warnings
cargo test --all-targets --locked --offline
for task in 1 2 3 4 5 6 7 8; do
  jq -e '.plan == "P05" and .task == '"$task"' and .result == "accepted"' "reports/gates/p05/task-$task.json" >/dev/null
done

tmp_root=$(cd "${TMPDIR:-/tmp}" && pwd -P)
prefix=$(mktemp -d "$tmp_root/taskseal-p05-gate.XXXXXX")
cleanup() {
  case "$(cd "$prefix" && pwd -P)" in
    "$tmp_root"/taskseal-p05-gate.*) rm -rf "$prefix" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
mkdir "$prefix/bin"
cargo build --bins --locked --offline
cp target/debug/taskseal "$prefix/bin/taskseal"
cp target/debug/tseal "$prefix/bin/tseal"
rustc fixtures/cli/fake-provider.rs -o "$prefix/bin/codex"
capture="$prefix/capture"
PATH="$prefix/bin" TASKSEAL_CAPTURE_PATH="$capture" "$prefix/bin/taskseal" codex "two words"
[ "$(cat "$capture")" = "two words" ]
PATH="$prefix/bin" TASKSEAL_CAPTURE_PATH="$capture" "$prefix/bin/tseal" codex "two words"
[ "$(cat "$capture")" = "two words" ]
echo P05_TERMINAL_GATE_PASS

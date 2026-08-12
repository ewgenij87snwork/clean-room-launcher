#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
for task in 3 4; do
  receipt="reports/gates/p06/task-$task.json"
  jq -e --argjson task "$task" '
    keys == ["claim","output","result","schema_version","sources","subject","task"] and
    .schema_version == "taskseal.foundation-receipt.v1" and .task == $task and .result == "accepted" and
    (.subject|test("^[0-9a-f]{40}$")) and (.sources|type == "array" and length == 2 and all(.[]; keys == ["path","sha256"] and (.path|test("^[A-Za-z0-9_./-]+$") and startswith("/")|not) and (.sha256|test("^[0-9a-f]{64}$")))) and (.output|keys == ["path","sha256"] and (.path|test("^outputs/foundation-v6-task-[34]\\.txt$")) and (.sha256|test("^[0-9a-f]{64}$")))
  ' "$receipt" >/dev/null || { echo "P06_FOUNDATION_INVALID_RECEIPT:$task" >&2; exit 2; }
  jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
    actual=$(shasum -a 256 "$path" | awk '{print $1}')
    test "$actual" = "$expected" || { echo "P06_FOUNDATION_SOURCE_MISMATCH:$task:$path" >&2; exit 2; }
  done
  artifact=$(jq -r '.output.path' "$receipt")
  expected=$(jq -r '.output.sha256' "$receipt")
  actual=$(shasum -a 256 "reports/gates/p06/$artifact" | awk '{print $1}')
  test "$actual" = "$expected" || { echo "P06_FOUNDATION_OUTPUT_MISMATCH:$task" >&2; exit 2; }
done
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/' reports/gates/p06/task-[34].json reports/gates/p06/outputs/foundation-v6-task-[34].txt >/dev/null; then
  echo 'P06_FOUNDATION_PRIVACY_REFUSED' >&2
  exit 2
fi
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

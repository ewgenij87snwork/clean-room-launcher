#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
jq -e '
  keys == ["gate","provider_launch","qualification","receipts","result","schema_version","subject"] and
  .schema_version == "taskseal.foundation-v6-report.v1" and (.subject|test("^[0-9a-f]{40}$")) and
  .gate == "foundation-v6-verify.sh" and .result == "FOUNDATION_4_OF_4_CANDIDATE" and
  .provider_launch == false and .qualification == "NOT_QUALIFIED" and
  ([.receipts[].path] == ["task-3.json","task-4.json"]) and all(.receipts[]; keys == ["path","sha256"] and (.sha256|test("^[0-9a-f]{64}$")))
' reports/gates/p06/foundation-v6.json >/dev/null || { echo 'P06_FOUNDATION_REPORT_INVALID' >&2; exit 2; }
subject=$(jq -r '.subject' reports/gates/p06/foundation-v6.json)
git rev-parse --verify "$subject^{commit}" >/dev/null 2>&1 || { echo 'P06_FOUNDATION_SUBJECT_UNKNOWN' >&2; exit 2; }
git merge-base --is-ancestor "$subject" HEAD || { echo 'P06_FOUNDATION_SUBJECT_NOT_ANCESTOR' >&2; exit 2; }
git diff --quiet "$subject" -- src/adapters/placement.rs tests/adapters/placement.rs src/adapters/qualification.rs tests/adapters/qualification.rs || { echo 'P06_FOUNDATION_SOURCE_CHANGED_AFTER_SUBJECT' >&2; exit 2; }
for receipt in task-3.json task-4.json; do
  expected=$(jq -r --arg receipt "$receipt" '.receipts[] | select(.path == $receipt) | .sha256' reports/gates/p06/foundation-v6.json)
  actual=$(shasum -a 256 "reports/gates/p06/$receipt" | awk '{print $1}')
  test "$actual" = "$expected" || { echo "P06_FOUNDATION_REPORT_RECEIPT_MISMATCH:$receipt" >&2; exit 2; }
done
for task in 3 4; do
  receipt="reports/gates/p06/task-$task.json"
  jq -e --argjson task "$task" '
    keys == ["claim","output","result","schema_version","sources","subject","task"] and
    .schema_version == "taskseal.foundation-receipt.v1" and .task == $task and .result == "accepted" and
    (.subject|test("^[0-9a-f]{40}$")) and (.sources|type == "array" and length == 2 and all(.[]; keys == ["path","sha256"] and (.path|test("^[A-Za-z0-9_./-]+$") and startswith("/")|not) and (.sha256|test("^[0-9a-f]{64}$")))) and (.output|keys == ["path","sha256"] and (.path|test("^outputs/foundation-v6-task-[34]\\.txt$")) and (.sha256|test("^[0-9a-f]{64}$")))
  ' "$receipt" >/dev/null || { echo "P06_FOUNDATION_INVALID_RECEIPT:$task" >&2; exit 2; }
  test "$(jq -r '.subject' "$receipt")" = "$subject" || { echo "P06_FOUNDATION_STALE_SUBJECT:$task" >&2; exit 2; }
  if [ "$task" = 3 ]; then
    expected_output='outputs/foundation-v6-task-3.txt'
  else
    expected_output='outputs/foundation-v6-task-4.txt'
  fi
  test "$(jq -r '.output.path' "$receipt")" = "$expected_output" || { echo "P06_FOUNDATION_OUTPUT_PATH_MISMATCH:$task" >&2; exit 2; }
  jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
    actual=$(git show "$subject:$path" | shasum -a 256 | awk '{print $1}')
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

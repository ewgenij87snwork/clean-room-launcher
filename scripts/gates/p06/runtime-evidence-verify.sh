#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
checkpoint='c987f4dff804cbb3c98d8c68b62f38d2a180edd5a6469d3b1de2bfa92ae547fa'
branch='feat/p06-runtime-evidence-v5'
previous='2f7a97f405f3c79793e7c97ad725ff6f6b4ee72e'
for task in 1 2; do
  receipt="reports/gates/p06/task-$task-v5.json"
  jq -e --argjson task "$task" --arg checkpoint "$checkpoint" --arg branch "$branch" '
    .schema_version == "taskseal.task-receipt.v1" and .plan == "P06" and .task == $task and .branch == $branch and .checkpoint_sha256 == $checkpoint and .result == "accepted" and .skips_counted_as_pass == 0 and ([.commands[].phase] == ["red","green","quality"]) and ([.commands[].exit] == [101,0,0]) and (.commands[1].output_sha256|test("^[0-9a-f]{64}$")) and (.commands[2].output_sha256|test("^[0-9a-f]{64}$")) and (.input_head|test("^[0-9a-f]{40}$")) and (.result_head|test("^[0-9a-f]{40}$")) and (.subjects|type == "array" and length > 0) and all(.subjects[]; (.path|test("^[A-Za-z0-9_./-]+$") and startswith("/")|not) and (.sha256|test("^[0-9a-f]{64}$")))
  ' "$receipt" >/dev/null || { echo "INVALID_RECEIPT:$task" >&2; exit 2; }
  head=$(jq -r '.result_head' "$receipt")
  git cat-file -e "$head^{commit}" || { echo "UNKNOWN_RESULT_HEAD:$task" >&2; exit 2; }
  input=$(jq -r '.input_head' "$receipt")
  test "$input" = "$previous" || { echo "BROKEN_RECEIPT_CHAIN:$task" >&2; exit 2; }
  git merge-base --is-ancestor "$input" "$head" || { echo "NON_DESCENDANT_RESULT:$task" >&2; exit 2; }
  jq -r '.subjects[] | [.path, .sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
    git cat-file -e "$head:$path" || { echo "MISSING_SUBJECT:$task:$path" >&2; exit 2; }
    actual=$(git show "$head:$path" | shasum -a 256 | awk '{print $1}')
    test "$actual" = "$expected" || { echo "SUBJECT_DIGEST_MISMATCH:$task:$path" >&2; exit 2; }
    if git diff --quiet "$input" "$head" -- "$path"; then
      echo "UNCHANGED_SUBJECT:$task:$path" >&2
      exit 2
    fi
  done
  previous=$head
done
if rg -n -i 'secret|token|password|/Users/|/home/' reports/gates/p06/task-*-v5.json >/dev/null; then echo "PRIVATE_RECEIPT_VALUE" >&2; exit 2; fi
timeout 120 cargo test --all-targets --locked --offline >/dev/null 2>&1
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings >/dev/null 2>&1
scripts/check-public-boundary.sh --root . >/dev/null 2>&1
echo P06_RUNTIME_EVIDENCE_PASS

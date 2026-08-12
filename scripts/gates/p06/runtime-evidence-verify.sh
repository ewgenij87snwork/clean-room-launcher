#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
checkpoint='c987f4dff804cbb3c98d8c68b62f38d2a180edd5a6469d3b1de2bfa92ae547fa'
branch='feat/p06-runtime-evidence-v5'
receipt_dir="$root/reports/gates/p06"
if [ "$#" -gt 0 ]; then
  [ "$#" -eq 2 ] && [ "$1" = '--receipt-dir' ] || { echo 'USAGE: --receipt-dir DIR' >&2; exit 2; }
  receipt_dir=$2
fi

previous='2f7a97f405f3c79793e7c97ad725ff6f6b4ee72e'
for task in 1 2; do
  receipt="$receipt_dir/task-$task-v5.json"
  jq -e --argjson task "$task" --arg checkpoint "$checkpoint" --arg branch "$branch" '
    keys == ["branch","checkpoint_sha256","commands","input_head","plan","result","result_head","schema_version","skips_counted_as_pass","subjects","task"] and
    .schema_version == "taskseal.task-receipt.v1" and .plan == "P06" and .task == $task and .branch == $branch and .checkpoint_sha256 == $checkpoint and .result == "accepted" and .skips_counted_as_pass == 0 and
    (.input_head|test("^[0-9a-f]{40}$")) and (.result_head|test("^[0-9a-f]{40}$")) and
    (.subjects|type == "array" and length > 0 and ([.[].path]|unique|length) == length and all(.[]; keys == ["path","sha256"] and (.path|test("^[A-Za-z0-9_./-]+$") and startswith("/")|not) and (.sha256|test("^[0-9a-f]{64}$")))) and
    (.commands|type == "array" and length == 3 and ([.[].phase] == ["red","green","quality"]) and ([.[].exit] == [101,0,0]) and all(.[]; keys == ["argv","cwd","exit","output_path","output_sha256","phase"] and (.argv|type == "array" and length > 0 and all(.[]; type == "string" and test("^[A-Za-z0-9_.:/=-]+$"))) and .cwd == "." and (.output_path|test("^outputs/task-[12]-(red|green|quality)\\.txt$")) and (.output_sha256|test("^[0-9a-f]{64}$"))))
  ' "$receipt" >/dev/null || { echo "INVALID_RECEIPT:$task" >&2; exit 2; }
  head=$(jq -r '.result_head' "$receipt")
  input=$(jq -r '.input_head' "$receipt")
  test "$input" = "$previous" || { echo "BROKEN_RECEIPT_CHAIN:$task" >&2; exit 2; }
  git cat-file -e "$head^{commit}" || { echo "UNKNOWN_RESULT_HEAD:$task" >&2; exit 2; }
  git merge-base --is-ancestor "$input" "$head" || { echo "NON_DESCENDANT_RESULT:$task" >&2; exit 2; }
  declared=$(jq -r '.subjects[].path' "$receipt" | LC_ALL=C sort)
  changed=$(git diff --name-only "$input" "$head" | LC_ALL=C sort)
  test "$declared" = "$changed" || { echo "SUBJECT_SET_MISMATCH:$task" >&2; exit 2; }
  jq -r '.subjects[] | [.path, .sha256] | @tsv' "$receipt" | while IFS='	' read -r filepath expected; do
    git cat-file -e "$head:$filepath" || { echo "MISSING_SUBJECT:$task:$filepath" >&2; exit 2; }
    actual=$(git show "$head:$filepath" | shasum -a 256 | awk '{print $1}')
    test "$actual" = "$expected" || { echo "SUBJECT_DIGEST_MISMATCH:$task:$filepath" >&2; exit 2; }
  done
  jq -r '.commands[] | [.output_path, .output_sha256] | @tsv' "$receipt" | while IFS='	' read -r relative expected; do
    artifact="$receipt_dir/$relative"
    test -f "$artifact" || { echo "MISSING_OUTPUT_ARTIFACT:$task:$relative" >&2; exit 2; }
    actual=$(shasum -a 256 "$artifact" | awk '{print $1}')
    test "$actual" = "$expected" || { echo "OUTPUT_DIGEST_MISMATCH:$task:$relative" >&2; exit 2; }
    test "$(LC_ALL=C tr -d '\011\012\015\040-\176' < "$artifact" | wc -c | tr -d ' ')" = 0 || { echo "NON_TEXT_OUTPUT:$task:$relative" >&2; exit 2; }
    if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/|[A-Za-z0-9+/]{32,}={0,2}' "$artifact" >/dev/null; then
      echo "PRIVATE_OUTPUT:$task:$relative" >&2
      exit 2
    fi
  done
  previous=$head
done
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/' "$receipt_dir"/*.json >/dev/null; then
  echo 'PRIVATE_RECEIPT_VALUE' >&2
  exit 2
fi
timeout 120 cargo test --all-targets --locked --offline >/dev/null 2>&1
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings >/dev/null 2>&1
scripts/check-public-boundary.sh --root . >/dev/null 2>&1
echo P06_RUNTIME_EVIDENCE_PASS

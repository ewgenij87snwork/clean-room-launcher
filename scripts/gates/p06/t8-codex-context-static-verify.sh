#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/task-8-static.json
jq -e '
  keys == ["canary_state","exact_tuple_bound","network","output","provider_launch","qualification","result","schema_version","sources","subject","task"] and
  .schema_version == "taskseal.codex-context-static-receipt.v1" and .task == 8 and .result == "partial" and
  .canary_state == "REFUSED" and .exact_tuple_bound == true and .network == "NOT_ATTEMPTED" and
  .provider_launch == false and .qualification == "NOT_QUALIFIED" and
  (.output.path == "outputs/task-8-static-context.txt") and (.output.sha256 | test("^[0-9a-f]{64}$"))
' "$receipt" >/dev/null
subject=$(jq -r .subject "$receipt")
git rev-parse --verify "$subject^{commit}" >/dev/null && git merge-base --is-ancestor "$subject" HEAD
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r source_file expected; do
  test "$(git show "$subject:$source_file" | shasum -a 256 | awk '{print $1}')" = "$expected"
done
test "$(shasum -a 256 reports/gates/p06/outputs/task-8-static-context.txt | awk '{print $1}')" = "$(jq -r .output.sha256 "$receipt")"
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/' "$receipt" reports/gates/p06/outputs/task-8-static-context.txt; then exit 2; fi
timeout 120 cargo test --test adapters codex_context_canaries --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
echo P06_T8_STATIC_REFUSAL_PASS

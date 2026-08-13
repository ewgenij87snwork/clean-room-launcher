#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/task-7.json
jq -e '
  keys == ["cli_no_spawn","network","output","projection_state","provider_launch","qualification","result","schema_version","sources","subject","task"] and
  .schema_version == "taskseal.codex-placement-receipt.v1" and .task == 7 and .result == "accepted" and
  .projection_state == "REFUSED" and .network == "NOT_ATTEMPTED" and .provider_launch == false and .qualification == "NOT_QUALIFIED" and
  .cli_no_spawn == "PASS" and
  ([.sources[].path] == ["src/adapters/codex/mod.rs","src/adapters/codex/placement.rs","src/cli/dispatch.rs","tests/adapters.rs","tests/adapters/codex/placement.rs","tests/cli/argv_passthrough.rs"]) and
  (.output.path == "outputs/task-7-projection-refusal.txt") and (.output.sha256|test("^[0-9a-f]{64}$"))
' "$receipt" >/dev/null
subject=$(jq -r .subject "$receipt")
git rev-parse --verify "$subject^{commit}" >/dev/null && git merge-base --is-ancestor "$subject" HEAD
git diff --quiet "$subject" -- src/adapters/codex/mod.rs src/adapters/codex/placement.rs src/cli/dispatch.rs tests/adapters.rs tests/adapters/codex/placement.rs tests/cli/argv_passthrough.rs
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do test "$(git show "$subject:$path" | shasum -a 256 | awk '{print $1}')" = "$expected"; done
test "$(shasum -a 256 reports/gates/p06/outputs/task-7-projection-refusal.txt | awk '{print $1}')" = "$(jq -r .output.sha256 "$receipt")"
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/' "$receipt" reports/gates/p06/outputs/task-7-projection-refusal.txt; then exit 2; fi
timeout 120 cargo test --test adapters codex_placement --locked --offline
timeout 120 cargo test --test cli argv_passthrough::unqualified_provider_route_refuses_before_ambient_path_can_spawn --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
rg -n 'P06_REQUIRED: provider tuple is not qualified' src/cli/dispatch.rs >/dev/null
echo P06_T7_PLACEMENT_PASS

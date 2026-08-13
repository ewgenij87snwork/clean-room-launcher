#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/task-8.json
jq -e '
  keys == ["attempts","binary_unchanged","forbidden_ambient_observed","product_route","progression","qualification","raw_output_retained","required_layers_observed","result","schema_version","sources","subject","task","temporary_auth_removed","tuple","user_auth_config_unchanged","worktree_unchanged"] and
  .schema_version == "taskseal.codex-context-negative-receipt.v1" and .task == 8 and .result == "sealed_negative" and
  .tuple == {"provider_id":"codex","artifact_digest":"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37","version":[0,147,0],"os":"macos","arch":"aarch64"} and
  ([.sources[].path] == ["src/adapters/codex/mod.rs","src/adapters/codex/context_canaries.rs","tests/adapters.rs","tests/adapters/codex/context_canaries.rs","fixtures/adapters/codex/context-canaries/native/codex-home/AGENTS.md","fixtures/adapters/codex/context-canaries/native/project/AGENTS.md","fixtures/adapters/codex/context-canaries/native/project/task/AGENTS.md","fixtures/adapters/codex/context-canaries/native/ambient-home/AGENTS.md","fixtures/adapters/codex/context-canaries/native/output-schema.json","src/cli/dispatch.rs"]) and
  (.attempts | length == 2) and
  .attempts[0].id == 1 and .attempts[0].state == "OS_SANDBOX_NETWORK_DENIED" and .attempts[0].provider_response == false and
  .attempts[1].id == 2 and .attempts[1].state == "UNAVAILABLE" and .attempts[1].permission_profile == "custom-read-only-network-enabled-no-writable-roots-v1" and .attempts[1].provider_response == false and
  .required_layers_observed == "0/3" and .forbidden_ambient_observed == false and
  .binary_unchanged == true and .user_auth_config_unchanged == true and .worktree_unchanged == true and
  .temporary_auth_removed == true and .raw_output_retained == false and
  .product_route == "P06_REQUIRED" and .qualification == "NOT_QUALIFIED" and .progression == "STOPPED_AT_T8"
' "$receipt" >/dev/null
subject=$(jq -r .subject "$receipt")
git rev-parse --verify "$subject^{commit}" >/dev/null && git merge-base --is-ancestor "$subject" HEAD
sources=$(jq -r '.sources[].path' "$receipt")
git diff --quiet "$subject" -- $sources
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r source_file expected; do
  test "$(git show "$subject:$source_file" | shasum -a 256 | awk '{print $1}')" = "$expected"
done
jq -r '.attempts[].output | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r output_path expected; do
  test "$(shasum -a 256 "reports/gates/p06/$output_path" | awk '{print $1}')" = "$expected"
done
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/|prompt=' "$receipt" reports/gates/p06/outputs/task-8-native-attempt-1.txt reports/gates/p06/outputs/task-8-native-attempt-2.txt; then exit 2; fi
command=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
command=$(realpath "$command")
before=$(shasum -a 256 "$command" | awk '{print $1}')
test "$before" = "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"
test "$(env -i PATH=/usr/bin:/bin "$command" --version)" = "codex-cli 0.147.0"
test "$(shasum -a 256 "$command" | awk '{print $1}')" = "$before"
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64
timeout 120 cargo test --test adapters codex_context_canaries --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
rg -n 'P06_REQUIRED: provider tuple is not qualified' src/cli/dispatch.rs >/dev/null
echo P06_T8_NEGATIVE_RECEIPT_PASS

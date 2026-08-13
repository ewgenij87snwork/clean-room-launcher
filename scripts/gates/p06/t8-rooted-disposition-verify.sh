#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/task-8-rooted-disposition.json
jq -e '
  .schema_version == "taskseal.codex-context-rooted-disposition.v1" and .task == 8 and .result == "sealed_negative" and
  .tuple == {"provider_id":"codex","artifact_digest":"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37","version":[0,147,0],"os":"macos","arch":"aarch64"} and
  .historical_receipt.path == "task-8.json" and
  .root_discovery.state == "L0_L2_L3" and .root_discovery.forbidden_ambient == false and .root_discovery.native_evidence == false and
  .rooted_attempt.process_exit == 1 and .rooted_attempt.native_observation == "UNAVAILABLE" and
  .rooted_attempt.provider_response == "ABSENT" and .rooted_attempt.result == "ABSENT" and
  .rooted_attempt.native_required_layers == "UNKNOWN" and .rooted_attempt.native_forbidden_ambient == "UNKNOWN" and
  .rooted_attempt.authority == {"kind":"T8_GIT_ROOT_PROVIDER_NATIVE","uses":1} and
  .rooted_attempt.binary_unchanged == true and .rooted_attempt.user_auth_config_unchanged == true and
  .rooted_attempt.worktree_unchanged == true and .rooted_attempt.temporary_auth_removed == true and .rooted_attempt.raw_output_retained == false and
  ([.sources[].path] == ["scripts/gates/p06/t8-native-observe-once.sh","scripts/gates/p06/t8-root-discovery-verify.sh","scripts/gates/p06/test-t8-native-sandbox-profile.sh","src/cli/dispatch.rs"]) and
  .product_route == "P06_REQUIRED" and .qualification == "NOT_QUALIFIED" and .progression == "STOPPED_AT_T8"
' "$receipt" >/dev/null
subject=$(jq -r .subject "$receipt")
test "$subject" = "f459f9187094e572a46799d0c1d4f673e19620bc"
git rev-parse --verify "$subject^{commit}" >/dev/null && git merge-base --is-ancestor "$subject" HEAD
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r source_file expected; do
  test "$(git show "$subject:$source_file" | shasum -a 256 | awk '{print $1}')" = "$expected"
done
test "$(shasum -a 256 reports/gates/p06/task-8.json | awk '{print $1}')" = "$(jq -r .historical_receipt.sha256 "$receipt")"
test "$(shasum -a 256 reports/gates/p06/outputs/task-8-root-discovery.txt | awk '{print $1}')" = "$(jq -r .root_discovery.output.sha256 "$receipt")"
test "$(shasum -a 256 reports/gates/p06/outputs/task-8-rooted-attempt.txt | awk '{print $1}')" = "$(jq -r .rooted_attempt.output.sha256 "$receipt")"
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/|prompt=' "$receipt" reports/gates/p06/outputs/task-8-rooted-attempt.txt; then exit 2; fi
P06_CODEX_BIN=${P06_CODEX_BIN:?P06_CODEX_BIN is required} scripts/gates/p06/t8-root-discovery-verify.sh >/dev/null
scripts/gates/p06/test-t8-native-sandbox-profile.sh >/dev/null
rg -n 'P06_REQUIRED: provider tuple is not qualified' src/cli/dispatch.rs >/dev/null
echo P06_T8_ROOTED_DISPOSITION_PASS

#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/native-skill-canaries-v1/task-9.json
jq -e '
  .schema_version == "taskseal.codex-native-skill-canaries-receipt.v1" and
  .task == 9 and .result == "sealed_negative" and .qualification == "NOT_QUALIFIED" and
  .tuple == {"provider_id":"codex","artifact_digest":"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37","version":[0,147,0],"os":"macos","arch":"aarch64"} and
  (.canaries | map(.case) == ["needed","unused","protected","outside_boundary","tampered"]) and
  (.canaries | map(.status) == ["NOT_OBSERVED","NOT_OBSERVED","REFUSED","REFUSED","REFUSED"]) and
  .native_processes == 0 and .network == "NOT_ATTEMPTED" and .raw_output_retained == false and
  .body_bytes_read_or_injected == false and .all_admitted_names_accounted == true and
  (.refusal_reason | length > 20) and ([.sources[].sha256] | all(test("^[0-9a-f]{64}$")))
' "$receipt" >/dev/null
subject=$(jq -r .subject "$receipt")
git rev-parse --verify "$subject^{commit}" >/dev/null
git merge-base --is-ancestor "$subject" HEAD
git diff --quiet "$subject" -- tests/adapters.rs tests/adapters/codex/skill_canaries.rs fixtures/adapters/codex/skill-canaries scripts/gates/p06/native-skill-canaries-v1 reports/gates/p06/native-skill-canaries-v1
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
  test "$(shasum -a 256 "$path" | awk '{print $1}')" = "$expected"
  test "$(git show "$subject:$path" | shasum -a 256 | awk '{print $1}')" = "$expected"
done
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/|prompt=|transcript' "$receipt" fixtures/adapters/codex/skill-canaries; then exit 2; fi
timeout 120 cargo test --test adapters codex_skill_canaries --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
echo P06_T9_NATIVE_SKILL_CANARIES_NOT_QUALIFIED

#!/bin/sh
set -eu
root=${P06_GATE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd -P)}
cd "$(CDPATH= cd -- "$root" && pwd -P)"
receipt=reports/gates/p06/codex/macos-aarch64-0.147.0.json
qualification=qualification/codex/macos-aarch64-0.147.0.json
expected_version=${P06_CODEX_EXPECTED_VERSION:-0.147.0}
test "$expected_version" = "0.147.0" || { echo P06_CODEX_EXPECTED_VERSION_MISMATCH >&2; exit 2; }
jq -e '
  .schema_version == "taskseal.codex-tuple-seal.v1" and .task == 12 and
  .result == "sealed" and .qualification == "QUALIFIED" and
  .tuple == {"provider_id":"codex","artifact_digest":"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37","version":[0,147,0],"os":"macos","arch":"aarch64"} and
  .tasks == [5,6,7,8,9,10,11] and .native_processes >= 1 and
  .raw_output_retained == false and .body_bytes_read_or_injected == false and
  .protected_state_unchanged == true and (.refusal_reason|length > 20) and
  (.subject|test("^[0-9a-f]{40}$"))
' "$receipt" >/dev/null
jq -e '
  .schema_version == "taskseal.qualification-receipt.v1" and
  .provider_id == "codex" and .version == [0,147,0] and .os == "macos" and .arch == "aarch64" and
  .qualification == "QUALIFIED" and .native_processes >= 1 and .raw_output_retained == false and
  .source_tasks == [9,10,11] and .required_tasks == [5,6,7,8,9,10,11] and
  (.reason|length > 20) and (.expiry|test("^2026-08-15T00:00:00Z$"))
' "$qualification" >/dev/null
for task in 5 6 7 8 9 10 11; do
  file="reports/gates/p06/task-$task.json"
  [ "$task" = 9 ] && file=reports/gates/p06/native-skill-canaries-v1/task-9.json
  test -f "$file" || { echo "P06_CODEX_MISSING_TASK:$task" >&2; exit 2; }
  jq -e --argjson task "$task" '.task == $task and (.result == "accepted" or .result == "sealed") and .qualification == "QUALIFIED"' "$file" >/dev/null || { echo "P06_CODEX_TASK_INVALID:$task" >&2; exit 2; }
done
git rev-parse --verify "$(jq -r .subject "$receipt")^{commit}" >/dev/null
git merge-base --is-ancestor "$(jq -r .subject "$receipt")" HEAD
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/|prompt=|transcript' "$receipt" "$qualification" reports/gates/p06/task-{5,6,7,8,10,11}.json reports/gates/p06/native-skill-canaries-v1/task-9.json; then
  echo P06_CODEX_PRIVACY_REFUSED >&2
  exit 2
fi
echo P06_CODEX_T12_SEALED_NOT_QUALIFIED

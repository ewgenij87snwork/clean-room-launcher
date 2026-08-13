#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
default_receipt="$root/reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json"
receipt=${P06_PHASE2_RECEIPT:-$default_receipt}
base=d3c753458c32dc7bc4105f1deddf35d8d43fb5d2
subject=a22ea823f4ed7c50f28ded65b39185de415ea6e8

test "$(pwd -P)" = "$root"
test "$(git -C "$root" rev-parse --show-toplevel)" = "$root"
test "$(git -C "$root" branch --show-current)" = feat/p06-codex-observation-capability-v1-phase2
head=$(git -C "$root" rev-parse HEAD)
git -C "$root" merge-base --is-ancestor "$base" "$head"
git -C "$root" merge-base --is-ancestor "$subject" "$head"
jq -e --arg root "$root" --arg head "$head" '
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2" and
  .repository_realpath == $root and .worktree_realpath == $root and
  .branch == "feat/p06-codex-observation-capability-v1-phase2" and .head == $head and
  .observation_authority.model_processes == 1 and .observation_authority.model_process_timeout_seconds == 120 and
  .observation_authority.intrinsic_provider_requests_and_retries == "included" and .observation_authority.retries == 0
' "$root/.taskseal-dev/execution-authority.json" >/dev/null
git -C "$root" diff --name-only "$base..$head" | "$root/scripts/gates/p06/successors/observation-capability-v1-phase2/validate-write-set.sh"
test -f "$root/.taskseal-dev/phase2-login-used"
test ! -e "$root/.taskseal-dev/phase2-model-used"

if test "$receipt" != "$default_receipt"; then
  receipt=$(realpath "$receipt")
  case "$receipt" in /private/tmp/taskseal-p06-phase2-verify.*/*.json) ;; *) exit 2 ;; esac
fi
jq -e --arg base "$base" --arg subject "$subject" '
  keys == ["base","controls","evidence","historical","observation","phase","plan_id","product","result","schema_version","sources","subject","tuple"] and
  .schema_version == "taskseal.p06-codex-observation-capability-v1-phase2.disposition.v1" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2" and .phase == 2 and .result == "sealed_negative" and
  .base == $base and .subject == $subject and
  .tuple == {provider_id:"codex",version:[0,147,0],os:"macos",arch:"aarch64",executable_sha256:"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"} and
  .observation == {credential_transport:"ANONYMOUS_STDIN",extract_result:"EXTRACT_PREVALIDATED",login_counter:"CONSUMED",login_process_exit:1,login_result:"LOGIN_REFUSED",model_counter:"UNUSED",model_process_exit:"NOT_STARTED",native_observation:"NOT_STARTED",root_discovery:"NOT_RUN",forbidden_ambient_observed:"UNKNOWN",result_sha256:"ABSENT"} and
  .controls == {keychain_access:"DENIED",credential_retained:false,binary_unchanged:true,protected_state_unchanged:true,worktree_unchanged:true,temporary_runtime_removed:true,raw_output_removed:true} and
  .product == {qualification:"NOT_QUALIFIED",original_progression:"STOPPED_AT_T8",original_tasks_9_13:"FORBIDDEN",next:"WAITING_FOR_OWNER_DISPOSITION"} and
  .historical == {phase_1_path:"reports/gates/p06/successors/observation-capability-v1/phase-1.json",phase_1_sha256:"7e0db9d70c99976e7666d6a9a4ecc903462d821620f8d862cd397a50c2136115",t8_path:"reports/gates/p06/task-8-rooted-disposition.json",t8_sha256:"250bd5a4d1591213ebd4ea145991572da5a0356aed777e179a74ae25db899d18"} and
  .sources == [
    {path:"scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh",sha256:"568a73855e26c836d70cdf739b0818b3b421291d1a5fdd76ad029ad6cc3c6497"},
    {path:"scripts/gates/p06/successors/observation-capability-v1-phase2/test-run-once.sh",sha256:"b3a05d40729c2924cb01e84495ec4229ac7d7a38506618210ca5afb6247d188f"},
    {path:"scripts/gates/p06/successors/observation-capability-v1-phase2/validate-write-set.sh",sha256:"af4165bf87bfee8b9adc92fed2e6815c3e7614dda85614a5216852252ab52d63"}
  ] and
  .evidence == {preflight_path:"reports/gates/p06/successors/observation-capability-v1-phase2/outputs/preflight.txt",preflight_sha256:"94c0c0172cf8fae3b52add679f328307820b67641ae9350cd3ec6f37b0a8f7f3",observation_path:"reports/gates/p06/successors/observation-capability-v1-phase2/outputs/observation.txt",observation_sha256:"547fccefdd2a26bde33c363d3efc3172a1cb7ebf425c865b872275e00933ee6d"}
' "$receipt" >/dev/null

jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
  test "$(git -C "$root" show "$subject:$path" | shasum -a 256 | awk '{print $1}')" = "$expected"
done
for kind in preflight observation; do
  path=$(jq -r ".evidence.${kind}_path" "$receipt")
  expected=$(jq -r ".evidence.${kind}_sha256" "$receipt")
  test "$(shasum -a 256 "$root/$path" | awk '{print $1}')" = "$expected"
done
for kind in phase_1 t8; do
  path=$(jq -r ".historical.${kind}_path" "$receipt")
  expected=$(jq -r ".historical.${kind}_sha256" "$receipt")
  test "$(shasum -a 256 "$root/$path" | awk '{print $1}')" = "$expected"
done
if rg -n 'Bearer |sk-[A-Za-z0-9]{20,}|eyJ[A-Za-z0-9_-]{20,}|TASKSEAL_PHASE2_SYNTHETIC_CREDENTIAL|/Users/ysorokin/\.codex/auth\.json' "$receipt" "$root/$(jq -r .evidence.observation_path "$receipt")"; then exit 2; fi
echo P06_CODEX_OBSERVATION_CAPABILITY_V1_PHASE2_PASS

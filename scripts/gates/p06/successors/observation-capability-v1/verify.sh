#!/bin/sh
set -eu
umask 077
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
default_receipt="$root/reports/gates/p06/successors/observation-capability-v1/phase-1.json"
receipt=${P06_CAPABILITY_RECEIPT:-$default_receipt}
codex_bin=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
predecessor=707622eaf5e1543e34341be4eed152dbfe3ae5c5
subject=a2315ea5b6b22f4ada0c253f5a513be80bb3ef67

test "$(pwd -P)" = "$root"
test "$(git -C "$root" rev-parse --show-toplevel)" = "$root"
test "$(git -C "$root" branch --show-current)" = feat/p06-codex-observation-capability-v1
current_head=$(git -C "$root" rev-parse HEAD)
git -C "$root" merge-base --is-ancestor "$predecessor" "$current_head"
git -C "$root" merge-base --is-ancestor "$subject" "$current_head"
jq -e --arg root "$root" --arg head "$current_head" '
  .schema_version == "taskseal.execution-authority.v2" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1" and
  .repository_realpath == $root and .worktree_realpath == $root and
  .branch == "feat/p06-codex-observation-capability-v1" and .head == $head
' "$root/.taskseal-dev/execution-authority.json" >/dev/null

git -C "$root" diff --name-only "$predecessor..$current_head" | \
  "$root/scripts/gates/p06/successors/observation-capability-v1/validate-write-set.sh"

if test "$receipt" != "$default_receipt"; then
  receipt=$(realpath "$receipt")
  case "$receipt" in
    /private/tmp/taskseal-p06-capability-verify.*/*.json) ;;
    *) echo P06_CAPABILITY_RECEIPT_OVERRIDE_REFUSED >&2; exit 2 ;;
  esac
fi

jq -e --arg predecessor "$predecessor" --arg subject "$subject" '
  keys == ["controls","diagnosis","evidence","phase","plan_id","predecessor","product","result","schema_version","sources","subject","tuple"] and
  .schema_version == "taskseal.p06-codex-observation-capability-v1.phase1.v1" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1" and .phase == 1 and .result == "pass" and
  .predecessor == $predecessor and .subject == $subject and
  .tuple == {provider_id:"codex",version:[0,147,0],os:"macos",arch:"aarch64",executable_sha256:"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"} and
  .diagnosis == {classification:"CAPABILITY_IDENTIFIED",transport_id:"CODEX_ACCESS_TOKEN_STDIN",selection_surface:"CODEX_LOGIN_WITH_ACCESS_TOKEN",evidence_kind:"LOCAL_CLI_HELP",selection_evidence_sha256:"df1abe69f07d364d5ca1d1c075f8c30f2f050b98ac18da2b404374cec1fb9ead",material_difference:"STDIN_TOKEN_SELECTION_NOT_COPIED_STORED_AUTH"} and
  .controls == {network_denied:true,provider_route_started:false,owner_auth_read:false,owner_config_mutated:false,temporary_home_removed:true,raw_output_removed:true} and
  .product == {qualification:"NOT_QUALIFIED",original_progression:"STOPPED_AT_T8",original_tasks_9_13:"FORBIDDEN",phase_2:"WAITING_FOR_OWNER"} and
  .sources == [
    {path:"scripts/gates/p06/successors/observation-capability-v1/probe-local.sh",sha256:"d741ad3860c572e2c086a824efdc727a16a56feff840677211064aaba556d15f"},
    {path:"scripts/gates/p06/successors/observation-capability-v1/test-probe-local.sh",sha256:"734e632e714fa9e37dba89631fcffdcc217ab6f7bdc5e1aa4076ea700739c211"}
  ] and
  .evidence == {task_1_output_path:"reports/gates/p06/successors/observation-capability-v1/outputs/task-1-boundary.txt",task_1_output_sha256:"652f91678ebee476d8f0ba036ab72d825483b3b888fee8b8de35735b7dd20cc2",output_path:"reports/gates/p06/successors/observation-capability-v1/outputs/task-2-capability.txt",output_sha256:"7c0b4d35091cb2cc077e97ebe9492ace0390327dc17c8f484c543b2b5090fa50"}
' "$receipt" >/dev/null

jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
  test "$(git -C "$root" show "$subject:$path" | shasum -a 256 | awk '{print $1}')" = "$expected"
done
output_path=$(jq -r .evidence.output_path "$receipt")
test "$(shasum -a 256 "$root/$output_path" | awk '{print $1}')" = "$(jq -r .evidence.output_sha256 "$receipt")"
task_1_output_path=$(jq -r .evidence.task_1_output_path "$receipt")
test "$(shasum -a 256 "$root/$task_1_output_path" | awk '{print $1}')" = "$(jq -r .evidence.task_1_output_sha256 "$receipt")"

temporary_root=$(mktemp -d /tmp/taskseal-p06-capability-gate.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-capability-gate.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
P06_CODEX_BIN="$codex_bin" "$root/scripts/gates/p06/successors/observation-capability-v1/probe-local.sh" >"$temporary_root/current.txt"
cmp "$temporary_root/current.txt" "$root/$output_path"
if rg -n 'TASKSEAL_SYNTHETIC_|(^|[^A-Za-z0-9])sk-[A-Za-z0-9]{20,}|Bearer [A-Za-z0-9]{20,}|/Users/ysorokin/\.codex' "$receipt" "$root/$output_path"; then
  exit 2
fi
cleanup
trap - EXIT HUP INT TERM
test ! -e "$temporary_root"

echo P06_CODEX_OBSERVATION_CAPABILITY_V1_PHASE1_PASS

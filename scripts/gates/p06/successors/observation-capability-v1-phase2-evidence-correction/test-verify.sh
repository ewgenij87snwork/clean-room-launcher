#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
verify="$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh"
correction="$root/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/correction.json"
task_1="$root/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-1.json"
phase_2="$root/reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json"
synthetic="$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/synthetic-boundary.sh"

temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-correction-verify.XXXXXX)
temporary_root=$(realpath "$temporary_root")
phase_2_temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-verify.XXXXXX)
phase_2_temporary_root=$(realpath "$phase_2_temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-phase2-correction-verify.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
  case "$phase_2_temporary_root" in
    /private/tmp/taskseal-p06-phase2-verify.*) rm -rf -- "$phase_2_temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

run_bootstrap_gate() {
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap "$verify" "$@"
}

run_bootstrap_gate >/dev/null

correction_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$correction" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
    P06_PHASE2_CORRECTION_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}

phase_2_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$phase_2_temporary_root/$name.json"
  jq "$filter" "$phase_2" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
    P06_PHASE2_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}

task_1_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$task_1" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
    P06_PHASE2_CORRECTION_TASK_1_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}

phase_2_mutation_must_fail privacy '.controls.credential_retained=true'
phase_2_mutation_must_fail protected_state '.controls.protected_state_unchanged=false'
phase_2_mutation_must_fail classification '.product.qualification="QUALIFIED"'
correction_mutation_must_fail historical_drift '.historical[0].sha256="0000000000000000000000000000000000000000000000000000000000000000"'
correction_mutation_must_fail task_1_digest '.task_1.receipt_sha256="0000000000000000000000000000000000000000000000000000000000000000"'
task_1_mutation_must_fail receipt_binding '.binding.implementation_result_head="0000000000000000000000000000000000000000"'

printf '%s\n' reports/gates/p06/task-8-rooted-disposition.json >"$temporary_root/changed-paths.txt"
set +e
P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
  P06_PHASE2_CORRECTION_CHANGED_PATHS_FILE="$temporary_root/changed-paths.txt" \
  "$verify" >/dev/null 2>&1
write_set_status=$?
set -e
test "$write_set_status" -ne 0

set +e
model_counter_output=$("$synthetic" \
  --tuple synthetic-provider@0.0.0/darwin/arm64 \
  --base 73d48ffbed1794c6691ba59be006aa096dcfcb22 \
  --authority P06-PHASE2-CORRECTION-SYNTHETIC-AUTHORITY-V1 \
  --login-counter 0 --model-counter 1 \
  --credential-source fixture://synthetic/credential \
  --credential-field .synthetic.access_token \
  --output P06_SYNTHETIC_SAFE_OUTPUT --keychain DENIED \
  --read-policy fixture-only --write-policy none --cleanup removed \
  --successor observation-capability-v1-phase2-evidence-correction 2>&1)
model_counter_status=$?
set -e
test "$model_counter_status" = 1
test "$model_counter_output" = P06_SYNTHETIC_REFUSAL_REUSED_COUNTER

printf '%s\n' P06_PHASE2_CORRECTION_MUTATIONS_PASS

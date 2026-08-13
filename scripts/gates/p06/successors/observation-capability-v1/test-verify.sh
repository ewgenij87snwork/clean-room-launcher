#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
verify="$root/scripts/gates/p06/successors/observation-capability-v1/verify.sh"
write_set_check="$root/scripts/gates/p06/successors/observation-capability-v1/validate-write-set.sh"
receipt="$root/reports/gates/p06/successors/observation-capability-v1/phase-1.json"
temporary_root=$(mktemp -d /tmp/taskseal-p06-capability-verify.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-capability-verify.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

P06_CODEX_BIN=${P06_CODEX_BIN:-/Users/ysorokin/.local/bin/codex} "$verify" >/dev/null

printf '%s\n' \
  'scripts/gates/p06/successors/observation-capability-v1/verify.sh' \
  'reports/gates/p06/successors/observation-capability-v1/phase-1.json' | "$write_set_check"
set +e
printf '%s\n' 'reports/gates/p06/task-8.json' | "$write_set_check" >/dev/null 2>&1
forbidden_diff_status=$?
set -e
test "$forbidden_diff_status" -ne 0

mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$receipt" >"$mutated"
  set +e
  P06_CODEX_BIN=${P06_CODEX_BIN:-/Users/ysorokin/.local/bin/codex} \
    P06_CAPABILITY_RECEIPT="$mutated" "$verify" >/dev/null 2>&1
  status=$?
  set -e
  test "$status" -ne 0
}

mutation_must_fail tuple '.tuple.version=[0,146,1]'
mutation_must_fail predecessor '.predecessor="0000000000000000000000000000000000000000"'
mutation_must_fail classification '.diagnosis.classification="QUALIFIED"'
mutation_must_fail transport '.diagnosis.transport_id="COPIED_STORED_AUTH"'
mutation_must_fail network '.controls.network_denied=false'
mutation_must_fail owner_auth '.controls.owner_auth_read=true'
mutation_must_fail output_digest '.evidence.output_sha256="0000000000000000000000000000000000000000000000000000000000000000"'
mutation_must_fail task_1_digest '.evidence.task_1_output_sha256="0000000000000000000000000000000000000000000000000000000000000000"'
mutation_must_fail source_digest '.sources[0].sha256="0000000000000000000000000000000000000000000000000000000000000000"'

echo P06_CODEX_OBSERVATION_CAPABILITY_V1_MUTATIONS_PASS

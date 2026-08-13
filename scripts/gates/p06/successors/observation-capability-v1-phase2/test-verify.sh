#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
verify="$root/scripts/gates/p06/successors/observation-capability-v1-phase2/verify.sh"
receipt="$root/reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json"
temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-verify.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-phase2-verify.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

"$verify" >/dev/null
mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$receipt" >"$mutated"
  set +e
  P06_PHASE2_RECEIPT="$mutated" "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}
mutation_must_fail tuple '.tuple.version=[0,146,1]'
mutation_must_fail base '.base="0000000000000000000000000000000000000000"'
mutation_must_fail login '.observation.login_result="LOGIN_ACCEPTED"'
mutation_must_fail model '.observation.model_counter="CONSUMED"'
mutation_must_fail qualification '.product.qualification="QUALIFIED"'
mutation_must_fail cleanup '.controls.raw_output_removed=false'
mutation_must_fail source '.sources[0].sha256="0000000000000000000000000000000000000000000000000000000000000000"'
mutation_must_fail output '.evidence.observation_sha256="0000000000000000000000000000000000000000000000000000000000000000"'
echo P06_PHASE2_DISPOSITION_MUTATIONS_PASS

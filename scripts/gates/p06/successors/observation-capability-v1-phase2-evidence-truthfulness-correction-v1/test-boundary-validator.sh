#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
validator="$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/boundary-validator.sh"
historical_runner="$root/scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh"
temporary_root=$(mktemp -d /tmp/taskseal-p06-truth-boundary-test.XXXXXX)

cleanup() {
  case "$temporary_root" in
    /tmp/taskseal-p06-truth-boundary-test.*|/private/tmp/taskseal-p06-truth-boundary-test.*)
      rm -rf -- "$temporary_root"
      ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

# Execute the real runner artifact against a disposable sibling validator. The
# required binary variable stops execution before git, auth, sandbox, Keychain,
# network or provider paths can run.
fixture_root="$temporary_root/import-fixture"
fixture_runner_dir="$fixture_root/scripts/gates/p06/successors/observation-capability-v1-phase2"
fixture_validator_dir="$fixture_root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1"
mkdir -p "$fixture_runner_dir" "$fixture_validator_dir"
cp "$historical_runner" "$fixture_runner_dir/run-once.sh"
printf '%s\n' 'printf "%s\n" P06_BOUNDARY_SHARED_VALIDATOR_IMPORTED' >"$fixture_validator_dir/boundary-validator.sh"
set +e
env -u P06_CODEX_BIN sh "$fixture_runner_dir/run-once.sh" --preflight >"$temporary_root/import.out" 2>"$temporary_root/import.err"
import_status=$?
set -e
test "$import_status" -ne 0
if test "$(cat "$temporary_root/import.out")" != P06_BOUNDARY_SHARED_VALIDATOR_IMPORTED; then
  printf '%s\n' P06_BOUNDARY_RED_RUNNER_DID_NOT_IMPORT_SHARED_VALIDATOR
  exit 1
fi

# shellcheck source=/dev/null
. "$validator"

expect_refusal() {
  expected=$1
  shift
  set +e
  actual=$("$@" 2>&1)
  status=$?
  set -e
  test "$status" = 1
  test "$actual" = "$expected"
}

expected_digest=19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37
p06_boundary_validate_tuple_platform "$expected_digest" Darwin arm64
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_TUPLE \
  p06_boundary_validate_tuple_platform 0000000000000000000000000000000000000000000000000000000000000000 Darwin arm64
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_TUPLE \
  p06_boundary_validate_tuple_platform "$expected_digest" Linux arm64
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_TUPLE \
  p06_boundary_validate_tuple_platform "$expected_digest" Darwin x86_64
p06_boundary_validate_tuple_version 'codex-cli 0.147.0'
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_TUPLE \
  p06_boundary_validate_tuple_version 'invented-cli 0.0.0'

fixture_root_path=/invented/taskseal-phase2
fixture_head=1111111111111111111111111111111111111111
p06_boundary_validate_base \
  "$fixture_root_path" "$fixture_root_path" "$fixture_root_path" \
  feat/p06-codex-observation-capability-v1-phase2 true
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_BASE \
  p06_boundary_validate_base /invented/wrong "$fixture_root_path" "$fixture_root_path" \
  feat/p06-codex-observation-capability-v1-phase2 true
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_BASE \
  p06_boundary_validate_base "$fixture_root_path" "$fixture_root_path" "$fixture_root_path" \
  feat/invented-wrong true
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_BASE \
  p06_boundary_validate_base "$fixture_root_path" "$fixture_root_path" "$fixture_root_path" \
  feat/p06-codex-observation-capability-v1-phase2 false

fixture_authority=$(jq -nc \
  --arg root "$fixture_root_path" --arg head "$fixture_head" '
  {
    schema_version:"taskseal.execution-authority.v2",
    plan_id:"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2",
    repository_realpath:$root,
    worktree_realpath:$root,
    branch:"feat/p06-codex-observation-capability-v1-phase2",
    head:$head,
    observation_authority:{
      id:"P06-CODEX-OBS-CAP-V1-PH2-D3C7534-ONE",
      credential_source:"/Users/ysorokin/.codex/auth.json",
      credential_field:".tokens.access_token",
      login_invocations:1,
      model_processes:1,
      model_process_timeout_seconds:120,
      intrinsic_provider_requests_and_retries:"included",
      retries:0
    }
  }')
p06_boundary_validate_authority "$fixture_authority" "$fixture_root_path" "$fixture_head"
wrong_authority=$(printf '%s\n' "$fixture_authority" | jq -c '.observation_authority.retries=1')
expect_refusal P06_BOUNDARY_REFUSAL_WRONG_AUTHORITY \
  p06_boundary_validate_authority "$wrong_authority" "$fixture_root_path" "$fixture_head"

p06_boundary_validate_counter_state UNUSED UNUSED
expect_refusal P06_BOUNDARY_REFUSAL_REUSED_COUNTER \
  p06_boundary_validate_counter_state CONSUMED UNUSED
expect_refusal P06_BOUNDARY_REFUSAL_REUSED_COUNTER \
  p06_boundary_validate_counter_state UNUSED CONSUMED

p06_boundary_validate_source_field /Users/ysorokin/.codex/auth.json .tokens.access_token
expect_refusal P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_SOURCE \
  p06_boundary_validate_source_field fixture://invented/alternate .tokens.access_token
expect_refusal P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_FIELD \
  p06_boundary_validate_source_field /Users/ysorokin/.codex/auth.json .invented.access_token

profile_base='(version 1)
(deny default)
(allow file-write* (subpath "/private/tmp/taskseal-p06-phase2-runtime.INVENTED"))'
offline_profile="$profile_base
(deny network*)
(deny mach-lookup)"
extract_profile="$profile_base
(allow file-read* (literal \"/Users/ysorokin/.codex/auth.json\"))
(deny network*)
(deny mach-lookup)"
online_profile="$profile_base
(allow network-outbound)
(deny file-read* (subpath \"/Users/ysorokin/Library/Keychains\") (subpath \"/Library/Keychains\") (subpath \"/System/Library/Keychains\"))
(deny mach-lookup (global-name-regex #\"^com\\.apple\\.(security|SecurityAgent)\"))"
keychain_profile="$profile_base
(deny network*)
(deny file-read* (subpath \"/Users/ysorokin/Library/Keychains\") (subpath \"/Library/Keychains\") (subpath \"/System/Library/Keychains\"))
(deny mach-lookup (global-name-regex #\"^com\\.apple\\.(security|SecurityAgent)\"))"
p06_boundary_validate_policy \
  "$offline_profile" "$extract_profile" "$online_profile" "$keychain_profile" \
  /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$online_profile" "$extract_profile" "$online_profile" "$keychain_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$offline_profile" "$offline_profile" "$online_profile" "$keychain_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$offline_profile" "$extract_profile" "$offline_profile" "$keychain_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$offline_profile" "$extract_profile" "$online_profile" "$online_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json

p06_boundary_validate_output \
  EXTRACT_PREVALIDATED 1 LOGIN_REFUSED false NOT_STARTED NOT_STARTED \
  NOT_RUN UNKNOWN ABSENT true true true
expect_refusal P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT \
  p06_boundary_validate_output \
    EXTRACT_PREVALIDATED 1 LOGIN_ACCEPTED false NOT_STARTED NOT_STARTED \
    NOT_RUN UNKNOWN ABSENT true true true
expect_refusal P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT \
  p06_boundary_validate_output \
    EXTRACT_PREVALIDATED 1 LOGIN_REFUSED false NOT_STARTED NOT_STARTED \
    NOT_RUN UNKNOWN 'Bearer invented-credential' true true true
expect_refusal P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT \
  p06_boundary_validate_output \
    EXTRACT_PREVALIDATED 1 LOGIN_REFUSED false NOT_STARTED NOT_STARTED \
    NOT_RUN UNKNOWN ABSENT maybe true true

p06_boundary_validate_cleanup /private/tmp/taskseal-p06-phase2-runtime.INVENTED removed
expect_refusal P06_BOUNDARY_REFUSAL_CLEANUP_FAILED \
  p06_boundary_validate_cleanup /private/tmp/invented-wrong removed
expect_refusal P06_BOUNDARY_REFUSAL_CLEANUP_FAILED \
  p06_boundary_validate_cleanup /private/tmp/taskseal-p06-phase2-runtime.INVENTED present

allowed_paths='scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh
reports/gates/p06/successors/observation-capability-v1-phase2/outputs/invented.txt'
p06_boundary_validate_write_set "$allowed_paths"
expect_refusal P06_PHASE2_WRITE_SET_REFUSED:reports/gates/p06/task-8.json \
  p06_boundary_validate_write_set reports/gates/p06/task-8.json

printf '%s\n' P06_PHASE2_TRUTH_BOUNDARY_VALIDATOR_PASS

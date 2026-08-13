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

provider_runner_path=scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh
provider_baseline_object=c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71

provider_source() {
  provider_object=$1
  provider_destination=$2
  if test "$provider_object" = WORKTREE; then
    cp "$root/$provider_runner_path" "$provider_destination"
  else
    git -C "$root" show "$provider_object:$provider_runner_path" >"$provider_destination"
  fi
}

extract_provider_login_region() {
  awk '
    /^# Validate the exact field type and single-line\/nonempty shape before consuming the login counter\.$/ { capture=1 }
    capture { print }
    /^extract_result=EXTRACT_PREVALIDATED$/ { exit }
  ' "$1" >"$2"
}

extract_provider_model_region() {
  awk '
    /^    if test "\$marker_status" = 0; then$/ { capture=1 }
    capture { print }
    /^      model_status=\$\?$/ { exit }
  ' "$1" >"$2"
}

run_provider_equivalence() {
  provider_current_object=$1
  baseline_source="$temporary_root/provider-baseline.sh"
  current_source="$temporary_root/provider-current.sh"
  baseline_login="$temporary_root/provider-baseline-login.txt"
  current_login="$temporary_root/provider-current-login.txt"
  normalized_login="$temporary_root/provider-current-login.normalized.txt"
  baseline_model="$temporary_root/provider-baseline-model.txt"
  current_model="$temporary_root/provider-current-model.txt"

  provider_source "$provider_baseline_object" "$baseline_source"
  provider_source "$provider_current_object" "$current_source"
  extract_provider_login_region "$baseline_source" "$baseline_login"
  extract_provider_login_region "$current_source" "$current_login"
  # Normalize the literal variable use in runner source.
  # shellcheck disable=SC2016
  sed 's/-extract "$credential_plutil_field" raw/-extract tokens.access_token raw/g' \
    "$current_login" >"$normalized_login"
  extract_provider_model_region "$baseline_source" "$baseline_model"
  extract_provider_model_region "$current_source" "$current_model"
  cmp "$baseline_login" "$normalized_login"
  cmp "$baseline_model" "$current_model"
  printf 'P06_PROVIDER_LOGIN_SEMANTICS_EQUIVALENT:%s\n' "$(shasum -a 256 "$normalized_login" | awk '{print $1}')"
  printf 'P06_PROVIDER_MODEL_BLOCK_UNCHANGED:%s\n' "$(shasum -a 256 "$current_model" | awk '{print $1}')"
}

if test "${P06_BOUNDARY_TEST_CASE:-}" = provider-equivalence; then
  run_provider_equivalence "${P06_BOUNDARY_PROVIDER_CURRENT_OBJECT:?P06_BOUNDARY_PROVIDER_CURRENT_OBJECT is required}"
  exit 0
fi

# Execute the real runner artifact against invented command and validator
# fixtures. An unconditional guard inserted immediately before clean_offline
# stops the disposable copy even if a validator call is removed, so the probe
# can never reach sandbox, auth, Keychain, network or provider paths.
fixture_root="$temporary_root/call-path-fixture"
fixture_runner_dir="$fixture_root/scripts/gates/p06/successors/observation-capability-v1-phase2"
fixture_validator_dir="$fixture_root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1"
fixture_bin="$fixture_root/bin"
fixture_trace="$fixture_root/validator-calls.txt"
mkdir -p "$fixture_runner_dir" "$fixture_validator_dir" "$fixture_bin" "$fixture_root/.taskseal-dev"
cp "$historical_runner" "$fixture_runner_dir/run-once.sh"
awk '
  /^clean_offline\(\) \{/ { print "exit 73" }
  { print }
' "$fixture_runner_dir/run-once.sh" >"$fixture_runner_dir/run-once.guarded.sh"
mv "$fixture_runner_dir/run-once.guarded.sh" "$fixture_runner_dir/run-once.sh"
cat >"$fixture_validator_dir/boundary-validator.sh" <<'EOF'
p06_fixture_trace() { printf '%s\n' "$1" >>"$P06_BOUNDARY_CALL_TRACE"; }
p06_boundary_validate_base() { p06_fixture_trace base; }
p06_boundary_validate_authority() { p06_fixture_trace authority; }
p06_boundary_validate_counter_state() { p06_fixture_trace counter_state; }
p06_boundary_validate_source_field() { p06_fixture_trace source_field; }
p06_boundary_validate_tuple_platform() { p06_fixture_trace tuple_platform; }
p06_boundary_validate_write_set() { p06_fixture_trace write_set; }
p06_boundary_validate_policy() { p06_fixture_trace policy; }
p06_boundary_validate_tuple_version() { p06_fixture_trace tuple_version; }
p06_boundary_validate_cleanup() { p06_fixture_trace cleanup; }
p06_boundary_validate_output() { p06_fixture_trace output; }
EOF
cat >"$fixture_bin/git" <<'EOF'
#!/bin/sh
case "$*" in
  *"rev-parse --show-toplevel") printf '%s\n' "$P06_BOUNDARY_FIXTURE_ROOT" ;;
  *"branch --show-current") printf '%s\n' feat/p06-codex-observation-capability-v1-phase2 ;;
  *"rev-parse HEAD") printf '%s\n' 1111111111111111111111111111111111111111 ;;
  *"merge-base --is-ancestor"*) exit 0 ;;
  *"diff --name-only"*) printf '%s\n' scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh ;;
  *) printf 'P06_BOUNDARY_UNEXPECTED_GIT:%s\n' "$*" >&2; exit 97 ;;
esac
EOF
cat >"$fixture_bin/invented-codex" <<'EOF'
#!/bin/sh
printf '%s\n' P06_BOUNDARY_FORBIDDEN_FAKE_COMMAND_EXECUTION >&2
exit 98
EOF
chmod +x "$fixture_bin/git" "$fixture_bin/invented-codex"
printf '%s\n' '{}' >"$fixture_root/.taskseal-dev/execution-authority.json"
set +e
(CDPATH='' cd -- "$fixture_root" &&
  P06_BOUNDARY_CALL_TRACE="$fixture_trace" \
  P06_BOUNDARY_FIXTURE_ROOT="$fixture_root" \
  P06_CODEX_BIN="$fixture_bin/invented-codex" \
  PATH="$fixture_bin:/usr/bin:/bin" \
  sh "$fixture_runner_dir/run-once.sh" --preflight) \
  >"$temporary_root/call-path.out" 2>"$temporary_root/call-path.err"
call_path_status=$?
set -e
expected_calls='base
authority
counter_state
source_field
tuple_platform
write_set
policy'
if test "$call_path_status" != 73 || test "$(cat "$fixture_trace" 2>/dev/null)" != "$expected_calls"; then
  printf '%s\n' P06_BOUNDARY_RED_RUNNER_CALL_PATH_NOT_PROVEN
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
  if test "$status" != 1 || test "$actual" != "$expected"; then
    printf 'P06_BOUNDARY_EXPECTED_REFUSAL_MISSING:%s\n' "$expected"
    exit 1
  fi
}

expect_acceptance() {
  acceptance_id=$1
  shift
  set +e
  acceptance_output=$("$@" 2>&1)
  acceptance_status=$?
  set -e
  if test "$acceptance_status" != 0 || test -n "$acceptance_output"; then
    printf 'P06_BOUNDARY_EXPECTED_ACCEPTANCE_MISSING:%s\n' "$acceptance_id"
    exit 1
  fi
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

expect_acceptance P06_BOUNDARY_SOURCE_FIELD_BINDING \
  p06_boundary_validate_source_field \
    /Users/ysorokin/.codex/auth.json .tokens.access_token tokens.access_token
expect_refusal P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_SOURCE \
  p06_boundary_validate_source_field \
    fixture://invented/alternate .tokens.access_token tokens.access_token
expect_refusal P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_FIELD \
  p06_boundary_validate_source_field \
    /Users/ysorokin/.codex/auth.json .invented.access_token invented.access_token
expect_refusal P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_FIELD \
  p06_boundary_validate_source_field \
    /Users/ysorokin/.codex/auth.json .tokens.access_token invented.access_token
field_use_count=$(rg -F -- '-extract "$credential_plutil_field" raw -expect string -n -o -' "$historical_runner" | wc -l | tr -d ' ')
if test "$field_use_count" != 2 || rg -F -- '-extract tokens.access_token raw -expect string -n -o -' "$historical_runner" >/dev/null; then
  printf '%s\n' P06_BOUNDARY_RED_CREDENTIAL_FIELD_NOT_BOUND_TO_EXTRACTION
  exit 1
fi

profile_base='(version 1)
(deny default)
(import "system.sb")
(allow file-read-metadata (subpath "/private"))
(allow file-read* (subpath "/private/tmp/taskseal-p06-phase2-runtime.INVENTED") (literal "/invented/codex") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))
(allow process*)
(allow sysctl-read)
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
  /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$online_profile" "$extract_profile" "$online_profile" "$keychain_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$offline_profile" "$offline_profile" "$online_profile" "$keychain_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$offline_profile" "$extract_profile" "$offline_profile" "$keychain_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$offline_profile" "$extract_profile" "$online_profile" "$online_profile" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex
broad_read_offline="$offline_profile
(allow file-read* (subpath \"/\"))"
broad_read_extract="$extract_profile
(allow file-read* (subpath \"/\"))"
broad_read_online="$online_profile
(allow file-read* (subpath \"/\"))"
broad_read_keychain="$keychain_profile
(allow file-read* (subpath \"/\"))"
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$broad_read_offline" "$broad_read_extract" "$broad_read_online" "$broad_read_keychain" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex
broad_write_offline="$offline_profile
(allow file-write* (subpath \"/\"))"
broad_write_extract="$extract_profile
(allow file-write* (subpath \"/\"))"
broad_write_online="$online_profile
(allow file-write* (subpath \"/\"))"
broad_write_keychain="$keychain_profile
(allow file-write* (subpath \"/\"))"
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$broad_write_offline" "$broad_write_extract" "$broad_write_online" "$broad_write_keychain" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex

replace_first_read_grant() {
  printf '%s\n' "$1" | awk '
    !replaced && index($0, "(allow file-read*") == 1 {
      print "(allow file-read* (subpath \"/Users\"))"
      replaced=1
      next
    }
    { print }
  '
}
count_preserving_read_offline=$(replace_first_read_grant "$offline_profile")
count_preserving_read_extract=$(replace_first_read_grant "$extract_profile")
count_preserving_read_online=$(replace_first_read_grant "$online_profile")
count_preserving_read_keychain=$(replace_first_read_grant "$keychain_profile")
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$count_preserving_read_offline" "$count_preserving_read_extract" \
    "$count_preserving_read_online" "$count_preserving_read_keychain" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex

replace_write_grant() {
  printf '%s\n' "$1" | awk '
    index($0, "(allow file-write*") == 1 {
      print "(allow file-write* (subpath \"/Users\"))"
      next
    }
    { print }
  '
}
count_preserving_write_offline=$(replace_write_grant "$offline_profile")
count_preserving_write_extract=$(replace_write_grant "$extract_profile")
count_preserving_write_online=$(replace_write_grant "$online_profile")
count_preserving_write_keychain=$(replace_write_grant "$keychain_profile")
expect_refusal P06_BOUNDARY_REFUSAL_POLICY_DRIFT \
  p06_boundary_validate_policy \
    "$count_preserving_write_offline" "$count_preserving_write_extract" \
    "$count_preserving_write_online" "$count_preserving_write_keychain" \
    /private/tmp/taskseal-p06-phase2-runtime.INVENTED /Users/ysorokin/.codex/auth.json /invented/codex

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

provider_output=$(run_provider_equivalence WORKTREE)
expected_provider_output='P06_PROVIDER_LOGIN_SEMANTICS_EQUIVALENT:c0f4d9a1013d16950b2bd9ff232d252f3c06bb196ec7e62d9c3155cda474c7a9
P06_PROVIDER_MODEL_BLOCK_UNCHANGED:e1e76237c95dff9352dea8a4ef1ff4637493d7139fea55d3bb1c0ae1295fe20e'
if test "$provider_output" != "$expected_provider_output"; then
  printf '%s\n' P06_BOUNDARY_PROVIDER_EQUIVALENCE_FAILED
  exit 1
fi

task_receipt="$root/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/task-1.json"
receipt_implementation_head=$(jq -r .binding.implementation_result_head "$task_receipt")
expected_provider_command="P06_BOUNDARY_PROVIDER_CURRENT_OBJECT=$receipt_implementation_head P06_BOUNDARY_TEST_CASE=provider-equivalence sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-boundary-validator.sh"
if test "$(jq -r '.evidence[] | select(.id == "P06-PHASE2-TRUTH-CORRECTION-T1-PROVIDER-BLOCK-EQUIVALENCE-V1") | .command' "$task_receipt")" != "$expected_provider_command" ||
  ! jq -e --arg baseline "$provider_baseline_object" --arg current "$receipt_implementation_head" --arg path "$provider_runner_path" '
    .provider_equivalence == {
      baseline_object:$baseline,
      current_object:$current,
      path:$path,
      login_region:{
        start:"# Validate the exact field type and single-line/nonempty shape before consuming the login counter.",
        end:"extract_result=EXTRACT_PREVALIDATED",
        normalization:"replace -extract \"$credential_plutil_field\" raw with -extract tokens.access_token raw"
      },
      model_region:{
        start:"    if test \"$marker_status\" = 0; then",
        end:"      model_status=$?",
        normalization:"none"
      }
    }
  ' "$task_receipt" >/dev/null; then
  printf '%s\n' P06_BOUNDARY_RED_PROVIDER_COMMAND_NOT_REPRODUCIBLE
  exit 1
fi

printf '%s\n' P06_PHASE2_TRUTH_BOUNDARY_VALIDATOR_PASS

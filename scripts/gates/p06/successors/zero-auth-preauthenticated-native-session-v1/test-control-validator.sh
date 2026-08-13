#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
validator="$root/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/control-validator.rb"
ssot_root=${TASKSEAL_OWNER_SSOT_ROOT:-/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal}
owner="$ssot_root/TASKSEAL-OWNER-DECISIONS.md"
master="$ssot_root/TASKSEAL-MASTER-PROGRAM.md"
trace="$ssot_root/TASKSEAL-REQUIREMENTS-TRACEABILITY.md"
execution_map="$root/controls/v0.1-execution-map.tsv"

# Break caught: deleting the production validator must make this focused suite fail.
if ! test -f "$validator"; then
  printf '%s\n' P06_ZERO_AUTH_RED_MISSING_CONTROL_VALIDATOR
  exit 1
fi

run_validator() {
  ruby "$validator" "$1" "$2" "$3" "$4"
}

expect_refusal() {
  expected=$1
  shift
  set +e
  actual=$(run_validator "$@" 2>&1)
  status=$?
  set -e
  test "$status" = 1
  test "$actual" = "P06_ZERO_AUTH_CONTROL_REFUSAL:$expected"
}

replace_once() {
  path=$1
  old=$2
  new=$3
  ruby -e '
    path, old, replacement = ARGV
    text = File.binread(path)
    abort "fixture source missing or non-unique" unless text.scan(old).length == 1
    File.binwrite(path, text.sub(old, replacement))
  ' "$path" "$old" "$new"
}

test "$(run_validator "$owner" "$master" "$trace" "$execution_map")" = P06_ZERO_AUTH_CONTROL_PASS

fixture_root=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-control.XXXXXX")
trap 'rm -rf "$fixture_root"' EXIT HUP INT TERM
cp "$owner" "$fixture_root/owner.md"
cp "$master" "$fixture_root/master.md"
cp "$trace" "$fixture_root/trace.md"
cp "$execution_map" "$fixture_root/map.tsv"

# Break caught: the stargazer threshold must never become an automatic switch.
cp "$fixture_root/owner.md" "$fixture_root/owner-threshold.md"
replace_once "$fixture_root/owner-threshold.md" \
  'The
threshold is reconsideration eligibility only; it does not change runtime
behavior automatically.' \
  'The threshold automatically enables authentication at runtime.'
expect_refusal OD10_THRESHOLD_NON_AUTOMATIC \
  "$fixture_root/owner-threshold.md" "$master" "$trace" "$execution_map"

# Break caught: TaskSeal must not regain ownership of keys or tokens.
cp "$fixture_root/owner.md" "$fixture_root/owner-credential.md"
replace_once "$fixture_root/owner-credential.md" \
  'MUST NOT request, read, copy, or store API keys or tokens' \
  'MAY request, read, copy, or store API keys or tokens'
expect_refusal OD10_ZERO_CREDENTIAL_OWNERSHIP \
  "$fixture_root/owner-credential.md" "$master" "$trace" "$execution_map"

# Break caught: unavailable or ambiguous native session state must refuse pre-birth.
cp "$fixture_root/owner.md" "$fixture_root/owner-birth.md"
replace_once "$fixture_root/owner-birth.md" \
  'Unavailable or ambiguous session state MUST fail before provider birth' \
  'Unavailable or ambiguous session state MAY continue to provider birth'
expect_refusal OD10_FAIL_CLOSED_PROVIDER_BIRTH \
  "$fixture_root/owner-birth.md" "$master" "$trace" "$execution_map"

# Break caught: provider unavailability must not take local TaskSeal away.
cp "$fixture_root/master.md" "$fixture_root/master-local.md"
replace_once "$fixture_root/master-local.md" \
  'local TaskSeal functionality remains
  available without qualification' \
  'local TaskSeal functionality becomes unavailable without qualification'
expect_refusal MASTER_LOCAL_CONTINUITY \
  "$owner" "$fixture_root/master-local.md" "$trace" "$execution_map"

# Break caught: the superseded ADP-05 raw-auth contract must not return.
cp "$fixture_root/trace.md" "$fixture_root/trace-adp.md"
replace_once "$fixture_root/trace-adp.md" \
  'TaskSeal transports no authentication material; provider access may use only an independently established provider-native preauthenticated session' \
  'Required auth works without recording secret values'
expect_refusal TRACE_OLD_ADP05_CONFLICT \
  "$owner" "$master" "$fixture_root/trace-adp.md" "$execution_map"

# Break caught: AUTH-01 must remain jointly assigned to P06 and P08.
cp "$fixture_root/map.tsv" "$fixture_root/map-owner.md"
replace_once "$fixture_root/map-owner.md" \
  'P06-ZERO-AUTH-T1;P08-T10' \
  'P06-ZERO-AUTH-T1'
expect_refusal MAP_AUTH01_ASSIGNMENT \
  "$owner" "$master" "$trace" "$fixture_root/map-owner.md"

# Break caught: the release map cannot turn the threshold into runtime automation.
cp "$fixture_root/map.tsv" "$fixture_root/map-threshold.md"
replace_once "$fixture_root/map-threshold.md" \
  'Threshold eligibility is owner-governed only; no runtime GitHub/stargazer check or automatic enablement' \
  'Runtime GitHub stargazer threshold automatically enables authentication'
expect_refusal MAP_AUTH01_THRESHOLD_NON_AUTOMATIC \
  "$owner" "$master" "$trace" "$fixture_root/map-threshold.md"

printf '%s\n' P06_ZERO_AUTH_CONTROL_TEST_PASS

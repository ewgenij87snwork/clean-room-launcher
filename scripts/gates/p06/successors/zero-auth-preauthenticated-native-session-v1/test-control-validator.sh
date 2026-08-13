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
  if test "$status" != 1; then
    printf '%s\n' "P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:$expected"
    exit 1
  fi
  if test "$actual" != "P06_ZERO_AUTH_CONTROL_REFUSAL:$expected"; then
    printf '%s\n' "P06_ZERO_AUTH_WRONG_REFUSAL:$expected:$actual"
    exit 1
  fi
}

collect_refusal() {
  expected=$1
  shift
  set +e
  actual=$(run_validator "$@" 2>&1)
  status=$?
  set -e
  if test "$status" != 1; then
    printf '%s\n' "P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:$expected"
    return 1
  fi
  if test "$actual" != "P06_ZERO_AUTH_CONTROL_REFUSAL:$expected"; then
    printf '%s\n' "P06_ZERO_AUTH_WRONG_REFUSAL:$expected:$actual"
    return 1
  fi
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

append_line() {
  path=$1
  line=$2
  ruby -e 'File.open(ARGV[0], "ab") { |file| file.write("\n#{ARGV[1]}\n") }' "$path" "$line"
}

test "$(run_validator "$owner" "$master" "$trace" "$execution_map")" = P06_ZERO_AUTH_CONTROL_PASS

fixture_root=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-control.XXXXXX")
trap 'rm -rf "$fixture_root"' EXIT HUP INT TERM
cp "$owner" "$fixture_root/owner.md"
cp "$master" "$fixture_root/master.md"
cp "$trace" "$fixture_root/trace.md"
cp "$execution_map" "$fixture_root/map.tsv"

# Break caught: retaining every required sentence cannot mask a contradictory permission.
cp "$fixture_root/owner.md" "$fixture_root/owner-provider-login-contradiction.md"
append_line "$fixture_root/owner-provider-login-contradiction.md" \
  'TaskSeal MAY invoke provider login.'
expect_refusal OD10_CONTRADICTORY_PERMISSION \
  "$fixture_root/owner-provider-login-contradiction.md" "$master" "$trace" "$execution_map"

# Break caught: every positive normative form of the exact prohibited provider-login
# clause must refuse in both governed sections, without relying on a partial modal list.
normative_failures=0
normative_index=0
for normative in MUST SHOULD MAY CAN SHALL WILL must should may can shall will; do
  normative_index=$((normative_index + 1))

  owner_normative="$fixture_root/owner-provider-login-normative-$normative_index.md"
  cp "$fixture_root/owner.md" "$owner_normative"
  append_line "$owner_normative" "TaskSeal $normative invoke provider login."
  if ! collect_refusal OD10_CONTRADICTORY_PERMISSION \
    "$owner_normative" "$master" "$trace" "$execution_map"; then
    normative_failures=1
  fi

  master_normative="$fixture_root/master-provider-login-normative-$normative_index.md"
  cp "$fixture_root/master.md" "$master_normative"
  replace_once "$master_normative" \
    '  only, with no runtime GitHub/stargazer check and no automatic enablement;' \
    "  only, with no runtime GitHub/stargazer check and no automatic enablement;
  TaskSeal $normative invoke provider login."
  if ! collect_refusal MASTER_CONTRADICTORY_PERMISSION \
    "$owner" "$master_normative" "$trace" "$execution_map"; then
    normative_failures=1
  fi
done
if test "$normative_failures" != 0; then
  exit 1
fi

# Break caught: the permanent law must keep dated, public threshold evidence.
cp "$fixture_root/master.md" "$fixture_root/master-dated-evidence.md"
replace_once "$fixture_root/master-dated-evidence.md" \
  'until dated public evidence records at least 50,000 public stargazers' \
  'until the canonical repository reaches at least 50,000 public stargazers'
expect_refusal MASTER_DATED_PUBLIC_EVIDENCE \
  "$owner" "$fixture_root/master-dated-evidence.md" "$trace" "$execution_map"

# Break caught: reaching the threshold cannot replace the separate named owner decision.
cp "$fixture_root/owner.md" "$fixture_root/owner-separate-decision.md"
replace_once "$fixture_root/owner-separate-decision.md" \
  '**AND** separately approves
a named superseding product decision' \
  'and automatically changes the product decision'
expect_refusal OD10_TWO_OWNER_GATES \
  "$fixture_root/owner-separate-decision.md" "$master" "$trace" "$execution_map"

# Break caught: the stargazer threshold must never become an automatic switch.
cp "$fixture_root/owner.md" "$fixture_root/owner-threshold.md"
replace_once "$fixture_root/owner-threshold.md" \
  'The
threshold is reconsideration eligibility only; it does not change runtime
behavior automatically.' \
  'The threshold automatically enables authentication at runtime.'
expect_refusal OD10_THRESHOLD_NON_AUTOMATIC \
  "$fixture_root/owner-threshold.md" "$master" "$trace" "$execution_map"

# Break caught: TaskSeal must not perform the runtime stargazer check itself.
cp "$fixture_root/owner.md" "$fixture_root/owner-runtime-count.md"
replace_once "$fixture_root/owner-runtime-count.md" \
  'MUST NOT perform a runtime GitHub or stargazer
count check' \
  'MAY perform a runtime GitHub or stargazer count check'
expect_refusal OD10_NO_RUNTIME_GITHUB_CHECK \
  "$fixture_root/owner-runtime-count.md" "$master" "$trace" "$execution_map"

# Break caught: threshold attainment cannot automatically enable an auth path.
cp "$fixture_root/owner.md" "$fixture_root/owner-auto-enable.md"
replace_once "$fixture_root/owner-auto-enable.md" \
  'MUST NOT automatically enable any login' \
  'MAY automatically enable login'
expect_refusal OD10_NO_AUTOMATIC_ENABLEMENT \
  "$fixture_root/owner-auto-enable.md" "$master" "$trace" "$execution_map"

# Break caught: TaskSeal cannot request login.
cp "$fixture_root/owner.md" "$fixture_root/owner-login.md"
replace_once "$fixture_root/owner-login.md" \
  'MUST NOT request login' \
  'MAY request login'
expect_refusal OD10_NO_LOGIN_REQUEST \
  "$fixture_root/owner-login.md" "$master" "$trace" "$execution_map"

# Break caught: TaskSeal cannot open or trigger browser OAuth/device flow.
cp "$fixture_root/owner.md" "$fixture_root/owner-browser.md"
replace_once "$fixture_root/owner-browser.md" \
  'open or trigger a browser OAuth or device flow' \
  'open a browser OAuth flow'
expect_refusal OD10_NO_BROWSER_OAUTH_DEVICE \
  "$fixture_root/owner-browser.md" "$master" "$trace" "$execution_map"

# Break caught: TaskSeal must not regain ownership of keys or tokens.
cp "$fixture_root/owner.md" "$fixture_root/owner-credential.md"
replace_once "$fixture_root/owner-credential.md" \
  'MUST NOT request, read, copy, or store API keys or tokens' \
  'MAY request, read, copy, or store API keys or tokens'
expect_refusal OD10_ZERO_CREDENTIAL_OWNERSHIP \
  "$fixture_root/owner-credential.md" "$master" "$trace" "$execution_map"

# Break caught: no authentication or billing fallback is available.
cp "$fixture_root/owner.md" "$fixture_root/owner-auth-billing.md"
replace_once "$fixture_root/owner-auth-billing.md" \
  'MUST NOT fall back to any authentication or billing flow' \
  'MAY fall back to an authentication or billing flow'
expect_refusal OD10_NO_AUTH_BILLING_FALLBACK \
  "$fixture_root/owner-auth-billing.md" "$master" "$trace" "$execution_map"

# Break caught: only an independently established provider-native session is admissible.
cp "$fixture_root/owner.md" "$fixture_root/owner-preauthenticated-only.md"
replace_once "$fixture_root/owner-preauthenticated-only.md" \
  'MAY
use only a provider-native preauthenticated session that was independently
established outside TaskSeal' \
  'MAY use a TaskSeal-created authentication session'
expect_refusal OD10_PREAUTHENTICATED_NATIVE_SESSION_ONLY \
  "$fixture_root/owner-preauthenticated-only.md" "$master" "$trace" "$execution_map"

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

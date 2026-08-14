#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
successor=scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1
verify="$root/$successor/verify.sh"
report="$root/reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/consolidated.json"
worklog=/Users/ysorokin/Documents/it/5-LVL\ -\ 2026/Temp\ in\ Projects/wisdom/taskseal/TASKSEAL-WORKLOG.jsonl

if ! test -x "$verify"; then
  printf '%s\n' P06_ZERO_AUTH_RED_VERIFIER_MISSING
  exit 1
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-gate.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
fixture_report="$scratch/consolidated.json"
fixture_worklog="$scratch/worklog.jsonl"

reset_fixtures() {
  jq . "$report" >"$fixture_report"
  awk '{ print }' "$worklog" >"$fixture_worklog"
}

run_metadata() {
  P06_ZERO_AUTH_METADATA_ONLY=1 \
    P06_ZERO_AUTH_REPORT="$fixture_report" \
    P06_ZERO_AUTH_WORKLOG="$fixture_worklog" \
    "$verify"
}

expect_refusal() {
  label=$1
  set +e
  output=$(run_metadata 2>&1)
  status=$?
  set -e
  if test "$status" = 0; then
    printf 'P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:%s\n' "$label" >&2
    exit 1
  fi
  test -n "$output"
  reset_fixtures
}

reset_fixtures
test "$(run_metadata)" = P06_ZERO_AUTH_METADATA_PASS

jq '.terminal_review.state="PASS"' "$report" >"$fixture_report"
expect_refusal terminal_review_checkpoint

jq '.owner_ssot.worklog_prefix.sha256="0000000000000000000000000000000000000000000000000000000000000000"' "$report" >"$fixture_report"
expect_refusal ssot_prefix_digest

jq '.owner_ssot.worklog_prefix.line_count=129' "$report" >"$fixture_report"
expect_refusal ssot_prefix_count

jq '.controls |= map(select(. != "AUTH-01"))' "$report" >"$fixture_report"
expect_refusal auth_control

jq '.task_receipts[2].sha256="0000000000000000000000000000000000000000000000000000000000000000"' "$report" >"$fixture_report"
expect_refusal task_receipt_digest

jq '.git.implementation_write_set |= map(select(. != "src/cli/screen.rs"))' "$report" >"$fixture_report"
expect_refusal implementation_write_set

jq '.verification.public_boundary_paths |= map(select(. != "src"))' "$report" >"$fixture_report"
expect_refusal public_boundary_inventory

jq '.result="PASS"' "$report" >"$fixture_report"
expect_refusal premature_plan_pass

ruby -rjson -e '
  lines = File.readlines(ARGV.fetch(0), chomp: false)
  first = JSON.parse(lines.fetch(0))
  first["status"] = "mutated"
  lines[0] = JSON.generate(first) + "\n"
  File.binwrite(ARGV.fetch(1), lines.join)
' "$worklog" "$fixture_worklog"
expect_refusal mutable_prefix

printf '%s\n' '{"schema":"taskseal.worklog.event.v1","event_id":"P06-ZERO-AUTH-T4-TERMINAL-REV-APPEND-FIXTURE","work_id":"P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1-T4","event":"terminal_review","recorded_at":"2026-08-14T12:00:00+02:00","result":"REV: PASS"}' >>"$fixture_worklog"
test "$(run_metadata)" = P06_ZERO_AUTH_METADATA_PASS
reset_fixtures

printf '%s\n' '{invalid-json' >>"$fixture_worklog"
expect_refusal invalid_append

duplicate_id=$(sed -n '1p' "$worklog" | jq -r .event_id)
printf '{"schema":"taskseal.worklog.event.v1","event_id":"%s","event":"terminal_review"}\n' "$duplicate_id" >>"$fixture_worklog"
expect_refusal duplicate_append

printf '%s\n' P06_ZERO_AUTH_GATE_MUTATIONS_PASS

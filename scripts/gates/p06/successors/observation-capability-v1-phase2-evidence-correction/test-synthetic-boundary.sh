#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
runner="$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/synthetic-boundary.sh"

run_vector() {
  tuple=synthetic-provider@0.0.0/darwin/arm64
  base=73d48ffbed1794c6691ba59be006aa096dcfcb22
  authority=P06-PHASE2-CORRECTION-SYNTHETIC-AUTHORITY-V1
  login_counter=0
  model_counter=0
  credential_source=fixture://synthetic/credential
  credential_field=.synthetic.access_token
  output=P06_SYNTHETIC_SAFE_OUTPUT
  keychain=DENIED
  read_policy=fixture-only
  write_policy=none
  cleanup=removed
  successor=observation-capability-v1-phase2-evidence-correction

  case "$1" in
    control) ;;
    wrong_tuple) tuple=wrong-provider@0.0.0/darwin/arm64 ;;
    wrong_base) base=deadbeef ;;
    wrong_authority) authority=wrong-authority ;;
    reused_counter) login_counter=1 ;;
    alternate_credential_source) credential_source=fixture://synthetic/alternate ;;
    alternate_credential_field) credential_field=.synthetic.alternate ;;
    credential_shaped_output) output='Bearer synthetic-token' ;;
    keychain_not_denied) keychain=AVAILABLE ;;
    broad_read_policy) read_policy=broad ;;
    broad_write_policy) write_policy=broad ;;
    cleanup_failed) cleanup=failed ;;
    non_successor_drift) successor=historical-phase-2 ;;
    *) return 64 ;;
  esac

  "$runner" \
    --tuple "$tuple" --base "$base" --authority "$authority" \
    --login-counter "$login_counter" --model-counter "$model_counter" \
    --credential-source "$credential_source" --credential-field "$credential_field" \
    --output "$output" --keychain "$keychain" --read-policy "$read_policy" \
    --write-policy "$write_policy" --cleanup "$cleanup" --successor "$successor"
}

expect_refusal() {
  vector=$1
  expected=$2
  set +e
  actual=$(run_vector "$vector" 2>&1)
  status=$?
  set -e
  test "$status" = 1
  test "$actual" = "$expected"
}

actual=$(run_vector control)
test "$actual" = P06_SYNTHETIC_CONTROL_ACCEPTED

expect_refusal wrong_tuple P06_SYNTHETIC_REFUSAL_WRONG_TUPLE
expect_refusal wrong_base P06_SYNTHETIC_REFUSAL_WRONG_BASE
expect_refusal wrong_authority P06_SYNTHETIC_REFUSAL_WRONG_AUTHORITY
expect_refusal reused_counter P06_SYNTHETIC_REFUSAL_REUSED_COUNTER
expect_refusal alternate_credential_source P06_SYNTHETIC_REFUSAL_ALTERNATE_CREDENTIAL_SOURCE
expect_refusal alternate_credential_field P06_SYNTHETIC_REFUSAL_ALTERNATE_CREDENTIAL_FIELD
expect_refusal credential_shaped_output P06_SYNTHETIC_REFUSAL_CREDENTIAL_SHAPED_OUTPUT
expect_refusal keychain_not_denied P06_SYNTHETIC_REFUSAL_KEYCHAIN_NOT_DENIED
expect_refusal broad_read_policy P06_SYNTHETIC_REFUSAL_BROAD_READ_POLICY
expect_refusal broad_write_policy P06_SYNTHETIC_REFUSAL_BROAD_WRITE_POLICY
expect_refusal cleanup_failed P06_SYNTHETIC_REFUSAL_CLEANUP_FAILED
expect_refusal non_successor_drift P06_SYNTHETIC_REFUSAL_NON_SUCCESSOR_DRIFT

printf '%s\n' P06_PHASE2_CORRECTION_SYNTHETIC_PASS

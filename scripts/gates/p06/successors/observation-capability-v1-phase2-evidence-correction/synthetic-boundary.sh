#!/bin/sh
set -eu

refuse() {
  printf '%s\n' "$1"
  exit 1
}

invalid_arguments() {
  refuse P06_SYNTHETIC_REFUSAL_INVALID_ARGUMENTS
}

test "$#" = 26 || invalid_arguments
test "$1" = --tuple || invalid_arguments
tuple=$2
shift 2
test "$1" = --base || invalid_arguments
base=$2
shift 2
test "$1" = --authority || invalid_arguments
authority=$2
shift 2
test "$1" = --login-counter || invalid_arguments
login_counter=$2
shift 2
test "$1" = --model-counter || invalid_arguments
model_counter=$2
shift 2
test "$1" = --credential-source || invalid_arguments
credential_source=$2
shift 2
test "$1" = --credential-field || invalid_arguments
credential_field=$2
shift 2
test "$1" = --output || invalid_arguments
output=$2
shift 2
test "$1" = --keychain || invalid_arguments
keychain=$2
shift 2
test "$1" = --read-policy || invalid_arguments
read_policy=$2
shift 2
test "$1" = --write-policy || invalid_arguments
write_policy=$2
shift 2
test "$1" = --cleanup || invalid_arguments
cleanup=$2
shift 2
test "$1" = --successor || invalid_arguments
successor=$2

test "$tuple" = synthetic-provider@0.0.0/darwin/arm64 ||
  refuse P06_SYNTHETIC_REFUSAL_WRONG_TUPLE
test "$base" = 73d48ffbed1794c6691ba59be006aa096dcfcb22 ||
  refuse P06_SYNTHETIC_REFUSAL_WRONG_BASE
test "$authority" = P06-PHASE2-CORRECTION-SYNTHETIC-AUTHORITY-V1 ||
  refuse P06_SYNTHETIC_REFUSAL_WRONG_AUTHORITY
test "$login_counter" = 0 && test "$model_counter" = 0 ||
  refuse P06_SYNTHETIC_REFUSAL_REUSED_COUNTER
test "$credential_source" = fixture://synthetic/credential ||
  refuse P06_SYNTHETIC_REFUSAL_ALTERNATE_CREDENTIAL_SOURCE
test "$credential_field" = .synthetic.access_token ||
  refuse P06_SYNTHETIC_REFUSAL_ALTERNATE_CREDENTIAL_FIELD
test "$output" = P06_SYNTHETIC_SAFE_OUTPUT ||
  refuse P06_SYNTHETIC_REFUSAL_CREDENTIAL_SHAPED_OUTPUT
test "$keychain" = DENIED ||
  refuse P06_SYNTHETIC_REFUSAL_KEYCHAIN_NOT_DENIED
test "$read_policy" = fixture-only ||
  refuse P06_SYNTHETIC_REFUSAL_BROAD_READ_POLICY
test "$write_policy" = none ||
  refuse P06_SYNTHETIC_REFUSAL_BROAD_WRITE_POLICY
test "$cleanup" = removed ||
  refuse P06_SYNTHETIC_REFUSAL_CLEANUP_FAILED
test "$successor" = observation-capability-v1-phase2-evidence-correction ||
  refuse P06_SYNTHETIC_REFUSAL_NON_SUCCESSOR_DRIFT

printf '%s\n' P06_SYNTHETIC_CONTROL_ACCEPTED

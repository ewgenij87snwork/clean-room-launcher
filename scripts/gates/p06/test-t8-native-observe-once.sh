#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
set +e
output=$(P06_CODEX_BIN=/usr/bin/false P06_T8_AUTH_SOURCE=/dev/null "$root/scripts/gates/p06/t8-native-observe-once.sh" 2>&1)
status=$?
set -e
test "$status" = 2
test "$output" = "P06_T8_FRESH_AUTHORITY_REQUIRED"
echo P06_T8_NATIVE_ONE_SHOT_REFUSAL_TEST_PASS

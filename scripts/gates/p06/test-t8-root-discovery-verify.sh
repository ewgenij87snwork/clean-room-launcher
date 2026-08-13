#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
output=$(P06_CODEX_BIN=${P06_CODEX_BIN:?P06_CODEX_BIN is required} "$root/scripts/gates/p06/t8-root-discovery-verify.sh")
test "$output" = "P06_T8_ROOT_DISCOVERY_PASS"
echo P06_T8_ROOT_DISCOVERY_TEST_PASS

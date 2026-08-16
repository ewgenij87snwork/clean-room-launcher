#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
gate="$root/scripts/gates/p07/successors/clroom-packaging-v1/verify.sh"
[ -x "$gate" ] || { echo "P07_CLROOM_PACKAGING_GATE_TEST_REFUSED:NOT_EXECUTABLE" >&2; exit 1; }
grep -Fq 'clean-room-launcher-*.tar.gz' "$gate"
grep -Fq 'bin/clroom' "$root/packaging/verify-artifact.py"
grep -Fq 'upgrade_n_plus_1 rollback_n uninstall_current untap' "$gate"
echo P07_CLROOM_PACKAGING_GATE_TEST_PASS

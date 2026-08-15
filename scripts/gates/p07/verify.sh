#!/bin/sh
set -eu
root=${P07_GATE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)}
exec python3 "$root/scripts/gates/p07/verify.py" "$root"

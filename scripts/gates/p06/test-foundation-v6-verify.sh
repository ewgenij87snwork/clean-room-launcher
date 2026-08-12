#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)

# A partial foundation can never silently become accepted evidence.
for receipt in task-3.json task-4.json foundation-v6.json; do
  if [ ! -f "$root/reports/gates/p06/$receipt" ]; then
    echo "P06_FOUNDATION_EVIDENCE_INCOMPLETE:$receipt"
    exit 1
  fi
done

echo P06_FOUNDATION_NEGATIVE_TEST_INVALID
exit 1

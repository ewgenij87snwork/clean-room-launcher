#!/bin/sh
set -eu
while IFS= read -r changed_path; do
  case "$changed_path" in
    scripts/gates/p06/successors/observation-capability-v1/*|reports/gates/p06/successors/observation-capability-v1/*) ;;
    *) echo "P06_CAPABILITY_WRITE_SET_REFUSED:$changed_path" >&2; exit 2 ;;
  esac
done

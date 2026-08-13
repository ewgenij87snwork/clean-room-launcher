#!/bin/sh
set -eu
while IFS= read -r changed_path; do
  case "$changed_path" in
    scripts/gates/p06/successors/observation-capability-v1-phase2/*|reports/gates/p06/successors/observation-capability-v1-phase2/*) ;;
    *) echo "P06_PHASE2_WRITE_SET_REFUSED:$changed_path" >&2; exit 2 ;;
  esac
done

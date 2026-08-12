#!/bin/sh
set -eu

root=$(cd "$(dirname "$0")/../../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p04-sensitivity.XXXXXX")
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT HUP INT TERM

git -C "$root" archive --format=tar HEAD | tar -xf - -C "$scratch"
perl -0pi -e 's/"full_bodies_at_startup":0/"full_bodies_at_startup":1/' \
  "$scratch/reports/gates/p04/acceptance-evidence.json"

if (
  cd "$scratch"
  cargo test --locked --offline \
    'catalog::pipeline_tests::committed_acceptance_evidence_is_derived_from_production_pipeline' \
    --lib
); then
  echo "P04_REGRESSION_SENSITIVITY_MISSED" >&2
  exit 1
fi

echo P04_REGRESSION_SENSITIVITY_PASS

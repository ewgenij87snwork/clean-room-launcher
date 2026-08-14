#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
workflow="$root/.github/workflows/release-candidate.yml"
subject=${TASKSEAL_SUBJECT_DIGEST:-}
scaffold=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --workflow) workflow=$2; shift 2 ;;
    --subject-digest) subject=$2; shift 2 ;;
    --scaffold) scaffold=1; shift ;;
    *) echo "UNKNOWN_OPTION:$1" >&2; exit 64 ;;
  esac
done

case "$subject" in
  "") subject=$(git -C "$root" rev-parse HEAD) ;;
  *[!0-9a-f]*) echo "INVALID_SUBJECT_DIGEST" >&2; exit 65 ;;
esac
[ -s "$workflow" ] || { echo "MISSING_WORKFLOW:$workflow" >&2; exit 66; }

# These are structural failures, never skips.  Keep this parser deliberately
# conservative because it is also used against poisoned workflow fixtures.
if grep -nE 'continue-on-error:|^[[:space:]]*if:|^[[:space:]]*-[[:space:]]*if:' "$workflow"; then
  echo "NO_SKIP_VIOLATION" >&2; exit 67
fi
for gate in p02 p03 p04 p05 p06 p07; do
  grep -q "${gate}-gate" "$workflow" || { echo "MISSING_GATE:$gate" >&2; exit 68; }
done
grep -q 'TASKSEAL_SUBJECT_DIGEST' "$workflow" || { echo "DIGEST_NOT_PROPAGATED" >&2; exit 69; }

if [ "$scaffold" -eq 1 ]; then
  echo "P07_SCAFFOLD_VALIDATION_PASS subject=$subject"
  exit 0
fi

results=$(mktemp "${TMPDIR:-/tmp}/taskseal-release-results.XXXXXX")
cleanup() {
  case "$results" in
    "${TMPDIR:-/tmp}"/taskseal-release-results.*) rm -f -- "$results" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

run_check() {
  name=$1; shift
  log=$(mktemp "${TMPDIR:-/tmp}/taskseal-release-check.XXXXXX")
  if TASKSEAL_SUBJECT_DIGEST="$subject" "$@" >"$log" 2>&1; then
    jq -cn --arg name "$name" --arg digest "$subject" \
      '{name:$name,exit:0,status:"PASS",subject_digest:$digest}' >>"$results"
  else
    code=$?
    jq -cn --arg name "$name" --arg digest "$subject" --argjson exit "$code" \
      '{name:$name,exit:$exit,status:"NOT_QUALIFIED",subject_digest:$digest}' >>"$results"
    cat "$log" >&2
    rm -f -- "$log"
    return 1
  fi
  rm -f -- "$log"
}

failed=0
run_check fmt cargo fmt --all -- --check || failed=1
run_check clippy cargo clippy --all-targets --locked --offline -- -D warnings || failed=1
run_check test cargo test --all-targets --locked --offline || failed=1
run_check schema cargo test --test schema_vectors --locked --offline || failed=1
run_check golden cargo test --all-targets --locked --offline golden || failed=1
run_check parity cargo test --all-targets --locked --offline parity || failed=1
run_check privacy scripts/check-public-boundary.sh --root "$root" || failed=1
run_check dependency cargo tree --locked --offline || failed=1
run_check license test -s LICENSE || failed=1

# P06 is intentionally consumed as evidence.  Its known NOT_QUALIFIED state is
# preserved in the receipt and cannot be relabelled as PASS.
run_check p06-qualification scripts/gates/p06/verify.sh || failed=1

commands=$(jq -s . "$results")
jq -n --arg subject "$subject" --argjson commands "$commands" --argjson failed "$failed" \
  '{schema_version:"taskseal.release-source-verification.v1",result:(if $failed == 0 then "PASS" else "NOT_QUALIFIED" end),subject_digest:$subject,commands:$commands,skips_counted_as_pass:0,p06_qualification:"NOT_QUALIFIED",network_or_provider_spend:false}'

[ "$failed" -eq 0 ]

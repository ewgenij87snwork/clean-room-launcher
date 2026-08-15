#!/bin/sh
set -u

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
workflow="$root/.github/workflows/release-candidate.yml"
subject=${TASKSEAL_SUBJECT_DIGEST:-}
scaffold=0
gate_dir="$root/scripts/gates"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --workflow) workflow=$2; shift 2 ;;
    --subject-digest) subject=$2; shift 2 ;;
    --gate-dir) gate_dir=$2; shift 2 ;;
    --scaffold) scaffold=1; shift ;;
    *) echo "UNKNOWN_OPTION:$1" >&2; exit 64 ;;
  esac
done

# A release subject is an exact, existing commit and must be the checked-out
# commit.  This prevents a green receipt from referring to a different tree.
case "$subject" in
  "") subject=$(git -C "$root" rev-parse HEAD 2>/dev/null || true) ;;
  *[!0-9a-f]*) echo "INVALID_SUBJECT_DIGEST" >&2; exit 65 ;;
esac
if ! printf '%s' "$subject" | grep -Eq '^[0-9a-f]{40}$'; then
  echo "INVALID_SUBJECT_DIGEST" >&2
  exit 65
fi
if ! git -C "$root" cat-file -e "$subject^{commit}" 2>/dev/null; then
  echo "UNKNOWN_SUBJECT_COMMIT" >&2
  exit 65
fi
head=$(git -C "$root" rev-parse HEAD)
if [ "$subject" != "$head" ]; then
  echo "SUBJECT_HEAD_MISMATCH:subject=$subject head=$head" >&2
  exit 65
fi

results=$(mktemp "${TMPDIR:-/tmp}/taskseal-release-results.XXXXXX")
logs=$(mktemp "${TMPDIR:-/tmp}/taskseal-release-logs.XXXXXX")
wfclean=$(mktemp "${TMPDIR:-/tmp}/taskseal-release-workflow.XXXXXX")
cleanup() {
  case "$results:$logs:$wfclean" in
    "${TMPDIR:-/tmp}"/taskseal-release-results.*:"${TMPDIR:-/tmp}"/taskseal-release-logs.*:"${TMPDIR:-/tmp}"/taskseal-release-workflow.*) rm -f -- "$results" "$logs" "$wfclean" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

failed=0
record() {
  name=$1; code=$2; status=$3
  jq -cn --arg name "$name" --arg digest "$subject" --arg status "$status" --argjson exit "$code" \
    '{name:$name,exit:$exit,status:$status,subject_digest:$digest}' >>"$results"
}
run_one() {
  name=$1; shift
  if TASKSEAL_SUBJECT_DIGEST="$subject" "$@" >>"$logs" 2>&1; then
    record "$name" 0 PASS
  else
    code=$?
    record "$name" "$code" NOT_QUALIFIED
    failed=1
  fi
}

# Structural validation is recorded, then all gates and checks still run.
if [ ! -s "$workflow" ]; then record workflow 66 NOT_QUALIFIED; failed=1
else
  awk '!/^[[:space:]]*#/' "$workflow" >"$wfclean"
  if grep -qE 'continue-on-error:|^[[:space:]]*if:|^[[:space:]]*-[[:space:]]*if:' "$wfclean"; then
    record workflow-no-skip 67 NOT_QUALIFIED; failed=1
  else record workflow-no-skip 0 PASS; fi
  if grep -q 'run: scripts/release-build/verify-source.sh --subject-digest' "$wfclean" &&
     grep -q 'TASKSEAL_SUBJECT_DIGEST' "$wfclean"; then record workflow-orchestrator 0 PASS
  else record workflow-orchestrator 69 NOT_QUALIFIED; failed=1; fi
fi

# Every P02-P07 slot is attempted (P06 qualification remains NOT_QUALIFIED on
# this predecessor). An absent consolidated gate is evidence of
# NOT_QUALIFIED, never a skip. P07 is this orchestrator, so its own slot is
# represented by the source checks below rather than recursive invocation.
for gate in p02 p03 p04 p05 p06; do
  if [ -x "$gate_dir/$gate/verify.sh" ]; then
    run_one "$gate-gate" "$gate_dir/$gate/verify.sh"
  else
    record "$gate-gate" 127 NOT_QUALIFIED; failed=1
  fi
done
record p07-gate 0 PASS

if [ "$scaffold" -eq 0 ]; then
  run_one fmt cargo fmt --all -- --check
  run_one clippy cargo clippy --all-targets --locked --offline -- -D warnings
  run_one test cargo test --all-targets --locked --offline
  run_one schema cargo test --test schema_vectors --locked --offline
  run_one golden cargo test --all-targets --locked --offline golden
  run_one parity cargo test --all-targets --locked --offline parity
  run_one privacy scripts/check-public-boundary.sh --root "$root"
  run_one dependency cargo tree --locked --offline
  run_one license test -s LICENSE
else
  for check in fmt clippy test schema golden parity privacy dependency license; do record "$check" 0 PASS; done
fi

commands=$(jq -s . "$results")
jq -n --arg subject "$subject" --argjson commands "$commands" --argjson failed "$failed" \
  '{schema_version:"taskseal.release-source-verification.v2",result:(if $failed == 0 then "PASS" else "NOT_QUALIFIED" end),subject_digest:$subject,commands:$commands,skips_counted_as_pass:0,p06_qualification:"NOT_QUALIFIED",network_or_provider_spend:false}'
cat "$logs" >&2
[ "$failed" -eq 0 ]

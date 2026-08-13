#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
successor_rel=scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1
truth_rel=reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/truthfulness.json
verify="$root/$successor_rel/verify.sh"
truth="$root/$truth_rel"

if ! test -x "$verify"; then
  printf '%s\n' P06_TRUTH_RED_VERIFIER_MISSING >&2
  exit 1
fi
if ! test -f "$truth"; then
  printf '%s\n' P06_TRUTH_RED_MANIFEST_MISSING >&2
  exit 1
fi

temporary_root=$(mktemp -d "/tmp/taskseal-p06-truth-verify.XXXXXX")
trap 'rm -rf "$temporary_root"' EXIT HUP INT TERM
fixture_truth="$temporary_root/truthfulness.json"
changed_paths="$temporary_root/changed-paths.txt"
scan_root="$temporary_root/privacy-subject"
mkdir -p "$scan_root/$successor_rel" "$scan_root/$(dirname -- "$truth_rel")"
cp "$truth" "$fixture_truth"
cp "$root/$successor_rel/verify.sh" "$scan_root/$successor_rel/verify.sh"
cp "$root/$successor_rel/test-verify.sh" "$scan_root/$successor_rel/test-verify.sh"
cp "$truth" "$scan_root/$truth_rel"
printf '%s\n' "$successor_rel/test-verify.sh" >"$changed_paths"

run_verify() {
  P06_TRUTH_STAGE=task-2-precommit \
    P06_TRUTH_RECEIPT="$fixture_truth" \
    P06_TRUTH_CHANGED_PATHS_FILE="$changed_paths" \
    P06_TRUTH_PRIVACY_ROOT="$scan_root" \
    "$verify"
}

mutation_must_fail() {
  name=$1
  filter=$2
  jq "$filter" "$truth" >"$fixture_truth"
  set +e
  run_verify >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf 'P06_TRUTH_EXPECTED_REFUSAL_MISSING:%s\n' "$name" >&2
    exit 1
  fi
}

set +e
baseline_output=$(run_verify 2>&1)
baseline_status=$?
set -e
if test "$baseline_status" -ne 0; then
  test -z "$baseline_output" || printf '%s\n' "$baseline_output" >&2
  printf '%s\n' P06_TRUTH_EXPECTED_BASELINE_PASS >&2
  exit 1
fi
test "$baseline_output" = P06_PHASE2_EVIDENCE_TRUTHFULNESS_CORRECTION_PASS

mutation_must_fail credential_state '.uncertainty.credential_retention.state="NOT_RETAINED"'
mutation_must_fail credential_verification '.uncertainty.credential_retention.verification="VERIFIED"'
mutation_must_fail protected_state '.uncertainty.protected_state_equality.state="EQUAL"'
mutation_must_fail protected_verification '.uncertainty.protected_state_equality.verification="VERIFIED"'
mutation_must_fail cleanup_state '.uncertainty.cleanup.state="COMPLETE"'
mutation_must_fail cleanup_verification '.uncertainty.cleanup.verification="VERIFIED"'
mutation_must_fail login_result '.preserved_facts.login_result="LOGIN_SUCCEEDED"'
mutation_must_fail qualification '.preserved_facts.qualification="QUALIFIED"'
mutation_must_fail progression '.preserved_facts.progression="CONTINUED_AT_T9"'
mutation_must_fail model_counter '.preserved_facts.model_counter="CONSUMED"'
mutation_must_fail model_process '.preserved_facts.model_process_exit="STARTED"'
mutation_must_fail forbidden_tasks '.preserved_facts.original_tasks_9_13="PERMITTED"'
mutation_must_fail extra_supported_fact '.preserved_facts.credential_retained=false'
mutation_must_fail boolean_authority '.historical_claim_authority.unsupported_booleans="AUTHORITATIVE"'
mutation_must_fail boolean_inventory '.historical_claim_authority.json_pointers |= .[1:]'
mutation_must_fail historical_receipt '.historical_receipts[0].sha256="0000000000000000000000000000000000000000000000000000000000000000"'
mutation_must_fail task_1_binding '.task_1.receipt_sha256="0000000000000000000000000000000000000000000000000000000000000000"'
mutation_must_fail primary_artifact '.primary_artifacts[1].sha256="0000000000000000000000000000000000000000000000000000000000000000"'

cp "$truth" "$fixture_truth"
printf '%s\n' reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json >"$changed_paths"
set +e
run_verify >/dev/null 2>&1
write_set_status=$?
set -e
if test "$write_set_status" = 0; then
  printf '%s\n' P06_TRUTH_EXPECTED_REFUSAL_MISSING:write_set >&2
  exit 1
fi
printf '%s\n' "$successor_rel/test-verify.sh" >"$changed_paths"

printf 'Bearer %s%s\n' AAAAAAAAAAAAAAAAAAAAAAAA AAAAAAAAAAAAAAAAAAAAAAAA \
  >"$scan_root/$successor_rel/privacy-fixture.txt"
set +e
run_verify >/dev/null 2>&1
privacy_status=$?
set -e
if test "$privacy_status" = 0; then
  printf '%s\n' P06_TRUTH_EXPECTED_REFUSAL_MISSING:privacy >&2
  exit 1
fi
rm "$scan_root/$successor_rel/privacy-fixture.txt"

prepare_history_repository() {
  name=$1
  history_repo="$temporary_root/$name-repository"
  git clone --quiet --no-hardlinks "$root" "$history_repo"
  git -C "$history_repo" config user.name taskseal-fixture
  git -C "$history_repo" config user.email taskseal-fixture.invalid
  cp "$verify" "$history_repo/$successor_rel/verify.sh"
}

run_history_only() {
  history_repo=$1
  set +e
  history_output=$(CDPATH='' cd -- "$history_repo" && P06_TRUTH_HISTORY_ONLY=1 "$history_repo/$successor_rel/verify.sh" 2>&1)
  history_status=$?
  set -e
  if test "$history_status" = 0; then
    printf 'P06_TRUTH_EXPECTED_REFUSAL_MISSING:%s\n' "$2" >&2
    exit 1
  fi
  test -n "$history_output"
}

prepare_history_repository historical-revert
historical_path=reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json
cp "$history_repo/$historical_path" "$temporary_root/historical-baseline.json"
printf '%s\n' ' ' >>"$history_repo/$historical_path"
git -C "$history_repo" add "$historical_path"
git -C "$history_repo" commit --quiet -m 'fixture: historical mutation'
cp "$temporary_root/historical-baseline.json" "$history_repo/$historical_path"
git -C "$history_repo" add "$historical_path"
git -C "$history_repo" commit --quiet -m 'fixture: historical restoration'
run_history_only "$history_repo" historical_change_then_revert

prepare_history_repository privacy-history
privacy_path=$successor_rel/privacy-history-fixture.txt
printf 'Bearer %s%s\n' BBBBBBBBBBBBBBBBBBBBBBBB BBBBBBBBBBBBBBBBBBBBBBBB >"$history_repo/$privacy_path"
git -C "$history_repo" add "$privacy_path"
git -C "$history_repo" commit --quiet -m 'fixture: private material'
printf '%s\n' harmless >"$history_repo/$privacy_path"
git -C "$history_repo" add "$privacy_path"
git -C "$history_repo" commit --quiet -m 'fixture: private material removal'
run_history_only "$history_repo" privacy_change_then_revert

prepare_history_repository topology
topology_branch=$(git -C "$history_repo" branch --show-current)
git -C "$history_repo" checkout --quiet -b fixture-side
printf '%s\n' side >"$history_repo/$successor_rel/topology-side.txt"
git -C "$history_repo" add "$successor_rel/topology-side.txt"
git -C "$history_repo" commit --quiet -m 'fixture: side'
git -C "$history_repo" checkout --quiet "$topology_branch"
printf '%s\n' mainline >"$history_repo/$successor_rel/topology-mainline.txt"
git -C "$history_repo" add "$successor_rel/topology-mainline.txt"
git -C "$history_repo" commit --quiet -m 'fixture: mainline'
git -C "$history_repo" merge --quiet --no-ff fixture-side -m 'fixture: merge'
run_history_only "$history_repo" merge_topology

printf '%s\n' P06_PHASE2_TRUTHFULNESS_MUTATIONS_PASS

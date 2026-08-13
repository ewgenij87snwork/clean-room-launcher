#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
base=c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71
task_1_commit=82e1acb883ebe4ba2f3f54ad57857b126c942409
task_1_implementation=29c5da46d80b79e91c548aaacc44958e95e37bcb
successor_rel=scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1
report_rel=reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1
truth_rel=$report_rel/truthfulness.json
task_1_rel=$report_rel/task-1.json
task_2_rel=$report_rel/task-2.json
default_truth="$root/$truth_rel"
default_task_1="$root/$task_1_rel"
default_task_2="$root/$task_2_rel"
truth=${P06_TRUTH_RECEIPT:-$default_truth}
privacy_root=${P06_TRUTH_PRIVACY_ROOT:-$root}
stage=${P06_TRUTH_STAGE:-task-2}

case "$stage" in
  task-2-precommit|task-2|task-3-precommit|final) ;;
  *) exit 2 ;;
esac

test "$(pwd -P)" = "$root"
test "$(git -C "$root" rev-parse --show-toplevel)" = "$root"
test "$(git -C "$root" branch --show-current)" = feat/p06-phase2-evidence-truthfulness-correction-v1
head=$(git -C "$root" rev-parse HEAD)
git -C "$root" merge-base --is-ancestor "$base" "$head"

temporary_fixture() {
  fixture=$(realpath "$1")
  case "$fixture" in
    /private/tmp/taskseal-p06-truth-verify.*/*) ;;
    *) exit 2 ;;
  esac
  printf '%s\n' "$fixture"
}

if test "$truth" != "$default_truth"; then
  test "$stage" = task-2-precommit
  truth=$(temporary_fixture "$truth")
fi
if test "$privacy_root" != "$root"; then
  test "$stage" = task-2-precommit
  privacy_root=$(temporary_fixture "$privacy_root")
fi
if test -n "${P06_TRUTH_CHANGED_PATHS_FILE:-}"; then
  test "$stage" = task-2-precommit
  changed_paths_file=$(temporary_fixture "$P06_TRUTH_CHANGED_PATHS_FILE")
else
  changed_paths_file=
fi

validate_write_set() {
  while IFS= read -r changed_path; do
    test -n "$changed_path" || continue
    case "$changed_path" in
      scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh) ;;
      "$successor_rel"/*) ;;
      "$report_rel"/*) ;;
      *) printf 'P06_TRUTH_WRITE_SET_REFUSED:%s\n' "$changed_path" >&2; exit 2 ;;
    esac
  done
}

contains_private_material() {
  LC_ALL=C grep -Eq 'Bearer [A-Za-z0-9_./+=-]{32,}|sk-[A-Za-z0-9]{20,}|eyJ[A-Za-z0-9_-]{32,}\.[A-Za-z0-9_-]{10,}' "$1"
}

validate_committed_history() {
  previous=$base
  git -C "$root" rev-list --reverse "$base..$head" | while IFS= read -r commit; do
    parent_line=$(git -C "$root" rev-list --parents -n 1 "$commit")
    if test "$(printf '%s\n' "$parent_line" | awk '{print NF}')" != 2 || \
       test "$(printf '%s\n' "$parent_line" | awk '{print $2}')" != "$previous"; then
      printf 'P06_TRUTH_TOPOLOGY_REFUSED:%s\n' "$commit" >&2
      exit 2
    fi
    git -C "$root" diff-tree --no-commit-id --name-only --no-renames -r "$commit" | validate_write_set
    git -C "$root" diff-tree --no-commit-id --name-only --no-renames -r "$commit" | while IFS= read -r changed_path; do
      test -n "$changed_path" || continue
      if git -C "$root" cat-file -e "$commit:$changed_path" 2>/dev/null; then
        material_file=$(mktemp "${TMPDIR:-/tmp}/taskseal-p06-truth-material.XXXXXX")
        git -C "$root" show "$commit:$changed_path" >"$material_file"
        if contains_private_material "$material_file"; then
          rm -f "$material_file"
          printf 'P06_TRUTH_PRIVACY_REFUSED:%s:%s\n' "$commit" "$changed_path" >&2
          exit 2
        fi
        rm -f "$material_file"
      fi
    done
    previous=$commit
  done
}

if test -n "${P06_TRUTH_HISTORY_ONLY:-}"; then
  test "$P06_TRUTH_HISTORY_ONLY" = 1
  case "$root" in
    /private/tmp/taskseal-p06-truth-verify.*/*-repository) ;;
    *) exit 2 ;;
  esac
  validate_committed_history
  printf '%s\n' P06_PHASE2_TRUTH_HISTORY_PASS
  exit 0
fi

if test -n "$changed_paths_file"; then
  validate_write_set <"$changed_paths_file"
else
  {
    git -C "$root" diff --name-only
    git -C "$root" diff --cached --name-only
    git -C "$root" ls-files --others --exclude-standard
  } | LC_ALL=C sort -u | validate_write_set
fi

for privacy_dir in "$privacy_root/$successor_rel" "$privacy_root/$report_rel"; do
  test -d "$privacy_dir" || continue
  find "$privacy_dir" -type f -print | while IFS= read -r privacy_path; do
    if contains_private_material "$privacy_path"; then
      printf 'P06_TRUTH_PRIVACY_REFUSED:%s\n' "$privacy_path" >&2
      exit 2
    fi
  done
done

authority="$root/.taskseal-dev/execution-authority.json"
checkpoint_path=$(jq -r .plan_checkpoint_path "$authority")
jq -e --arg root "$root" --arg base "$base" '
  keys == ["allowed_external_actions","allowed_task_first","allowed_task_last","approved_at","approved_by","branch","dashboard_path","forbidden_actions","head","plan_checkpoint_path","plan_checkpoint_sha256","plan_id","repository_realpath","schema_version","status_path","worklog_path","worktree_realpath"] and
  .schema_version == "taskseal.execution-authority.v2" and
  .plan_id == "P06-PHASE2-EVIDENCE-TRUTHFULNESS-CORRECTION-V1" and
  .allowed_task_first == 1 and .allowed_task_last == 3 and
  .repository_realpath == $root and .worktree_realpath == $root and
  .branch == "feat/p06-phase2-evidence-truthfulness-correction-v1" and .head == $base and
  .plan_checkpoint_sha256 == "a8a11bfa9b7f83369041c03f217f6d389a97c7a1d84756228e65e9a1e61ab9bf" and
  .allowed_external_actions == [] and
  (.forbidden_actions | index("network access")) != null and
  (.forbidden_actions | index("provider process")) != null and
  (.forbidden_actions | index("Codex process")) != null and
  (.forbidden_actions | index("credential read")) != null and
  (.forbidden_actions | index("Keychain access")) != null and
  (.forbidden_actions | index("historical receipt mutation")) != null and
  (.forbidden_actions | index("original P06 T9-T13")) != null and
  (.forbidden_actions | index("main mutation")) != null
' "$authority" >/dev/null
test "$(shasum -a 256 "$checkpoint_path" | awk '{print $1}')" = a8a11bfa9b7f83369041c03f217f6d389a97c7a1d84756228e65e9a1e61ab9bf

# Fail fast on the truth claims before replaying the accepted Task 1 interface.
# The full closed manifest contract and byte bindings are validated below.
jq -e '
  (.preserved_facts | keys) == ["login_result","model_counter","model_process_exit","original_tasks_9_13","progression","qualification"] and
  .preserved_facts == {login_result:"LOGIN_REFUSED",qualification:"NOT_QUALIFIED",progression:"STOPPED_AT_T8",model_counter:"UNUSED",model_process_exit:"NOT_STARTED",original_tasks_9_13:"FORBIDDEN"} and
  .uncertainty.credential_retention.state == "UNKNOWN" and .uncertainty.credential_retention.verification == "UNVERIFIED" and
  .uncertainty.credential_retention.reason == "No independent primary artifact proves post-attempt credential retention state." and
  .uncertainty.protected_state_equality.state == "UNKNOWN" and .uncertainty.protected_state_equality.verification == "UNVERIFIED" and
  .uncertainty.protected_state_equality.reason == "No independent before/after artifact proves equality of protected, auth, binary, or worktree state." and
  .uncertainty.cleanup.state == "UNKNOWN" and .uncertainty.cleanup.verification == "UNVERIFIED" and
  .uncertainty.cleanup.reason == "No independent primary artifact proves temporary-state and raw-output cleanup." and
  .historical_claim_authority.rule == "Byte binding preserves what a historical receipt said; it does not verify an unsupported runtime boolean." and
  .historical_claim_authority.unsupported_booleans == "NON_AUTHORITATIVE" and
  (.historical_claim_authority.json_pointers | length) == 20 and
  .task_1.receipt_sha256 == "7d6ae063881b69b1117402cae45bbe2b50ae3609595e2aec424ed5097ba49f8e" and
  .historical_receipts[0].sha256 == "7e0db9d70c99976e7666d6a9a4ecc903462d821620f8d862cd397a50c2136115" and
  .primary_artifacts == [
    {id:"P06_PHASE2_PREFLIGHT_OUTPUT",commit:"73d48ffbed1794c6691ba59be006aa096dcfcb22",path:"reports/gates/p06/successors/observation-capability-v1-phase2/outputs/preflight.txt",sha256:"94c0c0172cf8fae3b52add679f328307820b67641ae9350cd3ec6f37b0a8f7f3",blob_oid:"285542bc1d82144321ec38aa6f1a5cc7cfef5178",authority:"SUPPORTED_FACTS_ALLOWLIST_ONLY"},
    {id:"P06_PHASE2_OBSERVATION_OUTPUT",commit:"73d48ffbed1794c6691ba59be006aa096dcfcb22",path:"reports/gates/p06/successors/observation-capability-v1-phase2/outputs/observation.txt",sha256:"547fccefdd2a26bde33c363d3efc3172a1cb7ebf425c865b872275e00933ee6d",blob_oid:"781028f3666cedf2ed05acd40379ac5cd71fc4de",authority:"SUPPORTED_FACTS_ALLOWLIST_ONLY"}
  ]
' "$truth" >/dev/null

validate_committed_history

test "$(shasum -a 256 "$default_task_1" | awk '{print $1}')" = 7d6ae063881b69b1117402cae45bbe2b50ae3609595e2aec424ed5097ba49f8e
jq -e '
  keys == ["acceptance","binding","controls","evidence","inputs","plan_id","provider_equivalence","result","schema_version","subject","task"] and
  .schema_version == "taskseal.p06.phase2-evidence-truthfulness-correction-v1.task-receipt.v3" and
  .plan_id == "P06-PHASE2-EVIDENCE-TRUTHFULNESS-CORRECTION-V1" and
  .task == 1 and .result == "accepted" and
  .acceptance.id == "P06-PHASE2-TRUTH-CORRECTION-T1-SHARED-BOUNDARY-V3" and
  .binding.scheme == "parent-bound-receipt.v3" and
  .binding.implementation_result_head == "29c5da46d80b79e91c548aaacc44958e95e37bcb" and
  .binding.receipt_commit_parent == .binding.implementation_result_head and
  .inputs.original_input_head == "c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71" and
  .inputs.plan_checkpoint_sha256 == "a8a11bfa9b7f83369041c03f217f6d389a97c7a1d84756228e65e9a1e61ab9bf" and
  .inputs.predecessor_receipt_sha256 == "968779a627cd65d69f27edfa39293fd5f998f50029904ba52a1347463578b2e2" and
  .controls.historical_runner_runtime_executed == false and
  .controls.network_access == "not invoked" and
  .controls.provider_or_codex_process == "not invoked" and
  .controls.credential_or_keychain_read == "not invoked"
' "$default_task_1" >/dev/null
test "$(git -C "$root" rev-parse "$task_1_commit^1")" = "$task_1_implementation"
test "$(git -C "$root" diff-tree --no-commit-id --name-only --no-renames -r "$task_1_commit")" = "$task_1_rel"
test "$(git -C "$root" show "$task_1_commit:$task_1_rel" | shasum -a 256 | awk '{print $1}')" = 7d6ae063881b69b1117402cae45bbe2b50ae3609595e2aec424ed5097ba49f8e
jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$default_task_1" | while IFS='	' read -r source_path source_sha; do
  test "$(git -C "$root" show "$task_1_implementation:$source_path" | shasum -a 256 | awk '{print $1}')" = "$source_sha"
  test "$(shasum -a 256 "$root/$source_path" | awk '{print $1}')" = "$source_sha"
done
task_1_subject=$(jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$default_task_1" | LC_ALL=C sort | shasum -a 256 | awk '{print $1}')
test "$task_1_subject" = "$(jq -r .subject.sha256 "$default_task_1")"
jq -c '.evidence[]' "$default_task_1" | while IFS= read -r evidence; do
  test "$(printf '%s\n' "$(printf '%s\n' "$evidence" | jq -r .output)" | shasum -a 256 | awk '{print $1}')" = "$(printf '%s\n' "$evidence" | jq -r .output_sha256)"
done
test "$(sh "$root/$successor_rel/test-boundary-validator.sh")" = P06_PHASE2_TRUTH_BOUNDARY_VALIDATOR_PASS
provider_output=$(P06_BOUNDARY_PROVIDER_CURRENT_OBJECT="$task_1_implementation" P06_BOUNDARY_TEST_CASE=provider-equivalence sh "$root/$successor_rel/test-boundary-validator.sh")
test "$provider_output" = "P06_PROVIDER_LOGIN_SEMANTICS_EQUIVALENT:c0f4d9a1013d16950b2bd9ff232d252f3c06bb196ec7e62d9c3155cda474c7a9
P06_PROVIDER_MODEL_BLOCK_UNCHANGED:e1e76237c95dff9352dea8a4ef1ff4637493d7139fea55d3bb1c0ae1295fe20e"

jq -e '
  keys == ["controls","historical_claim_authority","historical_receipts","plan_id","preserved_facts","primary_artifacts","schema_version","scope","task_1","uncertainty"] and
  .schema_version == "taskseal.p06.phase2-evidence-truthfulness-correction.v1" and
  .plan_id == "P06-PHASE2-EVIDENCE-TRUTHFULNESS-CORRECTION-V1" and
  .scope == {
    base_head:"c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71",
    historical_mutation:false,
    runtime_reconstruction:false,
    exclusive_write_set:[
      "scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh",
      "scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/**",
      "reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/**"
    ]
  } and
  .task_1 == {
    receipt_path:"reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/task-1.json",
    receipt_sha256:"7d6ae063881b69b1117402cae45bbe2b50ae3609595e2aec424ed5097ba49f8e",
    receipt_commit:"82e1acb883ebe4ba2f3f54ad57857b126c942409",
    implementation_result_head:"29c5da46d80b79e91c548aaacc44958e95e37bcb",
    acceptance_id:"P06-PHASE2-TRUTH-CORRECTION-T1-SHARED-BOUNDARY-V3"
  } and
  [.historical_receipts[].id] == ["P06_PHASE1_RECEIPT","P06_T8_ROOTED_DISPOSITION","P06_PHASE2_RECEIPT","P06_PHASE2_EVIDENCE_CORRECTION","P06_PHASE2_EVIDENCE_CORRECTION_TASK1","P06_PHASE2_EVIDENCE_CORRECTION_TASK2","P06_PHASE2_EVIDENCE_CORRECTION_TASK3"] and
  [.historical_receipts[].commit] == ["d3c753458c32dc7bc4105f1deddf35d8d43fb5d2","73d48ffbed1794c6691ba59be006aa096dcfcb22","73d48ffbed1794c6691ba59be006aa096dcfcb22","c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71","c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71","c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71","c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71"] and
  [.historical_receipts[].path] == ["reports/gates/p06/successors/observation-capability-v1/phase-1.json","reports/gates/p06/task-8-rooted-disposition.json","reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json","reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/correction.json","reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-1.json","reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-2.json","reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-3.json"] and
  [.primary_artifacts[].id] == ["P06_PHASE2_PREFLIGHT_OUTPUT","P06_PHASE2_OBSERVATION_OUTPUT"] and
  .primary_artifacts == [
    {id:"P06_PHASE2_PREFLIGHT_OUTPUT",commit:"73d48ffbed1794c6691ba59be006aa096dcfcb22",path:"reports/gates/p06/successors/observation-capability-v1-phase2/outputs/preflight.txt",sha256:"94c0c0172cf8fae3b52add679f328307820b67641ae9350cd3ec6f37b0a8f7f3",blob_oid:"285542bc1d82144321ec38aa6f1a5cc7cfef5178",authority:"SUPPORTED_FACTS_ALLOWLIST_ONLY"},
    {id:"P06_PHASE2_OBSERVATION_OUTPUT",commit:"73d48ffbed1794c6691ba59be006aa096dcfcb22",path:"reports/gates/p06/successors/observation-capability-v1-phase2/outputs/observation.txt",sha256:"547fccefdd2a26bde33c363d3efc3172a1cb7ebf425c865b872275e00933ee6d",blob_oid:"781028f3666cedf2ed05acd40379ac5cd71fc4de",authority:"SUPPORTED_FACTS_ALLOWLIST_ONLY"}
  ] and
  .preserved_facts == {login_result:"LOGIN_REFUSED",qualification:"NOT_QUALIFIED",progression:"STOPPED_AT_T8",model_counter:"UNUSED",model_process_exit:"NOT_STARTED",original_tasks_9_13:"FORBIDDEN"} and
  .uncertainty.credential_retention.state == "UNKNOWN" and .uncertainty.credential_retention.verification == "UNVERIFIED" and
  .uncertainty.credential_retention.reason == "No independent primary artifact proves post-attempt credential retention state." and
  .uncertainty.protected_state_equality.state == "UNKNOWN" and .uncertainty.protected_state_equality.verification == "UNVERIFIED" and
  .uncertainty.protected_state_equality.reason == "No independent before/after artifact proves equality of protected, auth, binary, or worktree state." and
  .uncertainty.cleanup.state == "UNKNOWN" and .uncertainty.cleanup.verification == "UNVERIFIED" and
  .uncertainty.cleanup.reason == "No independent primary artifact proves temporary-state and raw-output cleanup." and
  [.uncertainty.credential_retention.historical_claims[],.uncertainty.protected_state_equality.historical_claims[],.uncertainty.cleanup.historical_claims[]] == [
    "reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#/controls/credential_retained",
    "reports/gates/p06/successors/observation-capability-v1/phase-1.json#/controls/owner_config_mutated",
    "reports/gates/p06/task-8-rooted-disposition.json#/rooted_attempt/observed/binary_unchanged",
    "reports/gates/p06/task-8-rooted-disposition.json#/rooted_attempt/observed/user_auth_config_unchanged",
    "reports/gates/p06/task-8-rooted-disposition.json#/rooted_attempt/observed/worktree_unchanged",
    "reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#/controls/binary_unchanged",
    "reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#/controls/protected_state_unchanged",
    "reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#/controls/worktree_unchanged",
    "reports/gates/p06/successors/observation-capability-v1/phase-1.json#/controls/temporary_home_removed",
    "reports/gates/p06/successors/observation-capability-v1/phase-1.json#/controls/raw_output_removed",
    "reports/gates/p06/task-8-rooted-disposition.json#/rooted_attempt/observed/temporary_auth_removed",
    "reports/gates/p06/task-8-rooted-disposition.json#/rooted_attempt/observed/raw_output_removed",
    "reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#/controls/temporary_runtime_removed",
    "reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#/controls/raw_output_removed"
  ] and
  .historical_claim_authority.receipt_bytes == "AUTHORITATIVE_FOR_PROVENANCE_ONLY" and
  .historical_claim_authority.unsupported_booleans == "NON_AUTHORITATIVE" and
  .historical_claim_authority.rule == "Byte binding preserves what a historical receipt said; it does not verify an unsupported runtime boolean." and
  (.historical_claim_authority.json_pointers | length) == 20 and
  (.historical_claim_authority.json_pointers | sort) == ([
    .uncertainty.credential_retention.historical_claims[],
    .uncertainty.protected_state_equality.historical_claims[],
    .uncertainty.cleanup.historical_claims[],
    "reports/gates/p06/successors/observation-capability-v1/phase-1.json#/controls/network_denied",
    "reports/gates/p06/successors/observation-capability-v1/phase-1.json#/controls/provider_route_started",
    "reports/gates/p06/successors/observation-capability-v1/phase-1.json#/controls/owner_auth_read",
    "reports/gates/p06/task-8-rooted-disposition.json#/root_discovery/forbidden_ambient",
    "reports/gates/p06/task-8-rooted-disposition.json#/root_discovery/native_evidence",
    "reports/gates/p06/task-8-rooted-disposition.json#/rooted_attempt/observed/forbidden_ambient_observed"
  ] | unique) and
  .controls == {offline_only:true,historical_runner_executed:false,network_access:"NOT_INVOKED",provider_or_codex_process:"NOT_INVOKED",credential_or_keychain_read:"NOT_INVOKED",original_tasks_9_13_executed:false}
' "$truth" >/dev/null

jq -c '.historical_receipts[]' "$truth" | while IFS= read -r object; do
  object_commit=$(printf '%s\n' "$object" | jq -r .commit)
  object_path=$(printf '%s\n' "$object" | jq -r .path)
  object_sha=$(printf '%s\n' "$object" | jq -r .sha256)
  object_oid=$(printf '%s\n' "$object" | jq -r .blob_oid)
  test "$(git -C "$root" rev-parse "$object_commit:$object_path")" = "$object_oid"
  test "$(git -C "$root" show "$object_commit:$object_path" | shasum -a 256 | awk '{print $1}')" = "$object_sha"
  test "$(shasum -a 256 "$root/$object_path" | awk '{print $1}')" = "$object_sha"
done

derived_boolean_pointers=$({
  jq -r --arg prefix 'reports/gates/p06/successors/observation-capability-v1/phase-1.json#' 'paths(type == "boolean") | $prefix + "/" + (map(tostring) | join("/"))' \
    "$root/reports/gates/p06/successors/observation-capability-v1/phase-1.json"
  jq -r --arg prefix 'reports/gates/p06/task-8-rooted-disposition.json#' 'paths(type == "boolean") | $prefix + "/" + (map(tostring) | join("/"))' \
    "$root/reports/gates/p06/task-8-rooted-disposition.json"
  jq -r --arg prefix 'reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json#' 'paths(type == "boolean") | $prefix + "/" + (map(tostring) | join("/"))' \
    "$root/reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json"
} | LC_ALL=C sort)
declared_boolean_pointers=$(jq -r '.historical_claim_authority.json_pointers[]' "$truth" | LC_ALL=C sort)
test "$(printf '%s\n' "$derived_boolean_pointers" | awk 'NF { count++ } END { print count+0 }')" = 20
test "$declared_boolean_pointers" = "$derived_boolean_pointers"

jq -c '.primary_artifacts[]' "$truth" | while IFS= read -r object; do
  object_commit=$(printf '%s\n' "$object" | jq -r .commit)
  object_path=$(printf '%s\n' "$object" | jq -r .path)
  object_sha=$(printf '%s\n' "$object" | jq -r .sha256)
  object_oid=$(printf '%s\n' "$object" | jq -r .blob_oid)
  test "$(git -C "$root" rev-parse "$object_commit:$object_path")" = "$object_oid"
  test "$(git -C "$root" show "$object_commit:$object_path" | shasum -a 256 | awk '{print $1}')" = "$object_sha"
  test "$(shasum -a 256 "$root/$object_path" | awk '{print $1}')" = "$object_sha"
done

observation="$root/reports/gates/p06/successors/observation-capability-v1-phase2/outputs/observation.txt"
fact_value() {
  awk -F= -v wanted="$1" '$1 == wanted { print substr($0, index($0, "=") + 1); found++ } END { if (found != 1) exit 2 }' "$observation"
}
test "$(fact_value login_result)" = "$(jq -r .preserved_facts.login_result "$truth")"
test "$(fact_value qualification)" = "$(jq -r .preserved_facts.qualification "$truth")"
test "$(fact_value original_progression)" = "$(jq -r .preserved_facts.progression "$truth")"
test "$(fact_value model_counter)" = "$(jq -r .preserved_facts.model_counter "$truth")"
test "$(fact_value model_process_exit)" = "$(jq -r .preserved_facts.model_process_exit "$truth")"
test "$(fact_value original_tasks_9_13)" = "$(jq -r .preserved_facts.original_tasks_9_13 "$truth")"

resolve_parent_bound_receipt() {
  receipt=$1
  receipt_rel=$2
  task_number=$3
  implementation_head=$(jq -r .binding.implementation_result_head "$receipt")
  test "$(jq -r .binding.receipt_commit_parent "$receipt")" = "$implementation_head"
  test "$(jq -r .binding.implementation_tree "$receipt")" = "$(git -C "$root" rev-parse "$implementation_head^{tree}")"
  test "$(jq -r .task "$receipt")" = "$task_number"
  receipt_blob=$(git -C "$root" hash-object "$receipt")
  receipt_commits=$(git -C "$root" rev-list --reverse "$base..$head" | while IFS= read -r commit; do
    parent_line=$(git -C "$root" rev-list --parents -n 1 "$commit")
    test "$(printf '%s\n' "$parent_line" | awk '{print NF}')" = 2 || continue
    test "$(printf '%s\n' "$parent_line" | awk '{print $2}')" = "$implementation_head" || continue
    test "$(git -C "$root" diff-tree --no-commit-id --name-only --no-renames -r "$commit")" = "$receipt_rel" || continue
    test "$(git -C "$root" rev-parse "$commit:$receipt_rel")" = "$receipt_blob" || continue
    printf '%s\n' "$commit"
  done)
  test "$(printf '%s\n' "$receipt_commits" | awk 'NF { count++ } END { print count+0 }')" = 1
  printf '%s\n' "$receipt_commits"
}

validate_task_2() {
  test -f "$default_task_2"
  jq -e '
    keys == ["acceptance","binding","controls","evidence","inputs","plan_id","result","schema_version","subject","task"] and
    .schema_version == "taskseal.p06.phase2-evidence-truthfulness-correction-v1.task-receipt.v2" and
    .plan_id == "P06-PHASE2-EVIDENCE-TRUTHFULNESS-CORRECTION-V1" and
    .task == 2 and .result == "accepted" and
    .acceptance == {id:"P06-PHASE2-TRUTH-CORRECTION-T2-TRUTHFUL-HISTORY-V1",operator_result:"The sole offline successor gate preserves only LOGIN_REFUSED, NOT_QUALIFIED, STOPPED_AT_T8, an unused/not-started model process and forbidden T9-T13 while classifying unsupported retention, protected-state and cleanup claims UNKNOWN and UNVERIFIED.",mutation_pass_marker:"P06_PHASE2_TRUTHFULNESS_MUTATIONS_PASS"} and
    .binding.scheme == "parent-bound-receipt.v2" and
    .binding.input_head == "2ba9e0fb92d82632b382296c1cb07067d8432879" and
    .binding.replaces_receipt_commit == "2ba9e0fb92d82632b382296c1cb07067d8432879" and
    .binding.receipt_commit_parent == .binding.implementation_result_head and
    (.binding.implementation_result_head | test("^[0-9a-f]{40}$")) and
    (.binding.implementation_tree | test("^[0-9a-f]{40}$")) and
    .inputs.plan_checkpoint_sha256 == "a8a11bfa9b7f83369041c03f217f6d389a97c7a1d84756228e65e9a1e61ab9bf" and
    .inputs.predecessor_checkpoint == "c54284cb3c2a2cfb7fb8508c5eef35204fd8ed71" and
    .inputs.predecessor_receipt_sha256 == "968779a627cd65d69f27edfa39293fd5f998f50029904ba52a1347463578b2e2" and
    .inputs.task_1_receipt_sha256 == "7d6ae063881b69b1117402cae45bbe2b50ae3609595e2aec424ed5097ba49f8e" and
    .inputs.superseded_task_2_receipt_sha256 == "f87eb90933b234c6e8ad6ddddcd2697c9fb92fe06ea5930df3f384a77063d4d0" and
    .subject.algorithm == "sha256 of sorted path, tab, sha256, newline source records" and
    [.subject.sources[].path] == [
      "reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/truthfulness.json",
      "scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh",
      "scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/verify.sh"
    ] and
    [.evidence[].id] == ["P06-PHASE2-TRUTH-CORRECTION-T2-RED-MISSING-VERIFIER-V1","P06-PHASE2-TRUTH-CORRECTION-T2-GREEN-MUTATIONS-PASS-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-BOOLEAN-CENSUS-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-AUTHORITY-RULE-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-CREDENTIAL-REASON-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-PROTECTED-REASON-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-CLEANUP-REASON-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-PREFLIGHT-COMMIT-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-OBSERVATION-COMMIT-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-PREFLIGHT-PATH-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-RED-OBSERVATION-PATH-V1","P06-PHASE2-TRUTH-CORRECTION-T2-REVIEW1-GREEN-MUTATIONS-PASS-V1"] and
    [.evidence[].command] == ["sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=boolean_inventory_bound_source sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=authority_rule sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=credential_reason sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=protected_reason sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=cleanup_reason sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=primary_preflight_commit sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=primary_observation_commit sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=primary_preflight_path sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","P06_TRUTH_TEST_CASE=primary_observation_path sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh","sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/test-verify.sh"] and
    [.evidence[].exit] == [1,0,1,1,1,1,1,1,1,1,1,0] and
    [.evidence[].output] == ["P06_TRUTH_RED_VERIFIER_MISSING","P06_PHASE2_TRUTHFULNESS_MUTATIONS_PASS","P06_TRUTH_EXPECTED_REFUSAL_MISSING:boolean_inventory_bound_source","P06_TRUTH_EXPECTED_REFUSAL_MISSING:authority_rule","P06_TRUTH_EXPECTED_REFUSAL_MISSING:credential_reason","P06_TRUTH_EXPECTED_REFUSAL_MISSING:protected_reason","P06_TRUTH_EXPECTED_REFUSAL_MISSING:cleanup_reason","P06_TRUTH_EXPECTED_REFUSAL_MISSING:primary_preflight_commit","P06_TRUTH_EXPECTED_REFUSAL_MISSING:primary_observation_commit","P06_TRUTH_EXPECTED_REFUSAL_MISSING:primary_preflight_path","P06_TRUTH_EXPECTED_REFUSAL_MISSING:primary_observation_path","P06_PHASE2_TRUTHFULNESS_MUTATIONS_PASS"] and
    .controls == {fixtures_only:true,network_access:"not invoked",provider_or_codex_process:"not invoked",credential_or_keychain_read:"not invoked",historical_receipt_mutation:false,original_tasks_9_13_executed:false}
  ' "$default_task_2" >/dev/null
  implementation_head=$(jq -r .binding.implementation_result_head "$default_task_2")
  test "$(git -C "$root" rev-parse 2ba9e0fb92d82632b382296c1cb07067d8432879^1)" = 3c6368249ad47b7876cf713056050e8b567d4884
  test "$(git -C "$root" diff-tree --no-commit-id --name-only --no-renames -r 2ba9e0fb92d82632b382296c1cb07067d8432879)" = "$task_2_rel"
  test "$(git -C "$root" show 2ba9e0fb92d82632b382296c1cb07067d8432879:$task_2_rel | shasum -a 256 | awk '{print $1}')" = f87eb90933b234c6e8ad6ddddcd2697c9fb92fe06ea5930df3f384a77063d4d0
  git -C "$root" merge-base --is-ancestor 2ba9e0fb92d82632b382296c1cb07067d8432879 "$implementation_head"
  git -C "$root" rev-list --reverse 2ba9e0fb92d82632b382296c1cb07067d8432879.."$implementation_head" | while IFS= read -r implementation_commit; do
    test -z "$(git -C "$root" rev-list --min-parents=2 -n 1 "$implementation_commit")"
    git -C "$root" diff-tree --no-commit-id --name-only --no-renames -r "$implementation_commit" | while IFS= read -r implementation_path; do
      jq -e --arg path "$implementation_path" '.subject.sources | map(.path) | index($path) != null' "$default_task_2" >/dev/null
    done
  done
  task_2_commit=$(resolve_parent_bound_receipt "$default_task_2" "$task_2_rel" 2)
  jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$default_task_2" | while IFS='	' read -r source_path source_sha; do
    test "$(git -C "$root" show "$implementation_head:$source_path" | shasum -a 256 | awk '{print $1}')" = "$source_sha"
  done
  subject_sha=$(jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$default_task_2" | LC_ALL=C sort | shasum -a 256 | awk '{print $1}')
  test "$subject_sha" = "$(jq -r .subject.sha256 "$default_task_2")"
  test "$(jq -r '.subject.sources[] | select(.path == "reports/gates/p06/successors/observation-capability-v1-phase2-evidence-truthfulness-correction-v1/truthfulness.json") | .sha256' "$default_task_2")" = "$(jq -r .inputs.truthfulness_sha256 "$default_task_2")"
  jq -c '.evidence[]' "$default_task_2" | while IFS= read -r evidence; do
    test "$(printf '%s\n' "$(printf '%s\n' "$evidence" | jq -r .output)" | shasum -a 256 | awk '{print $1}')" = "$(printf '%s\n' "$evidence" | jq -r .output_sha256)"
  done
  printf '%s\n' "$task_2_commit"
}

case "$stage" in
  task-2-precommit) ;;
  task-2)
    task_2_commit=$(validate_task_2)
    test "$head" = "$task_2_commit"
    ;;
  task-3-precommit|final)
    validate_task_2 >/dev/null
    ;;
esac

printf '%s\n' P06_PHASE2_EVIDENCE_TRUTHFULNESS_CORRECTION_PASS

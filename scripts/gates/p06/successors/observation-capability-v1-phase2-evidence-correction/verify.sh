#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
base=73d48ffbed1794c6691ba59be006aa096dcfcb22
task_1_commit=09ec751eddc33b5c941393503271de0afcb70744
correction_rel=reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/correction.json
task_1_rel=reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-1.json
task_2_rel=reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-2.json
task_3_rel=reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-3.json
default_correction="$root/$correction_rel"
default_task_1="$root/$task_1_rel"
default_task_2="$root/$task_2_rel"
default_task_3="$root/$task_3_rel"
correction=${P06_PHASE2_CORRECTION_RECEIPT:-$default_correction}
task_1=${P06_PHASE2_CORRECTION_TASK_1_RECEIPT:-$default_task_1}
stage=${P06_PHASE2_CORRECTION_STAGE:-final}

case "$stage" in
  task-2-bootstrap|task-2|task-3-precommit|final) ;;
  *) exit 2 ;;
esac

test "$(pwd -P)" = "$root"
test "$(git -C "$root" rev-parse --show-toplevel)" = "$root"
test "$(git -C "$root" branch --show-current)" = feat/p06-phase2-evidence-correction
head=$(git -C "$root" rev-parse HEAD)
git -C "$root" merge-base --is-ancestor "$base" "$head"

checkpoint_path=$(jq -r .plan_checkpoint_path "$root/.taskseal-dev/execution-authority.json")
jq -e --arg root "$root" --arg base "$base" '
  .schema_version == "taskseal.execution-authority.v2" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
  .allowed_task_first == 1 and .allowed_task_last == 3 and
  .repository_realpath == $root and .worktree_realpath == $root and
  .branch == "feat/p06-phase2-evidence-correction" and .head == $base and
  .plan_checkpoint_sha256 == "1cc0164fe6aec201f69b09da212a7fbd79d19f5cef7d96eadde1dc68548fd07b" and
  .allowed_external_actions == [] and
  (.forbidden_actions | index("network access")) != null and
  (.forbidden_actions | index("provider process")) != null and
  (.forbidden_actions | index("Codex process")) != null and
  (.forbidden_actions | index("credential read")) != null and
  (.forbidden_actions | index("Keychain access")) != null and
  (.forbidden_actions | index("main mutation")) != null
' "$root/.taskseal-dev/execution-authority.json" >/dev/null
test "$(shasum -a 256 "$checkpoint_path" | awk '{print $1}')" = 1cc0164fe6aec201f69b09da212a7fbd79d19f5cef7d96eadde1dc68548fd07b

bootstrap_fixture() {
  test "$stage" = task-2-bootstrap || exit 2
  fixture=$(realpath "$1")
  case "$fixture" in
    /private/tmp/taskseal-p06-phase2-correction-verify.*/*) ;;
    *) exit 2 ;;
  esac
  printf '%s\n' "$fixture"
}

if test "$correction" != "$default_correction"; then
  correction=$(bootstrap_fixture "$correction")
fi
if test "$task_1" != "$default_task_1"; then
  task_1=$(bootstrap_fixture "$task_1")
fi
if test -n "${P06_PHASE2_CORRECTION_CHANGED_PATHS_FILE:-}"; then
  changed_paths_file=$(bootstrap_fixture "$P06_PHASE2_CORRECTION_CHANGED_PATHS_FILE")
else
  changed_paths_file=
fi
if test -n "${P06_PHASE2_RECEIPT:-}"; then
  test "$stage" = task-2-bootstrap
  phase_2_override=$(realpath "$P06_PHASE2_RECEIPT")
  case "$phase_2_override" in
    /private/tmp/taskseal-p06-phase2-verify.*/*.json) ;;
    *) exit 2 ;;
  esac
else
  phase_2_override=
fi

validate_write_set() {
  while IFS= read -r changed_path; do
    test -n "$changed_path" || continue
    case "$changed_path" in
      scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/*) ;;
      reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/*) ;;
      *) printf 'P06_PHASE2_CORRECTION_WRITE_SET_REFUSED:%s\n' "$changed_path" >&2; exit 2 ;;
    esac
  done
}

if test -n "$changed_paths_file"; then
  validate_write_set <"$changed_paths_file"
else
  {
    git -C "$root" diff --name-only "$base..$head"
    git -C "$root" diff --name-only
    git -C "$root" diff --cached --name-only
    git -C "$root" ls-files --others --exclude-standard
  } | LC_ALL=C sort -u | validate_write_set
fi

jq -e '
  keys == ["classification","controls","historical","plan_id","predecessor","schema_version","scope","task_1"] and
  .schema_version == "taskseal.p06.phase2-evidence-correction.v1" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
  .scope == {
    base_head:"73d48ffbed1794c6691ba59be006aa096dcfcb22",
    exclusive_write_set:[
      "scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/**",
      "reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/**"
    ],
    historical_mutation:false
  } and
  .predecessor == {
    checkpoint:"73d48ffbed1794c6691ba59be006aa096dcfcb22",
    branch:"feat/p06-codex-observation-capability-v1-phase2",
    gate_path:"scripts/gates/p06/successors/observation-capability-v1-phase2/verify.sh",
    gate_sha256:"2e597069bb9fc3287e88ccc18071123e768d488697128c5ff6c9debaa0b3ece9",
    gate_blob_oid:"6df1553f643ba8ad9918a90fffc468e51eefc3b6",
    receipt_path:"reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json",
    receipt_sha256:"2a2e51fc2e97d657aa1fc7d413304f7b11362791abf4e354c3039406c5f1dfed",
    receipt_blob_oid:"53f2424669815d64136745194703a7be48d77c8d",
    pass_marker:"P06_CODEX_OBSERVATION_CAPABILITY_V1_PHASE2_PASS"
  } and
  .task_1 == {
    receipt_path:"reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-1.json",
    receipt_sha256:"403a73a8513d84db52a91e386549c2348b7e8d7906aaccb4d5d872416e742346",
    receipt_blob_oid:"3e5d403ab51e961698a088fc77aca563535b0700",
    receipt_commit:"09ec751eddc33b5c941393503271de0afcb70744",
    implementation_result_head:"f2a018aee29074199edbc2ee231d00e077249d84",
    subject_sha256:"6f4173333eb04ea4ddae6ef87c8276d943fe163628170ae4de57ef0df42eb553",
    acceptance_id:"P06-PHASE2-CORRECTION-T1-SYNTHETIC-BOUNDARY-V1"
  } and
  .historical == [
    {id:"P06_PHASE1_RECEIPT",commit:"d3c753458c32dc7bc4105f1deddf35d8d43fb5d2",path:"reports/gates/p06/successors/observation-capability-v1/phase-1.json",sha256:"7e0db9d70c99976e7666d6a9a4ecc903462d821620f8d862cd397a50c2136115",blob_oid:"a1bb617da344b99f5be0db56c068ba25ad47d2e2"},
    {id:"P06_T8_ROOTED_DISPOSITION",commit:"73d48ffbed1794c6691ba59be006aa096dcfcb22",path:"reports/gates/p06/task-8-rooted-disposition.json",sha256:"250bd5a4d1591213ebd4ea145991572da5a0356aed777e179a74ae25db899d18",blob_oid:"706057bdeb50eeae4a3ffa557a6141b6b00eda68"},
    {id:"P06_PHASE2_RECEIPT",commit:"73d48ffbed1794c6691ba59be006aa096dcfcb22",path:"reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json",sha256:"2a2e51fc2e97d657aa1fc7d413304f7b11362791abf4e354c3039406c5f1dfed",blob_oid:"53f2424669815d64136745194703a7be48d77c8d"}
  ] and
  .classification == {qualification:"NOT_QUALIFIED",progression:"STOPPED_AT_T8",original_tasks_9_13:"FORBIDDEN",phase_2_result:"SEALED_NEGATIVE",independent_review:"CORRECTION_PENDING"} and
  .controls == {fixtures_only:true,privacy:"NO_CREDENTIAL_BYTES",protected_state:"HISTORICAL_ATTESTATION_BOUND",network_access:"NOT_INVOKED",provider_or_codex_process:"NOT_INVOKED",credential_or_keychain_read:"NOT_INVOKED"}
' "$correction" >/dev/null

jq -c '.historical[]' "$correction" | while IFS= read -r object; do
  object_commit=$(printf '%s\n' "$object" | jq -r .commit)
  object_path=$(printf '%s\n' "$object" | jq -r .path)
  object_sha=$(printf '%s\n' "$object" | jq -r .sha256)
  object_oid=$(printf '%s\n' "$object" | jq -r .blob_oid)
  test "$(git -C "$root" rev-parse "$object_commit:$object_path")" = "$object_oid"
  test "$(git -C "$root" show "$object_commit:$object_path" | shasum -a 256 | awk '{print $1}')" = "$object_sha"
  test "$(shasum -a 256 "$root/$object_path" | awk '{print $1}')" = "$object_sha"
done

predecessor_branch=feat/p06-codex-observation-capability-v1-phase2
predecessor_root=$(git -C "$root" worktree list --porcelain | awk -v wanted="branch refs/heads/$predecessor_branch" '
  $1 == "worktree" { candidate=$2 }
  $0 == wanted { print candidate }
')
test "$(printf '%s\n' "$predecessor_root" | wc -l | tr -d ' ')" = 1
test -n "$predecessor_root"
test "$(git -C "$predecessor_root" branch --show-current)" = "$predecessor_branch"
test "$(git -C "$predecessor_root" rev-parse HEAD)" = "$base"
test -z "$(git -C "$predecessor_root" status --short)"
predecessor_gate="$predecessor_root/$(jq -r .predecessor.gate_path "$correction")"
test "$(shasum -a 256 "$predecessor_gate" | awk '{print $1}')" = "$(jq -r .predecessor.gate_sha256 "$correction")"
test "$(git -C "$root" rev-parse "$base:$(jq -r .predecessor.gate_path "$correction")")" = "$(jq -r .predecessor.gate_blob_oid "$correction")"
if test -n "$phase_2_override"; then
  predecessor_output=$(CDPATH='' cd -- "$predecessor_root" && P06_PHASE2_RECEIPT="$phase_2_override" "$predecessor_gate")
else
  predecessor_output=$(CDPATH='' cd -- "$predecessor_root" && "$predecessor_gate")
fi
test "$predecessor_output" = "$(jq -r .predecessor.pass_marker "$correction")"

jq -e --arg root "$root" '
  keys == ["acceptance","binding","controls","evidence","inputs","plan_id","result","schema_version","subject","task"] and
  .schema_version == "taskseal.p06.phase2-evidence-correction.task-receipt.v1" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
  .task == 1 and .result == "accepted" and
  .acceptance.id == "P06-PHASE2-CORRECTION-T1-SYNTHETIC-BOUNDARY-V1" and
  .binding.scheme == "parent-bound-receipt.v1" and
  .binding.input_head == "73d48ffbed1794c6691ba59be006aa096dcfcb22" and
  .binding.implementation_result_head == "f2a018aee29074199edbc2ee231d00e077249d84" and
  .binding.receipt_commit_parent == .binding.implementation_result_head and
  .inputs.plan_checkpoint_sha256 == "1cc0164fe6aec201f69b09da212a7fbd79d19f5cef7d96eadde1dc68548fd07b" and
  .inputs.predecessor_receipt_sha256 == "2a2e51fc2e97d657aa1fc7d413304f7b11362791abf4e354c3039406c5f1dfed" and
  .subject.sha256 == "6f4173333eb04ea4ddae6ef87c8276d943fe163628170ae4de57ef0df42eb553" and
  .controls == {fixtures_only:true,network_access:"not invoked",provider_or_codex_process:"not invoked",credential_or_keychain_read:"not invoked",historical_phase_1_t8_phase_2_mutation:false}
' "$task_1" >/dev/null
test "$(git -C "$root" rev-parse "$task_1_commit^")" = "$(jq -r .binding.implementation_result_head "$task_1")"
test "$(git -C "$root" diff-tree --no-commit-id --name-only -r "$task_1_commit")" = "$task_1_rel"
test "$(git -C "$root" rev-parse "$task_1_commit:$task_1_rel")" = "$(jq -r .task_1.receipt_blob_oid "$correction")"
test "$(shasum -a 256 "$task_1" | awk '{print $1}')" = "$(jq -r .task_1.receipt_sha256 "$correction")"
jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$task_1" | while IFS='	' read -r source_path source_sha; do
  test "$(git -C "$root" show "$(jq -r .binding.implementation_result_head "$task_1"):$source_path" | shasum -a 256 | awk '{print $1}')" = "$source_sha"
done
task_1_subject=$(jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$task_1" | LC_ALL=C sort | shasum -a 256 | awk '{print $1}')
test "$task_1_subject" = "$(jq -r .subject.sha256 "$task_1")"
test "$(sh "$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/test-synthetic-boundary.sh")" = P06_PHASE2_CORRECTION_SYNTHETIC_PASS

validate_parent_bound_receipt() {
  receipt=$1
  receipt_rel=$2
  task_number=$3
  receipt_commit=$(git -C "$root" log --format=%H --diff-filter=A "$base..$head" -- "$receipt_rel")
  test "$(printf '%s\n' "$receipt_commit" | wc -l | tr -d ' ')" = 1
  test -n "$receipt_commit"
  implementation_head=$(jq -r .binding.implementation_result_head "$receipt")
  test "$(git -C "$root" rev-parse "$receipt_commit^")" = "$implementation_head"
  test "$(jq -r .binding.receipt_commit_parent "$receipt")" = "$implementation_head"
  test "$(git -C "$root" diff-tree --no-commit-id --name-only -r "$receipt_commit")" = "$receipt_rel"
  test "$(jq -r .task "$receipt")" = "$task_number"
}

validate_subject_sources() {
  receipt=$1
  result_head=$(jq -r .binding.implementation_result_head "$receipt")
  jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r source_path source_sha; do
    test "$(git -C "$root" show "$result_head:$source_path" | shasum -a 256 | awk '{print $1}')" = "$source_sha"
  done
  subject_sha=$(jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$receipt" | LC_ALL=C sort | shasum -a 256 | awk '{print $1}')
  test "$subject_sha" = "$(jq -r .subject.sha256 "$receipt")"
}

validate_task_2() {
  task_2=$default_task_2
  test -f "$task_2"
  jq -e '
    keys == ["acceptance","binding","controls","evidence","inputs","plan_id","result","schema_version","subject","task"] and
    .schema_version == "taskseal.p06.phase2-evidence-correction.task-receipt.v1" and
    .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
    .task == 2 and .result == "accepted" and
    .acceptance == {id:"P06-PHASE2-CORRECTION-T2-HISTORICAL-BINDING-V1",operator_result:"The sole correction gate chains the immutable Phase 2 verifier and rejects privacy, protected-state, write-set, historical-drift, receipt-binding and classification mutations.",mutation_pass_marker:"P06_PHASE2_CORRECTION_MUTATIONS_PASS"} and
    .binding.scheme == "parent-bound-receipt.v1" and
    .binding.input_head == "09ec751eddc33b5c941393503271de0afcb70744" and
    .binding.receipt_commit_parent == .binding.implementation_result_head and
    .inputs.plan_checkpoint_sha256 == "1cc0164fe6aec201f69b09da212a7fbd79d19f5cef7d96eadde1dc68548fd07b" and
    .inputs.predecessor_checkpoint == "73d48ffbed1794c6691ba59be006aa096dcfcb22" and
    .inputs.predecessor_receipt_sha256 == "2a2e51fc2e97d657aa1fc7d413304f7b11362791abf4e354c3039406c5f1dfed" and
    .inputs.task_1_receipt_sha256 == "403a73a8513d84db52a91e386549c2348b7e8d7906aaccb4d5d872416e742346" and
    .inputs.correction_receipt_sha256 == "8e039c31d16ff355cf0cdcd35f5113a3cb08e4210b77ea46e43877e42573d818" and
    .subject.algorithm == "sha256 of sorted path, tab, sha256, newline source records" and
    [.subject.sources[].path] == [
      "reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/correction.json",
      "scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/test-verify.sh",
      "scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh"
    ] and
    .controls == {fixtures_only:true,network_access:"not invoked",provider_or_codex_process:"not invoked",credential_or_keychain_read:"not invoked",historical_phase_1_t8_phase_2_mutation:false} and
    [.evidence[].id] == ["P06-PHASE2-CORRECTION-T2-RED-MISSING-VERIFIER-V1","P06-PHASE2-CORRECTION-T2-GREEN-MUTATIONS-PASS-V1"] and
    .evidence[0].command == "sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/test-verify.sh" and
    .evidence[0].exit == 1 and
    .evidence[0].output_sha256 == "2a1e0389d3210a2f08f0adb6b1522f0b1fac375f7bafe2ff3e194395c78245ec" and
    .evidence[1].command == "sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/test-verify.sh" and
    .evidence[1].exit == 0 and .evidence[1].output == "P06_PHASE2_CORRECTION_MUTATIONS_PASS"
  ' "$task_2" >/dev/null
  validate_parent_bound_receipt "$task_2" "$task_2_rel" 2
  validate_subject_sources "$task_2"
  test "$(printf '%s\n' "$(jq -r .evidence[0].output "$task_2")" | shasum -a 256 | awk '{print $1}')" = "$(jq -r .evidence[0].output_sha256 "$task_2")"
  test "$(printf '%s\n' "$(jq -r .evidence[1].output "$task_2")" | shasum -a 256 | awk '{print $1}')" = "$(jq -r .evidence[1].output_sha256 "$task_2")"
}

validate_task_3() {
  task_3=$default_task_3
  test -f "$task_3"
  jq -e --arg status_path "$(jq -r .status_path "$root/.taskseal-dev/execution-authority.json")" --arg dashboard_path "$(jq -r .dashboard_path "$root/.taskseal-dev/execution-authority.json")" --arg worklog_path "$(jq -r .worklog_path "$root/.taskseal-dev/execution-authority.json")" '
    keys == ["acceptance","binding","controls","evidence","inputs","plan_id","result","schema_version","subject","task"] and
    .schema_version == "taskseal.p06.phase2-evidence-correction.task-receipt.v1" and
    .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
    .task == 3 and .result == "accepted" and
    .acceptance == {id:"P06-PHASE2-CORRECTION-T3-OWNER-EVIDENCE-V1",operator_result:"Owner evidence reports P06 7/13 as 53%, removes premature correction REV PASS, and preserves the sealed-negative NOT_QUALIFIED disposition.",current_progress:{completed:7,total:13,percent:53},independent_review:"PENDING"} and
    .binding.scheme == "parent-bound-receipt.v1" and
    .binding.input_head == .binding.implementation_result_head and
    .binding.receipt_commit_parent == .binding.implementation_result_head and
    .inputs.plan_checkpoint_sha256 == "1cc0164fe6aec201f69b09da212a7fbd79d19f5cef7d96eadde1dc68548fd07b" and
    .controls == {owner_ssot_only:true,worklog_append_only:true,network_access:"not invoked",provider_or_codex_process:"not invoked",credential_or_keychain_read:"not invoked",historical_phase_1_t8_phase_2_mutation:false} and
    [.subject.sources[].path] == [$dashboard_path,$status_path,$worklog_path] and
    [.evidence[].id] == ["P06-PHASE2-CORRECTION-T3-PRECOMMIT-GATE-PASS-V1"] and
    .evidence[0].command == "P06_PHASE2_CORRECTION_STAGE=task-3-precommit sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh" and
    .evidence[0].exit == 0 and .evidence[0].output == "P06_PHASE2_EVIDENCE_CORRECTION_PASS"
  ' "$task_3" >/dev/null
  task_2_sha=$(shasum -a 256 "$default_task_2" | awk '{print $1}')
  test "$(jq -r .inputs.task_2_receipt_sha256 "$task_3")" = "$task_2_sha"
  test "$(jq -r .inputs.task_1_receipt_sha256 "$task_3")" = "$(shasum -a 256 "$default_task_1" | awk '{print $1}')"
  test "$(jq -r .inputs.correction_receipt_sha256 "$task_3")" = "$(shasum -a 256 "$default_correction" | awk '{print $1}')"
  jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$task_3" | while IFS='	' read -r source_path source_sha; do
    test "$(shasum -a 256 "$source_path" | awk '{print $1}')" = "$source_sha"
  done
  subject_sha=$(jq -r '.subject.sources[] | [.path,.sha256] | @tsv' "$task_3" | LC_ALL=C sort | shasum -a 256 | awk '{print $1}')
  test "$subject_sha" = "$(jq -r .subject.sha256 "$task_3")"
  test "$(printf '%s\n' "$(jq -r .evidence[0].output "$task_3")" | shasum -a 256 | awk '{print $1}')" = "$(jq -r .evidence[0].output_sha256 "$task_3")"
  if test "$stage" = task-3-precommit; then
    test "$(jq -r .binding.implementation_result_head "$task_3")" = "$head"
  else
    validate_parent_bound_receipt "$task_3" "$task_3_rel" 3
  fi
}

case "$stage" in
  task-2-bootstrap) ;;
  task-2) validate_task_2 ;;
  task-3-precommit|final) validate_task_2; validate_task_3 ;;
esac

if rg -n 'Bearer |sk-[A-Za-z0-9]{20,}|eyJ[A-Za-z0-9_-]{20,}|/Users/ysorokin/\.codex/auth\.json' "$correction" "$task_1"; then
  exit 2
fi

printf '%s\n' P06_PHASE2_EVIDENCE_CORRECTION_PASS

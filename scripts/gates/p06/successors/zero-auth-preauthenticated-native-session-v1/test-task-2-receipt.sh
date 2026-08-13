#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-2.json
receipt="$root/$receipt_rel"
input_head=2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6
parent_receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json

refuse() {
  printf '%s\n' "P06_ZERO_AUTH_RECEIPT_REFUSAL:$1" >&2
  exit 1
}

sha_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

sha_output() {
  printf '%s\n' "$1" | shasum -a 256 | awk '{print $1}'
}

test -f "$receipt" || refuse MISSING_RECEIPT
cd "$root"

implementation_head=$(jq -er '.binding.implementation_result_head' "$receipt") || refuse IMPLEMENTATION_HEAD
implementation_tree=$(jq -er '.binding.implementation_tree' "$receipt") || refuse IMPLEMENTATION_TREE

jq -e \
  --arg input_head "$input_head" \
  --arg implementation_head "$implementation_head" \
  --arg implementation_tree "$implementation_tree" '
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.task-receipt.v1" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .task == 2 and .result == "accepted" and
  .acceptance == {
    id:"P06-ZERO-AUTH-T2-EXECUTABLE-PATH-CLOSURE-V1",
    operator_result:"Current Phase 2 and T8 credential/login launchers are non-executable unconditional historical-only refusals; executable product inventory refuses auth-file, credential extraction/copy, provider-login, OAuth/device/browser, and API-key/token-input source paths; the former auth fingerprint is historical provenance only and non-authoritative.",
    control_ids:["AUTH-01"],
    evidence_ids:[
      "P06-ZERO-AUTH-T2-RED-MISSING-SOURCE-INVENTORY-V1",
      "P06-ZERO-AUTH-T2-RED-AUTH-FINGERPRINT-SUPERSESSION-V1",
      "P06-ZERO-AUTH-T2-GREEN-EXECUTABLE-SOURCE-INVENTORY-V1"
    ]
  } and
  .binding == {
    scheme:"parent-bound-receipt.v1",
    input_head:$input_head,
    implementation_result_head:$implementation_head,
    implementation_tree:$implementation_tree,
    receipt_commit_parent:$implementation_head,
    parent_task_receipt:{
      path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json",
      commit:$input_head,
      blob_oid:"20d4c618732d42be403964e5530b6187c61ac7b3",
      sha256:"a20e8c793f99a6022a96d9e274c8c6685011e8762d90bee3afe1320f445df472"
    },
    resolution:"Resolve exactly one commit in input_head..tip whose task-2 receipt blob equals these bytes; it must be a single-parent receipt-only child of implementation_result_head. Repository subjects are read from that implementation commit, while the accepted Task 1 receipt is read from input_head."
  } and
  .inputs == {
    plan_checkpoint_path:"/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-13-p06-zero-auth-preauthenticated-native-session-v1.md",
    plan_checkpoint_sha256:"b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68",
    accepted_task_1_head:$input_head
  } and
  [.subject.sources[].path] == [
    "reports/contracts/provider-capability-truth.json",
    "scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory.rb",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-source-inventory.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-2-receipt-durability.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-2-receipt.sh",
    "scripts/gates/p06/t8-native-observe-once.sh"
  ] and
  ([.subject.sources[].path] | unique | length) == 7 and
  ([.subject.sources[] | select(.kind == "repository_implementation_commit")] | length) == 7 and
  .evidence == [
    {
      id:"P06-ZERO-AUTH-T2-RED-MISSING-SOURCE-INVENTORY-V1",
      command:"sh scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-source-inventory.sh",
      exit:1,
      output:"P06_ZERO_AUTH_RED_SOURCE_INVENTORY_MISSING",
      output_sha256:"4fd792b5d3f95496fa6f316f4f23aa3630775dbafbf2ee82ce548eaf2c530d97",
      meaning:"The focused test failed before an executable source inventory existed."
    },
    {
      id:"P06-ZERO-AUTH-T2-RED-AUTH-FINGERPRINT-SUPERSESSION-V1",
      command:"sh scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-source-inventory.sh",
      exit:1,
      output:"P06_ZERO_AUTH_PROVIDER_TRUTH_SUPERSESSION_REQUIRED",
      output_sha256:"fcb32a4925a5dd6b8092f79a1154a4567cfe354fed005d4f83aeefbb90abff0c",
      input:"Runnable launcher refusal and source inventory were present; provider-capability-truth.json was restored to its predecessor v1 bytes with current auth fingerprint claims.",
      meaning:"The focused contract refused until current auth fingerprints were removed and their predecessor object was classified as non-authoritative historical provenance."
    },
    {
      id:"P06-ZERO-AUTH-T2-GREEN-EXECUTABLE-SOURCE-INVENTORY-V1",
      command:"sh scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-source-inventory.sh",
      exit:0,
      output:"P06_ZERO_AUTH_SOURCE_INVENTORY_TEST_PASS",
      output_sha256:"ff2c843a18a4b76f59ff24589d0335a7057640cb926ea647f7246d40ae7b64af",
      meaning:"The offline focused suite executed both refusal stubs, checked their non-executable tracked modes and predecessor Git objects, accepted clean product source, and independently refused all required credential/login/browser/token fixture classes plus executable or contaminated historical stubs."
    }
  ] and
  .seal_tdd == {
    red_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-2-receipt.sh",
    red_exit:1,
    red_output:"P06_ZERO_AUTH_RECEIPT_REFUSAL:MISSING_RECEIPT",
    red_output_sha256:"cd7e1bd1379fcbc7b10ad08b273b5a3af02e625dddd39487663bb6cca26c93a0",
    green_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-2-receipt.sh",
    green_exit:0,
    green_output:"P06_ZERO_AUTH_TASK_2_RECEIPT_PASS",
    green_output_sha256:"fc86433c51cb122e21c7476f58017e8c04336c85b9b7c19d7bed8c1f4ad8d7c0"
  } and
  .controls == {
    historical_runners_executable:false,
    historical_runners_behavior:"HISTORICAL_ONLY_REFUSED",
    historical_git_objects_preserved:true,
    executable_product_source_inventory:"PASS",
    refused_classes:["AUTH_FILE","CREDENTIAL_EXTRACTION","CREDENTIAL_COPY","PROVIDER_LOGIN","BROWSER_AUTH","TOKEN_INPUT"],
    current_auth_fingerprint_claim:"ABSENT",
    historical_auth_fingerprint_authority:"NON_AUTHORITATIVE",
    network_access:"not invoked",
    provider_or_auth_process:"not invoked",
    browser_launch:"not invoked",
    credential_or_keychain_read:"not invoked",
    main_mutation:false
  }
  ' "$receipt" >/dev/null || refuse RECEIPT_CONTRACT

test "$(git rev-parse "$implementation_head^{tree}")" = "$implementation_tree" || refuse IMPLEMENTATION_TREE_MISMATCH
git merge-base --is-ancestor "$input_head" "$implementation_head" || refuse IMPLEMENTATION_LINEAGE
test "$(git rev-parse "$input_head:$parent_receipt_rel")" = "$(jq -r '.binding.parent_task_receipt.blob_oid' "$receipt")" || refuse PARENT_RECEIPT_BLOB
test "$(git show "$input_head:$parent_receipt_rel" | shasum -a 256 | awk '{print $1}')" = "$(jq -r '.binding.parent_task_receipt.sha256' "$receipt")" || refuse PARENT_RECEIPT_DIGEST

expected_paths='reports/contracts/provider-capability-truth.json
scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory.rb
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-source-inventory.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-2-receipt-durability.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-2-receipt.sh
scripts/gates/p06/t8-native-observe-once.sh'
test "$(git diff --name-only "$input_head..$implementation_head")" = "$expected_paths" || refuse IMPLEMENTATION_WRITE_SET
test "$(git ls-tree "$implementation_head" scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh | awk '{print $1}')" = 100644 || refuse PHASE2_MODE
test "$(git ls-tree "$implementation_head" scripts/gates/p06/t8-native-observe-once.sh | awk '{print $1}')" = 100644 || refuse T8_MODE

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-task2-receipt.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
receipt_blob=$(git hash-object "$receipt")
: >"$scratch/matching-commits"
for commit in $(git rev-list --reverse "$input_head..HEAD" -- "$receipt_rel"); do
  if test "$(git rev-parse "$commit:$receipt_rel")" = "$receipt_blob"; then
    printf '%s\n' "$commit" >>"$scratch/matching-commits"
  fi
done

match_count=$(wc -l <"$scratch/matching-commits" | tr -d ' ')
case "$match_count" in
  0)
    test "$(git rev-parse HEAD)" = "$implementation_head" || refuse UNCOMMITTED_RECEIPT_WRONG_HEAD
    test "$(git status --porcelain=v1 --untracked-files=all)" = "?? $receipt_rel" || refuse UNCOMMITTED_RECEIPT_WRITE_SET
    ;;
  1)
    receipt_commit=$(sed -n '1p' "$scratch/matching-commits")
    set -- $(git rev-list --parents -n 1 "$receipt_commit")
    test "$#" = 2 || refuse RECEIPT_PARENT_COUNT
    test "$2" = "$implementation_head" || refuse RECEIPT_PARENT
    test "$(git diff-tree --no-commit-id --name-only -r "$receipt_commit")" = "$receipt_rel" || refuse RECEIPT_NOT_RECEIPT_ONLY
    ;;
  *) refuse RECEIPT_COMMIT_NOT_UNIQUE ;;
esac

: >"$scratch/subject-records"
while IFS=$(printf '\t') read -r kind path expected_sha; do
  test "$kind" = repository_implementation_commit || refuse SUBJECT_KIND
  git cat-file -e "$implementation_head:$path" || refuse REPOSITORY_SUBJECT_MISSING
  actual_sha=$(git show "$implementation_head:$path" | shasum -a 256 | awk '{print $1}')
  test "$actual_sha" = "$expected_sha" || refuse SUBJECT_SOURCE_DIGEST
  printf '%s\t%s\n' "$path" "$actual_sha" >>"$scratch/subject-records"
done <<EOF
$(jq -r '.subject.sources[] | [.kind, .path, .sha256] | @tsv' "$receipt")
EOF
sort "$scratch/subject-records" >"$scratch/subject-records.sorted"
cmp -s "$scratch/subject-records" "$scratch/subject-records.sorted" || refuse SUBJECT_ORDER
test "$(sha_file "$scratch/subject-records")" = "$(jq -r '.subject.sha256' "$receipt")" || refuse SUBJECT_AGGREGATE

evidence_index=0
evidence_count=$(jq '.evidence | length' "$receipt")
while test "$evidence_index" -lt "$evidence_count"; do
  output=$(jq -r --argjson index "$evidence_index" '.evidence[$index].output' "$receipt")
  expected_sha=$(jq -r --argjson index "$evidence_index" '.evidence[$index].output_sha256' "$receipt")
  test "$(sha_output "$output")" = "$expected_sha" || refuse EVIDENCE_OUTPUT_DIGEST
  evidence_index=$((evidence_index + 1))
done
test "$(sha_output "$(jq -r '.seal_tdd.red_output' "$receipt")")" = "$(jq -r '.seal_tdd.red_output_sha256' "$receipt")" || refuse SEAL_RED_OUTPUT_DIGEST
test "$(sha_output "$(jq -r '.seal_tdd.green_output' "$receipt")")" = "$(jq -r '.seal_tdd.green_output_sha256' "$receipt")" || refuse SEAL_GREEN_OUTPUT_DIGEST

printf '%s\n' P06_ZERO_AUTH_TASK_2_RECEIPT_PASS

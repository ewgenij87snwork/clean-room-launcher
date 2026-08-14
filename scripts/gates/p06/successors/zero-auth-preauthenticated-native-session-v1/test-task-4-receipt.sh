#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json
receipt="$root/$receipt_rel"
manifest_rel=scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/entrypoint-inventory.json
input_head=6a54523b4dbf8869ca7035a7dc42df88ac11c45a
correction_input_head=707db75475879a95ccf53bc295088362e3f3a560
parent_receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-3.json

refuse() {
  printf 'P06_ZERO_AUTH_TASK_4_RECEIPT_REFUSAL:%s\n' "$1" >&2
  exit 1
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
  --arg correction_input_head "$correction_input_head" \
  --arg implementation_head "$implementation_head" \
  --arg implementation_tree "$implementation_tree" '
  keys == ["acceptance","binding","evidence","inputs","invariant","plan_id","preserved_controls","result","schema_version","seal_tdd","subject","task"] and
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.task-receipt.v2" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .task == 4 and .result == "accepted" and
  .acceptance == {
    id:"P06-ZERO-AUTH-T4-EXACT-CONTENT-BOUND-PUBLIC-PROBE-V2",
    control_ids:["ADP-05","AUTH-01","OD-10"],
    evidence_ids:[
      "P06-ZERO-AUTH-T4-FIX10-RED-EXACT-PUBLIC-PROBE-V2",
      "P06-ZERO-AUTH-T4-FIX10-GREEN-EXACT-PUBLIC-PROBE-V2",
      "P06-ZERO-AUTH-T4-FIX10-GREEN-PREAUTH-ZERO-CHILD-V2",
      "P06-ZERO-AUTH-T4-FIX10-RED-REPLACEMENT-RECEIPT-V2",
      "P06-ZERO-AUTH-T4-FIX10-GREEN-SOLE-GATE-V2"
    ]
  } and
  .binding == {
    scheme:"parent-bound-corrective-receipt.v3",
    input_head:$input_head,
    correction_input_head:$correction_input_head,
    implementation_result_head:$implementation_head,
    implementation_tree:$implementation_tree,
    receipt_commit_parent:$implementation_head,
    replaces_receipt_commit:$correction_input_head,
    predecessor_task_receipt:{
      path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json",
      commit:$correction_input_head,
      blob_oid:"d868cfabeed248ed4aa19c29aabb420f9d753927",
      sha256:"0a04f635e9d754b57caadbe263d594f3936dc41104b65450b5c98db46c050258"
    },
    parent_task_receipt:{
      path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-3.json",
      commit:$input_head,
      blob_oid:"08febe5afe412e00e46f32d4c915f94e6cbe209d",
      sha256:"8916cd8d268d91988931985ffb952b95fde491445f84ddbfa9a1c22352a68de8"
    }
  } and
  .inputs == {
    plan_checkpoint_path:"/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-13-p06-zero-auth-preauthenticated-native-session-v1.md",
    plan_checkpoint_sha256:"b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68",
    accepted_task_3_head:$input_head,
    corrective_base_head:$correction_input_head
  } and
  .invariant == {
    manifest:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/entrypoint-inventory.json",
    treeish:"COMMITTED_IMPLEMENTATION_TREE",
    path:"scripts/probe/provider-capabilities.sh",
    mode:"100755",
    object_type:"blob",
    blob_oid:"e3089827b02914093474b72607d7b6a41547316a",
    content_sha256:"bfa281f230109be7927e82e3e263beb7f890e87a1d15e9c1adbdf5e98f7f536b",
    purpose:"Observe provider capabilities only after opaque preauthenticated-session availability is established.",
    scope:"V0_1_PUBLIC_PROBE_EXECUTABLE",
    tracked_descendant_count:1,
    every_other_tracked_descendant:"REFUSED",
    every_tuple_or_content_mismatch:"REFUSED",
    generic_shell_analysis_claimed:false
  } and
  .preserved_controls == {
    missing_unavailable_ambiguous_exit:78,
    refusal_phase:"BEFORE_ROOT_FIXTURE_EXECUTABLE_DISCOVERY_OR_CHILD_BIRTH",
    fake_capture_child_birth:false,
    cli_identity_inventory:"PRESERVED",
    rust_identity_guard:"PRESERVED",
    source_privacy_public_boundary:"PRESERVED",
    local_continuity_transcripts:"PRESERVED"
  } and
  .subject.algorithm == "sha256 of sorted path, tab, sha256, newline correction records" and
  (.subject.sha256 | test("^[0-9a-f]{64}$")) and
  all(.subject.sources[]; .kind == "repository_implementation_commit" and (.path | type == "string") and (.sha256 | test("^[0-9a-f]{64}$"))) and
  [.evidence[].id] == .acceptance.evidence_ids and
  [.evidence[].command] == [
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-entrypoint-inventory.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-entrypoint-inventory.sh",
    "CARGO_INCREMENTAL=0 cargo test --offline --test provider_capability_truth provider_probe_requires_opaque_preauthentication_before_process_birth -- --exact --test-threads=1",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh"
  ] and
  [.evidence[].exit] == [1,0,0,1,0] and
  [.evidence[].output] == [
    "P06_ZERO_AUTH_EXPECTED_ENTRYPOINT_REFUSAL_MISSING:extra_python_descendant\nP06_ZERO_AUTH_EXPECTED_ENTRYPOINT_REFUSAL_MISSING:content_mutation\nP06_ZERO_AUTH_EXPECTED_PUBLIC_BOUNDARY_REFUSAL_MISSING:extra_python_descendant",
    "P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_TEST_PASS",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out",
    "P06_ZERO_AUTH_TASK_4_RECEIPT_REFUSAL:RECEIPT_CONTRACT",
    "P06_ZERO_AUTH_PREAUTHENTICATED_NATIVE_SESSION_V1_PASS"
  ] and
  .seal_tdd == {
    red_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    red_exit:1,
    red_output:"P06_ZERO_AUTH_TASK_4_RECEIPT_REFUSAL:RECEIPT_CONTRACT",
    red_output_sha256:"83b45149afb5e944ec80635c75db0137640b84eea964af5858a98e9f0400b735",
    green_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    green_exit:0,
    green_output:"P06_ZERO_AUTH_TASK_4_RECEIPT_PASS",
    green_output_sha256:"5817c91c2d32c47a64aa80641772d471b3b3f2261334e30262eb5c33fff4c30d"
  }
' "$receipt" >/dev/null || refuse RECEIPT_CONTRACT

test "$(git rev-parse "$implementation_head^{tree}")" = "$implementation_tree" || refuse IMPLEMENTATION_TREE
git merge-base --is-ancestor "$correction_input_head" "$implementation_head" || refuse CORRECTION_LINEAGE
test -z "$(git rev-list --min-parents=2 "$correction_input_head..$implementation_head")" || refuse IMPLEMENTATION_MERGE

test "$(git rev-parse "$correction_input_head:$receipt_rel")" = "$(jq -r '.binding.predecessor_task_receipt.blob_oid' "$receipt")" || refuse PREDECESSOR_RECEIPT_BLOB
test "$(git show "$correction_input_head:$receipt_rel" | shasum -a 256 | awk '{print $1}')" = "$(jq -r '.binding.predecessor_task_receipt.sha256' "$receipt")" || refuse PREDECESSOR_RECEIPT_DIGEST
test "$(git rev-parse "$input_head:$parent_receipt_rel")" = "$(jq -r '.binding.parent_task_receipt.blob_oid' "$receipt")" || refuse PARENT_RECEIPT_BLOB

expected_paths=$(git diff --name-only "$correction_input_head..$implementation_head")
receipt_paths=$(jq -r '.subject.sources[].path' "$receipt")
test "$receipt_paths" = "$expected_paths" || refuse SUBJECT_PATHS

subject_records=""
while IFS= read -r path; do
  test -n "$path" || continue
  expected_sha=$(jq -r --arg path "$path" '.subject.sources[] | select(.path == $path) | .sha256' "$receipt")
  actual_sha=$(git show "$implementation_head:$path" | shasum -a 256 | awk '{print $1}')
  test "$actual_sha" = "$expected_sha" || refuse "SUBJECT_DIGEST:$path"
  subject_records="${subject_records}${path}\t${actual_sha}\n"
done <<EOF
$receipt_paths
EOF
test "$(printf '%b' "$subject_records" | shasum -a 256 | awk '{print $1}')" = "$(jq -r '.subject.sha256' "$receipt")" || refuse SUBJECT_AGGREGATE

tree_tuple=$(git ls-tree "$implementation_head" -- scripts/probe/provider-capabilities.sh)
expected_tree_tuple=$(printf '100755 blob e3089827b02914093474b72607d7b6a41547316a\tscripts/probe/provider-capabilities.sh')
test "$tree_tuple" = "$expected_tree_tuple" || refuse PUBLIC_PROBE_TUPLE
test "$(git ls-tree -r "$implementation_head" -- scripts/probe | wc -l | tr -d ' ')" = 1 || refuse PUBLIC_PROBE_DESCENDANTS
test "$(git show "$implementation_head:scripts/probe/provider-capabilities.sh" | shasum -a 256 | awk '{print $1}')" = bfa281f230109be7927e82e3e263beb7f890e87a1d15e9c1adbdf5e98f7f536b || refuse PUBLIC_PROBE_CONTENT

evidence_index=0
while test "$evidence_index" -lt 5; do
  output=$(jq -r --argjson index "$evidence_index" '.evidence[$index].output' "$receipt")
  expected_sha=$(jq -r --argjson index "$evidence_index" '.evidence[$index].output_sha256' "$receipt")
  test "$(sha_output "$output")" = "$expected_sha" || refuse EVIDENCE_OUTPUT_DIGEST
  evidence_index=$((evidence_index + 1))
done

receipt_blob=$(git hash-object "$receipt")
receipt_commits=""
for commit in $(git rev-list --reverse "$correction_input_head..HEAD"); do
  if test "$(git rev-parse "$commit:$receipt_rel" 2>/dev/null || true)" = "$receipt_blob"; then
    receipt_commits="${receipt_commits}${commit}\n"
  fi
done
receipt_count=$(printf '%b' "$receipt_commits" | sed '/^$/d' | wc -l | tr -d ' ')
test "$receipt_count" = 1 || refuse RECEIPT_COMMIT_COUNT
receipt_commit=$(printf '%b' "$receipt_commits" | sed '/^$/d')
test "$(git rev-parse "$receipt_commit^")" = "$implementation_head" || refuse RECEIPT_PARENT
test "$(git diff-tree --no-commit-id --name-only -r "$receipt_commit")" = "$receipt_rel" || refuse RECEIPT_ONLY

printf '%s\n' P06_ZERO_AUTH_TASK_4_RECEIPT_PASS

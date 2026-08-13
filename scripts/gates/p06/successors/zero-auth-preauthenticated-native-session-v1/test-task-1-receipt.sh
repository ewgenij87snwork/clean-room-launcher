#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json
receipt="$root/$receipt_rel"
input_head=b9a8d2c9dda084d90b984f77faa8a26dd4f03b3a
replaced_receipt_commit=ee7bbef06f97c03eb1f05ee9c9d7cba8658a3e8d

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
  --arg implementation_tree "$implementation_tree" \
  --arg replaced_receipt_commit "$replaced_receipt_commit" '
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.task-receipt.v2" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .task == 1 and
  .result == "accepted" and
  .acceptance.id == "P06-ZERO-AUTH-T1-GOVERNANCE-V2" and
  .acceptance.control_ids == ["ADP-05", "AUTH-01", "OD-10"] and
  .acceptance.evidence_ids == [
    "P06-ZERO-AUTH-T1-RED-MISSING-CONTROL-VALIDATOR-V1",
    "P06-ZERO-AUTH-T1-GREEN-CONTROL-CONTRACT-V1",
    "P06-ZERO-AUTH-T1-FIX1-RED-CONTRADICTORY-PERMISSION-V1",
    "P06-ZERO-AUTH-T1-FIX1-GREEN-SEMANTIC-BRANCHES-V1",
    "P06-ZERO-AUTH-T1-FIX1-GREEN-446-COVERAGE-V1",
    "P06-ZERO-AUTH-T1-FIX1-RED-DESCENDANT-RECEIPT-V1",
    "P06-ZERO-AUTH-T1-FIX1-GREEN-DESCENDANT-RECEIPT-V1"
  ] and
  .binding.scheme == "parent-bound-receipt.v2" and
  .binding.input_head == $input_head and
  .binding.implementation_result_head == $implementation_head and
  .binding.implementation_tree == $implementation_tree and
  .binding.receipt_commit_parent == $implementation_head and
  .binding.replaces_receipt_commit == $replaced_receipt_commit and
  .inputs.plan_checkpoint_sha256 == "b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68" and
  .controls.threshold_reconsideration_only == true and
  .controls.runtime_github_or_stargazer_check == false and
  .controls.automatic_auth_enablement == false and
  .controls.credential_ownership == "none" and
  .controls.provider_birth_on_unavailable_or_ambiguous_session == false and
  .controls.local_functionality_continues == true and
  .controls.canonical_control_count == 446 and
  .controls.previous_445_state_refuses == true and
  .controls.contradictory_permissions_refuse == true and
  .controls.network_access == "not invoked" and
  .controls.provider_or_auth_process == "not invoked" and
  .controls.credential_or_keychain_read == "not invoked" and
  .controls.main_mutation == false and
  [.subject.sources[].path] == [
    "/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/TASKSEAL-MASTER-PROGRAM.md",
    "/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/TASKSEAL-OWNER-DECISIONS.md",
    "/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/TASKSEAL-REQUIREMENTS-TRACEABILITY.md",
    "controls/v0.1-execution-map.tsv",
    "scripts/check-control-coverage.rb",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/control-validator.rb",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-control-validator.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-1-receipt-durability.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-1-receipt.sh",
    "src/contracts/trace.rs",
    "tests/contracts/trace_metadata.rs"
  ] and
  ([.subject.sources[].path] | unique | length) == 11 and
  ([.subject.sources[] | select(.kind == "external_owner_ssot_current")] | length) == 3 and
  ([.subject.sources[] | select(.kind == "repository_implementation_commit")] | length) == 8 and
  .evidence == [
    {
      "id":"P06-ZERO-AUTH-T1-RED-MISSING-CONTROL-VALIDATOR-V1",
      "command":"sh scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-control-validator.sh",
      "exit":1,
      "output":"P06_ZERO_AUTH_RED_MISSING_CONTROL_VALIDATOR",
      "output_sha256":"bf10638ed78669e4b80e3eb2e255a7d4e79d1517d5a6fa35cde79b5c5f497b0d",
      "meaning":"The focused contract test failed before the validator and governed semantics existed."
    },
    {
      "id":"P06-ZERO-AUTH-T1-GREEN-CONTROL-CONTRACT-V1",
      "command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-control-validator.sh",
      "exit":0,
      "output":"P06_ZERO_AUTH_CONTROL_TEST_PASS",
      "output_sha256":"28fe4d0eb9e577e479f071c5edfa5060197bfe346d9798825ab6e646475908b4",
      "meaning":"The original focused suite accepted the governed SSOT/map and its first refusal matrix."
    },
    {
      "id":"P06-ZERO-AUTH-T1-FIX1-RED-CONTRADICTORY-PERMISSION-V1",
      "command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-control-validator.sh",
      "exit":1,
      "output":"P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:OD10_CONTRADICTORY_PERMISSION",
      "output_sha256":"6d472ef409e8189e022c080d328ca396e4b6292e6a063778eebbef7c5b1d6057",
      "meaning":"The literal-only validator accepted an appended affirmative provider-login permission while required prohibition text remained."
    },
    {
      "id":"P06-ZERO-AUTH-T1-FIX1-GREEN-SEMANTIC-BRANCHES-V1",
      "command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-control-validator.sh",
      "exit":0,
      "output":"P06_ZERO_AUTH_CONTROL_TEST_PASS",
      "output_sha256":"28fe4d0eb9e577e479f071c5edfa5060197bfe346d9798825ab6e646475908b4",
      "meaning":"The semantic suite refused contradictory permission plus every critical zero-auth branch mutation."
    },
    {
      "id":"P06-ZERO-AUTH-T1-FIX1-GREEN-446-COVERAGE-V1",
      "command":"ruby scripts/check-control-coverage.rb",
      "exit":0,
      "output":"446/446 explicit controls pass",
      "output_sha256":"ba8021f85dc71498fdb979e50bce268c50d39965284768e8c8a622fb2bf8766d",
      "meaning":"The live canonical coverage checker accepted all 446 unique explicit control rows."
    },
    {
      "id":"P06-ZERO-AUTH-T1-FIX1-RED-DESCENDANT-RECEIPT-V1",
      "command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-1-receipt-durability.sh",
      "exit":1,
      "output":"P06_ZERO_AUTH_RECEIPT_DESCENDANT_RED:1:NO_OUTPUT",
      "output_sha256":"fda533ecbf4ef4d92cf35e261bb66c6aa203774ff4f7efb59f097204c59c185e",
      "meaning":"The original checker false-refused when a real local clone added a descendant commit after the Task 1 receipt."
    },
    {
      "id":"P06-ZERO-AUTH-T1-FIX1-GREEN-DESCENDANT-RECEIPT-V1",
      "command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-1-receipt-durability.sh",
      "exit":0,
      "output":"P06_ZERO_AUTH_TASK_1_RECEIPT_DURABILITY_PASS",
      "output_sha256":"65257ef5671dd25f1fc3aedc7ed3b7e647036506b13212500b764e80afcb712b",
      "meaning":"A disposable local clone added a descendant commit and still resolved and verified the unique replacement receipt commit."
    }
  ] and
  .seal_tdd == {
    "red_command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-1-receipt-durability.sh",
    "red_exit":1,
    "red_output":"P06_ZERO_AUTH_RECEIPT_DESCENDANT_RED:1:NO_OUTPUT",
    "red_output_sha256":"fda533ecbf4ef4d92cf35e261bb66c6aa203774ff4f7efb59f097204c59c185e",
    "green_command":"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-1-receipt.sh",
    "green_exit":0,
    "green_output":"P06_ZERO_AUTH_TASK_1_RECEIPT_PASS",
    "green_output_sha256":"415745ed8be46c34368256cf3c26cdf5b32325552db353bec86f16ffb42d8715"
  }
  ' "$receipt" >/dev/null || refuse RECEIPT_CONTRACT

test "$(git rev-parse "$implementation_head^{tree}")" = "$implementation_tree" || refuse IMPLEMENTATION_TREE_MISMATCH
git merge-base --is-ancestor "$replaced_receipt_commit" "$implementation_head" || refuse REPLACEMENT_LINEAGE
test "$(git diff-tree --no-commit-id --name-only -r "$replaced_receipt_commit")" = "$receipt_rel" || refuse REPLACED_RECEIPT_NOT_RECEIPT_ONLY

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-receipt-check.XXXXXX")
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
    test "$(git diff --name-only HEAD --)" = "$receipt_rel" || refuse UNCOMMITTED_RECEIPT_WRITE_SET
    ;;
  1)
    receipt_commit=$(sed -n '1p' "$scratch/matching-commits")
    set -- $(git rev-list --parents -n 1 "$receipt_commit")
    test "$#" = 2 || refuse RECEIPT_PARENT_COUNT
    test "$2" = "$implementation_head" || refuse RECEIPT_PARENT
    test "$(git diff-tree --no-commit-id --name-only -r "$receipt_commit")" = "$receipt_rel" || refuse RECEIPT_NOT_RECEIPT_ONLY
    test "$(git rev-parse "$receipt_commit:$receipt_rel")" = "$receipt_blob" || refuse RECEIPT_BLOB
    ;;
  *) refuse RECEIPT_COMMIT_NOT_UNIQUE ;;
esac

: >"$scratch/subject-records"
while IFS=$(printf '\t') read -r kind path expected_sha; do
  case "$kind" in
    external_owner_ssot_current)
      test -f "$path" || refuse EXTERNAL_SUBJECT_MISSING
      actual_sha=$(sha_file "$path")
      ;;
    repository_implementation_commit)
      git cat-file -e "$implementation_head:$path" || refuse REPOSITORY_SUBJECT_MISSING
      actual_sha=$(git show "$implementation_head:$path" | shasum -a 256 | awk '{print $1}')
      ;;
    *) refuse SUBJECT_KIND ;;
  esac
  test "$actual_sha" = "$expected_sha" || refuse SUBJECT_SOURCE_DIGEST
  printf '%s\t%s\n' "$path" "$actual_sha" >>"$scratch/subject-records"
done <<EOF
$(jq -r '.subject.sources[] | [.kind, .path, .sha256] | @tsv' "$receipt")
EOF

sort "$scratch/subject-records" >"$scratch/subject-records.sorted"
cmp -s "$scratch/subject-records" "$scratch/subject-records.sorted" || refuse SUBJECT_ORDER
test "$(sha_file "$scratch/subject-records")" = "$(jq -r '.subject.sha256' "$receipt")" || refuse SUBJECT_AGGREGATE

while IFS=$(printf '\t') read -r output expected_sha; do
  test "$(sha_output "$output")" = "$expected_sha" || refuse EVIDENCE_OUTPUT_DIGEST
done <<EOF
$(jq -r '.evidence[] | [.output, .output_sha256] | @tsv' "$receipt")
EOF

test "$(sha_output "$(jq -r '.seal_tdd.red_output' "$receipt")")" = "$(jq -r '.seal_tdd.red_output_sha256' "$receipt")" || refuse SEAL_RED_OUTPUT_DIGEST
test "$(sha_output "$(jq -r '.seal_tdd.green_output' "$receipt")")" = "$(jq -r '.seal_tdd.green_output_sha256' "$receipt")" || refuse SEAL_GREEN_OUTPUT_DIGEST

printf '%s\n' P06_ZERO_AUTH_TASK_1_RECEIPT_PASS

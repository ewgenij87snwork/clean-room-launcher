#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json
receipt="$root/$receipt_rel"
input_head=6a54523b4dbf8869ca7035a7dc42df88ac11c45a
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
  --arg implementation_head "$implementation_head" \
  --arg implementation_tree "$implementation_tree" '
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.task-receipt.v1" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .task == 4 and .result == "accepted" and
  .acceptance.id == "P06-ZERO-AUTH-T4-LOCAL-UX-CONSOLIDATED-CLOSURE-V1" and
  .acceptance.control_ids == ["ADP-05","AUTH-01","OD-10"] and
  .acceptance.evidence_ids == [
    "P06-ZERO-AUTH-T4-RED-TRANSCRIPT-API-V1",
    "P06-ZERO-AUTH-T4-GREEN-EXACT-TRANSCRIPTS-V1",
    "P06-ZERO-AUTH-T4-RED-MISSING-CONSOLIDATED-VERIFIER-V1",
    "P06-ZERO-AUTH-T4-GREEN-GATE-MUTATIONS-V1",
    "P06-ZERO-AUTH-T4-FIX1-RED-UNOWNED-FMT-SCOPE-V1",
    "P06-ZERO-AUTH-T4-FIX1-GREEN-OWNED-FMT-SCOPE-V1",
    "P06-ZERO-AUTH-T4-FIX2-RED-UNGOVERNED-PUBLIC-INVENTORY-V1",
    "P06-ZERO-AUTH-T4-FIX2-GREEN-GOVERNED-PUBLIC-INVENTORY-V1",
    "P06-ZERO-AUTH-T4-FIX3-RED-SOURCE-SCANNER-SELF-MATCH-V1",
    "P06-ZERO-AUTH-T4-FIX3-GREEN-CURRENT-SOURCE-INVENTORY-V1",
    "P06-ZERO-AUTH-T4-GREEN-SOLE-GATE-V1",
    "P06-ZERO-AUTH-T4-GREEN-DESCENDANT-DURABILITY-V1"
  ] and
  .binding == {
    scheme:"parent-bound-receipt.v2",
    input_head:$input_head,
    implementation_result_head:$implementation_head,
    implementation_tree:$implementation_tree,
    receipt_commit_parent:$implementation_head,
    replaces_receipt_commit:"95529ea783007d374bac6bddd990fe7f013b3e84",
    parent_task_receipt:{
      path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-3.json",
      commit:$input_head,
      blob_oid:"08febe5afe412e00e46f32d4c915f94e6cbe209d",
      sha256:"8916cd8d268d91988931985ffb952b95fde491445f84ddbfa9a1c22352a68de8"
    },
    resolution:"Resolve exactly one commit in input_head..tip whose task-4 receipt blob equals these bytes; it must be a single-parent receipt-only child of implementation_result_head and replace receipt-only 95529ea. Repository subjects are read from that implementation commit, while the accepted Task 3 receipt is read from input_head."
  } and
  .inputs == {
    plan_checkpoint_path:"/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-13-p06-zero-auth-preauthenticated-native-session-v1.md",
    plan_checkpoint_sha256:"b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68",
    accepted_task_3_head:$input_head
  } and
  .subject.algorithm == "sha256 of sorted path, tab, sha256, newline repository records" and
  (.subject.sha256 | test("^[0-9a-f]{64}$")) and
  [.subject.sources[].path] == [
    "fixtures/cli/first-screen-unqualified-narrow.txt",
    "fixtures/cli/first-screen-unqualified-non-tty.txt",
    "fixtures/cli/first-screen-unqualified-plain.txt",
    "fixtures/cli/first-screen-unqualified-tty.txt",
    "fixtures/cli/first-screen-unqualified.txt",
    "reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/consolidated.json",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt-durability.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-verify.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "src/cli/screen.rs",
    "tests/cli/first_screen.rs"
  ] and
  ([.subject.sources[].path] | unique | length) == 12 and
  ([.subject.sources[] | select(.kind == "repository_implementation_commit" and (.sha256 | test("^[0-9a-f]{64}$")))] | length) == 12 and
  [.evidence[].id] == .acceptance.evidence_ids and
  [.evidence[].command] == [
    "CARGO_INCREMENTAL=0 cargo test --offline --test cli first_screen:: -- --test-threads=1",
    "CARGO_INCREMENTAL=0 cargo test --offline --test cli first_screen:: -- --test-threads=1",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-verify.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-verify.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "rustfmt --edition 2024 --check src/cli/screen.rs tests/cli/first_screen.rs",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "git archive HEAD governed release paths | scripts/check-public-boundary.sh --root <archive>",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "ruby scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory.rb .",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt-durability.sh"
  ] and
  [.evidence[].exit] == [101,0,1,0,1,0,1,0,1,0,0,0] and
  [.evidence[].output] == [
    "error[E0425]: RenderContext and render_unqualified_for are absent",
    "test result: ok. 4 passed; 0 failed; 0 ignored",
    "P06_ZERO_AUTH_RED_VERIFIER_MISSING",
    "P06_ZERO_AUTH_GATE_MUTATIONS_PASS",
    "cargo fmt --check found pre-existing formatting drift outside Task 4 write-set",
    "rustfmt --edition 2024 --check src/cli/screen.rs tests/cli/first_screen.rs: exit 0",
    "full Git archive public scan matched pre-existing evidence path identifiers outside the governed release inventory",
    "PUBLIC_BOUNDARY_PASS",
    "P06_ZERO_AUTH_SOURCE_REFUSAL:CREDENTIAL_EXTRACTION:scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "P06_ZERO_AUTH_SOURCE_INVENTORY_PASS",
    "P06_ZERO_AUTH_PREAUTHENTICATED_NATIVE_SESSION_V1_PASS",
    "P06_ZERO_AUTH_TASK_4_RECEIPT_DURABILITY_PASS"
  ] and
  .seal_tdd == {
    red_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    red_exit:1,
    red_output:"P06_ZERO_AUTH_TASK_4_RECEIPT_REFUSAL:MISSING_RECEIPT",
    red_output_sha256:"9dbea76ca126f71bb1a809186e5b78f0cd2b6096d0276a2bd60e362c2d165302",
    green_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    green_exit:0,
    green_output:"P06_ZERO_AUTH_TASK_4_RECEIPT_PASS",
    green_output_sha256:"5817c91c2d32c47a64aa80641772d471b3b3f2261334e30262eb5c33fff4c30d"
  } and
  .controls == {
    unqualified_default_action:"CONTINUE_LOCALLY",
    exact_transcripts:["TTY_80","TTY_40_NARROW","PLAIN_TTY_80","NON_TTY_80"],
    prohibited_cta_classes:["LOGIN","SETTINGS","API_KEY","DOCUMENTATION"],
    prohibited_cta_present:false,
    non_tty_prompt:false,
    task_receipts_validated:[1,2,3,4],
    governed_controls:["ADP-05","AUTH-01","OD-10"],
    source_inventory:"CURRENT_TRACKED_INVENTORY",
    public_boundary_inventory:"CURRENT_GIT_GOVERNED_RELEASE_INVENTORY",
    full_zero_auth_call_paths:true,
    git_history_immutable:true,
    implementation_write_set_closed:true,
    receipt_topology:"PARENT_BOUND_RECEIPT_ONLY",
    owner_ssot_binding:"SEALED_WORKLOG_PREFIX",
    mutable_worklog_tail_hashed:false,
    terminal_review_state:"PENDING",
    network_access:"not invoked",
    provider_or_auth_process:"not invoked",
    credential_or_keychain_read:"not invoked",
    main_mutation:false
  }
  ' "$receipt" >/dev/null || refuse RECEIPT_CONTRACT

test "$(git rev-parse "$implementation_head^{tree}")" = "$implementation_tree" || refuse IMPLEMENTATION_TREE_MISMATCH
git merge-base --is-ancestor "$input_head" "$implementation_head" || refuse IMPLEMENTATION_LINEAGE
test "$(git rev-parse "$input_head:$parent_receipt_rel")" = "$(jq -r '.binding.parent_task_receipt.blob_oid' "$receipt")" || refuse PARENT_RECEIPT_BLOB
test "$(git show "$input_head:$parent_receipt_rel" | shasum -a 256 | awk '{print $1}')" = "$(jq -r '.binding.parent_task_receipt.sha256' "$receipt")" || refuse PARENT_RECEIPT_DIGEST
git merge-base --is-ancestor 95529ea783007d374bac6bddd990fe7f013b3e84 "$implementation_head" || refuse REPLACEMENT_LINEAGE
test "$(git diff-tree --no-commit-id --name-only -r 95529ea783007d374bac6bddd990fe7f013b3e84)" = "$receipt_rel" || refuse REPLACED_RECEIPT_NOT_RECEIPT_ONLY

expected_paths='fixtures/cli/first-screen-unqualified-narrow.txt
fixtures/cli/first-screen-unqualified-non-tty.txt
fixtures/cli/first-screen-unqualified-plain.txt
fixtures/cli/first-screen-unqualified-tty.txt
fixtures/cli/first-screen-unqualified.txt
reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/consolidated.json
reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt-durability.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-verify.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh
src/cli/screen.rs
tests/cli/first_screen.rs'
test "$(git diff --name-only "$input_head..$implementation_head")" = "$expected_paths" || refuse IMPLEMENTATION_WRITE_SET
test -z "$(git rev-list --min-parents=2 "$input_head..$implementation_head")" || refuse MERGE_TOPOLOGY

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-task4-receipt.XXXXXX")
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
    test "$(git status --porcelain=v1 --untracked-files=all)" = " M $receipt_rel" || refuse UNCOMMITTED_RECEIPT_WRITE_SET
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
  actual_sha=$(git show "$implementation_head:$path" | shasum -a 256 | awk '{print $1}')
  test "$actual_sha" = "$expected_sha" || refuse SUBJECT_SOURCE_DIGEST
  printf '%s\t%s\n' "$path" "$actual_sha" >>"$scratch/subject-records"
done <<EOF
$(jq -r '.subject.sources[] | [.kind, .path, .sha256] | @tsv' "$receipt")
EOF
LC_ALL=C sort "$scratch/subject-records" >"$scratch/subject-records.sorted"
cmp -s "$scratch/subject-records" "$scratch/subject-records.sorted" || refuse SUBJECT_ORDER
test "$(shasum -a 256 "$scratch/subject-records" | awk '{print $1}')" = "$(jq -r '.subject.sha256' "$receipt")" || refuse SUBJECT_AGGREGATE

jq -c '.evidence[]' "$receipt" | while IFS= read -r evidence; do
  output=$(printf '%s\n' "$evidence" | jq -r .output)
  test "$(sha_output "$output")" = "$(printf '%s\n' "$evidence" | jq -r .output_sha256)" || refuse EVIDENCE_OUTPUT_DIGEST
done
test "$(sha_output "$(jq -r '.seal_tdd.red_output' "$receipt")")" = "$(jq -r '.seal_tdd.red_output_sha256' "$receipt")" || refuse SEAL_RED_OUTPUT_DIGEST
test "$(sha_output "$(jq -r '.seal_tdd.green_output' "$receipt")")" = "$(jq -r '.seal_tdd.green_output_sha256' "$receipt")" || refuse SEAL_GREEN_OUTPUT_DIGEST

printf '%s\n' P06_ZERO_AUTH_TASK_4_RECEIPT_PASS

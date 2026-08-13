#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-3.json
receipt="$root/$receipt_rel"
input_head=3f2bda4dadd22395a9313e80b03b029b43e4a5f3
parent_receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-2.json

refuse() {
  printf '%s\n' "P06_ZERO_AUTH_TASK_3_RECEIPT_REFUSAL:$1" >&2
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
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.task-receipt.v2" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .task == 3 and .result == "accepted" and
  .acceptance == {
    id:"P06-ZERO-AUTH-T3-LAUNCHER-ENVIRONMENT-CLOSURE-V1",
    operator_result:"Named routes refuse after consuming only the command token and generic routes after only the boundary plus executable, without reading or copying credential-shaped tails; every real external route still refuses before child birth; local commands remain available; the environment carries only an opaque provider-native preauthenticated-session availability state; and descendant execution is sealed.",
    control_ids:["AUTH-01"],
    evidence_ids:[
      "P06-ZERO-AUTH-T3-RED-STABLE-REFUSAL-V1",
      "P06-ZERO-AUTH-T3-RED-OPAQUE-SESSION-API-V1",
      "P06-ZERO-AUTH-T3-GREEN-CLI-CALL-PATHS-V1",
      "P06-ZERO-AUTH-T3-GREEN-OPAQUE-SESSION-V1",
      "P06-ZERO-AUTH-T3-FIX1-RED-TAIL-CONSUMPTION-V1",
      "P06-ZERO-AUTH-T3-FIX1-GREEN-UNCONSUMED-TAIL-V1",
      "P06-ZERO-AUTH-T3-FIX1-GREEN-VALUE-BEARING-CALL-PATHS-V1",
      "P06-ZERO-AUTH-T3-FIX1-GREEN-OFFLINE-RELEVANT-SUITES-V1",
      "P06-ZERO-AUTH-T3-GREEN-SOURCE-INVENTORY-V1",
      "P06-ZERO-AUTH-T3-GREEN-CLIPPY-V1",
      "P06-ZERO-AUTH-T3-FIX1-RED-REPLACEMENT-RECEIPT-V1",
      "P06-ZERO-AUTH-T3-FIX1-GREEN-DESCENDANT-DURABILITY-V1"
    ]
  } and
  .binding == {
    scheme:"parent-bound-receipt.v2",
    input_head:$input_head,
    implementation_result_head:$implementation_head,
    implementation_tree:$implementation_tree,
    receipt_commit_parent:$implementation_head,
    replaces_receipt_commit:"487c762fb79429337b6067a80a9ab7c3de3a3178",
    parent_task_receipt:{
      path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-2.json",
      commit:$input_head,
      blob_oid:"7a06a1a17ff68b931385326896fde7ba6adc9002",
      sha256:"81673336fa08bb90a48229779556bb4a1c9ee21d2b8c0b7bd44eef3db583105a"
    },
    resolution:"Resolve exactly one commit in input_head..tip whose task-3 receipt blob equals these bytes; it must be a single-parent receipt-only child of implementation_result_head. Repository subjects are read from that implementation commit, while the accepted Task 2 receipt is read from input_head."
  } and
  .inputs == {
    plan_checkpoint_path:"/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-13-p06-zero-auth-preauthenticated-native-session-v1.md",
    plan_checkpoint_sha256:"b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68",
    accepted_task_2_head:$input_head
  } and
  .subject.algorithm == "sha256 of sorted path, tab, sha256, newline repository records" and
  (.subject.sha256 | test("^[0-9a-f]{64}$")) and
  [.subject.sources[].path] == [
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt-durability.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt.sh",
    "src/adapters/environment.rs",
    "src/cli/dispatch.rs",
    "src/cli/help.rs",
    "src/cli/mod.rs",
    "src/cli/parser.rs",
    "src/cli/process.rs",
    "tests/adapters/environment.rs",
    "tests/cli.rs",
    "tests/cli/argv_passthrough.rs",
    "tests/cli/foreground_process.rs"
  ] and
  ([.subject.sources[].path] | unique | length) == 12 and
  ([.subject.sources[] | select(.kind == "repository_implementation_commit" and (.sha256 | test("^[0-9a-f]{64}$")))] | length) == 12 and
  .evidence == [
    {
      id:"P06-ZERO-AUTH-T3-RED-STABLE-REFUSAL-V1",
      command:"cargo test --test cli argv_passthrough::named_and_generic_auth_routes_share_one_pre_birth_zero_auth_refusal -- --exact",
      exit:101,
      output:"left: P06_REQUIRED: provider tuple is not qualified; right: ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally",
      output_sha256:"d0311996f44e8d760c7bce53e20d1f7454104321b43a1007ce543bf688fb6c26",
      meaning:"The real named CLI path refused before child birth but lacked the stable zero-auth contract shared by every prohibited external route."
    },
    {
      id:"P06-ZERO-AUTH-T3-RED-OPAQUE-SESSION-API-V1",
      command:"cargo test --test adapters environment::unavailable_and_ambiguous_session_states_refuse_without_fallback -- --exact",
      exit:101,
      output:"error[E0432]: ProviderNativePreauthenticatedSession API is absent",
      output_sha256:"7be5d005764ae638406e0c7b66f1957aa482660509918449204ad851b6032e8c",
      meaning:"The typed adapter test could not compile before the data-free provider-native session state replaced the raw value map."
    },
    {
      id:"P06-ZERO-AUTH-T3-GREEN-CLI-CALL-PATHS-V1",
      command:"cargo test --test cli argv_passthrough::",
      exit:0,
      output:"test result: ok. 4 passed; 0 failed; 0 ignored",
      output_sha256:"12b870455880fdf988e0e26f4add7aa2681d66166767172857044de0689dcc8f",
      meaning:"Named and generic zero-auth negatives and local-only positives passed; fake external capture files remained absent."
    },
    {
      id:"P06-ZERO-AUTH-T3-GREEN-OPAQUE-SESSION-V1",
      command:"cargo test --test adapters environment::",
      exit:0,
      output:"test result: ok. 5 passed; 0 failed; 0 ignored",
      output_sha256:"5488d903bcf9e747af3425d8a71354c9d318f4a9f43af02e88e873f792e44b06",
      meaning:"The data-free available state passed while unavailable, ambiguous and wrong-identity inputs refused without a value or fallback path."
    },
    {
      id:"P06-ZERO-AUTH-T3-FIX1-RED-TAIL-CONSUMPTION-V1",
      command:"cargo test --offline --test cli argv_passthrough::provider_and_generic_refusal_do_not_consume_credential_shaped_tails -- --exact",
      exit:101,
      output:"TaskSeal consumed a prohibited provider tail containing 2 unread values",
      output_sha256:"c80465777a3dcd248678302f7ea1774fb5e06c1c6a61d099799e8905265ed82f",
      meaning:"The poison iterator proved cli::run eagerly enumerated a value-bearing provider tail before zero-auth refusal."
    },
    {
      id:"P06-ZERO-AUTH-T3-FIX1-GREEN-UNCONSUMED-TAIL-V1",
      command:"cargo test --offline --test cli argv_passthrough::provider_and_generic_refusal_do_not_consume_credential_shaped_tails -- --exact",
      exit:0,
      output:"test result: ok. 1 passed; 0 failed; 0 ignored",
      output_sha256:"e9452ac4fb913b961bfcddcb2990f876000c3af2780281421ed04e69499594c0",
      meaning:"The counting iterator observed exactly one read for a named route and two for a generic route, leaving both credential-shaped tails unread."
    },
    {
      id:"P06-ZERO-AUTH-T3-FIX1-GREEN-VALUE-BEARING-CALL-PATHS-V1",
      command:"cargo test --offline --test cli argv_passthrough::named_and_generic_auth_routes_share_one_pre_birth_zero_auth_refusal -- --exact",
      exit:0,
      output:"test result: ok. 1 passed; 0 failed; 0 ignored",
      output_sha256:"e9452ac4fb913b961bfcddcb2990f876000c3af2780281421ed04e69499594c0",
      meaning:"Real named and generic binaries received value-bearing prohibited tails yet produced the stable refusal without creating the fake-provider capture."
    },
    {
      id:"P06-ZERO-AUTH-T3-FIX1-GREEN-OFFLINE-RELEVANT-SUITES-V1",
      command:"cargo test --test cli --test adapters --offline",
      exit:0,
      output:"adapters: 31 passed; cli: 31 passed; 0 failed",
      output_sha256:"2adde19e405983dcab10bbca487d9159dc3d75b8adc1b7436c0bff27dc804587",
      meaning:"The exact offline CLI and adapter command passed after lazy prefix refusal and the opaque environment contract."
    },
    {
      id:"P06-ZERO-AUTH-T3-GREEN-SOURCE-INVENTORY-V1",
      command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-source-inventory.sh",
      exit:0,
      output:"P06_ZERO_AUTH_SOURCE_INVENTORY_TEST_PASS",
      output_sha256:"ff2c843a18a4b76f59ff24589d0335a7057640cb926ea647f7246d40ae7b64af",
      meaning:"The current governed executable and compiled-source inventory remained free of credential, provider-login and browser-flow execution paths."
    },
    {
      id:"P06-ZERO-AUTH-T3-GREEN-CLIPPY-V1",
      command:"cargo clippy --all-targets --offline -- -D warnings",
      exit:0,
      output:"cargo clippy --all-targets --offline -- -D warnings: exit 0",
      output_sha256:"98f57ed33b0abfb146322bda89690a037b88bcbbbe93ad84019dc7199ae0eac4",
      meaning:"Every Rust target compiled under strict warning denial without network access."
    },
    {
      id:"P06-ZERO-AUTH-T3-FIX1-RED-REPLACEMENT-RECEIPT-V1",
      command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt.sh",
      exit:1,
      output:"P06_ZERO_AUTH_TASK_3_RECEIPT_REFUSAL:RECEIPT_CONTRACT",
      output_sha256:"99105b71affcc63e638e4be1bb9d98bd83aff56f72ed8e6d9e50d1e9e4874fb0",
      meaning:"The hardened checker rejected the predecessor receipt because it lacked lazy-tail, exact offline-suite and descendant-durability bindings."
    },
    {
      id:"P06-ZERO-AUTH-T3-FIX1-GREEN-DESCENDANT-DURABILITY-V1",
      command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt-durability.sh",
      exit:0,
      output:"P06_ZERO_AUTH_TASK_3_RECEIPT_DURABILITY_PASS",
      output_sha256:"6611239986d8ef1e5d89d4d3ce74caef1948305d36b6d87e6045f1f5f9f39bc9",
      meaning:"A no-network local clone added a real descendant commit and reproduced the parent-bound Task 3 receipt."
    }
  ] and
  .seal_tdd == {
    red_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt.sh",
    red_exit:1,
    red_output:"P06_ZERO_AUTH_TASK_3_RECEIPT_REFUSAL:MISSING_RECEIPT",
    red_output_sha256:"5657c4852348d0aef68f019f6d7c5375bf84f0dd465adaf365dd57af1e1d4f62",
    green_command:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt.sh",
    green_exit:0,
    green_output:"P06_ZERO_AUTH_TASK_3_RECEIPT_PASS",
    green_output_sha256:"dde3b4e2e699bf0abbcd33dfda90e45060637af03d7671648f8967d08f1703a0"
  } and
  .controls == {
    arbitrary_external_execution:"REFUSED",
    external_allowlist:"TASKSEAL_OWNED_LOCAL_COMMANDS_ONLY",
    external_routes:["NAMED","GENERIC","DEVICE_HELPER","BROWSER_HELPER","RENAMED","SYMLINKED"],
    stable_refusal:"ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally",
    local_commands_available:["status","scan","prepare","check"],
    environment_contract:"PROVIDER_NATIVE_PREAUTHENTICATED_SESSION",
    provider_tail_values_consumed:0,
    provider_tail_values_copied:0,
    unavailable_state:"REFUSED_PRE_BIRTH",
    ambiguous_state:"REFUSED_PRE_BIRTH",
    auth_or_billing_fallback:false,
    external_child_birth:false,
    browser_or_prompt_birth:false,
    network_access:"not invoked",
    main_mutation:false,
    verification_environment_exceptions:[
      {
        test:"clean_public_inventory_passes",
        reason:"The working-tree public-boundary test sees the active ignored SDD ledger path .superpowers/sdd/.gitignore; that execution-only path is absent from Git subjects and release inventory."
      }
    ]
  }
  ' "$receipt" >/dev/null || refuse RECEIPT_CONTRACT

test "$(git rev-parse "$implementation_head^{tree}")" = "$implementation_tree" || refuse IMPLEMENTATION_TREE_MISMATCH
git merge-base --is-ancestor "$input_head" "$implementation_head" || refuse IMPLEMENTATION_LINEAGE
test "$(git rev-parse "$input_head:$parent_receipt_rel")" = "$(jq -r '.binding.parent_task_receipt.blob_oid' "$receipt")" || refuse PARENT_RECEIPT_BLOB
test "$(git show "$input_head:$parent_receipt_rel" | shasum -a 256 | awk '{print $1}')" = "$(jq -r '.binding.parent_task_receipt.sha256' "$receipt")" || refuse PARENT_RECEIPT_DIGEST
git merge-base --is-ancestor 487c762fb79429337b6067a80a9ab7c3de3a3178 "$implementation_head" || refuse REPLACEMENT_LINEAGE
test "$(git diff-tree --no-commit-id --name-only -r 487c762fb79429337b6067a80a9ab7c3de3a3178)" = "$receipt_rel" || refuse REPLACED_RECEIPT_NOT_RECEIPT_ONLY

expected_paths='reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-3.json
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt-durability.sh
scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt.sh
src/adapters/environment.rs
src/cli/dispatch.rs
src/cli/help.rs
src/cli/mod.rs
src/cli/parser.rs
src/cli/process.rs
tests/adapters/environment.rs
tests/cli.rs
tests/cli/argv_passthrough.rs
tests/cli/foreground_process.rs'
test "$(git diff --name-only "$input_head..$implementation_head")" = "$expected_paths" || refuse IMPLEMENTATION_WRITE_SET

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-task3-receipt.XXXXXX")
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

printf '%s\n' P06_ZERO_AUTH_TASK_3_RECEIPT_PASS

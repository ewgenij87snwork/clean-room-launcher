#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
successor=scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1
report=${P06_ZERO_AUTH_REPORT:-$root/reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/consolidated.json}
worklog=${P06_ZERO_AUTH_WORKLOG:-/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/TASKSEAL-WORKLOG.jsonl}
checkpoint=/Users/ysorokin/Documents/it/5-LVL\ -\ 2026/Temp\ in\ Projects/wisdom/taskseal/plans/2026-08-13-p06-zero-auth-preauthenticated-native-session-v1.md
input_head=6a54523b4dbf8869ca7035a7dc42df88ac11c45a

refuse() {
  printf 'P06_ZERO_AUTH_GATE_REFUSAL:%s\n' "$1" >&2
  exit 1
}

sha_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

test -f "$report" || refuse CONSOLIDATED_REPORT_MISSING
test -f "$worklog" || refuse WORKLOG_MISSING
cd "$root"

jq -e '
  keys == ["checkpoint","controls","dispatch_boundary","git","ingestion_closure","local_continuity","owner_ssot","plan_id","provider_birth_boundary","result","schema_version","task_receipts","terminal_review","transcripts","verification"] and
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.consolidated.v1" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .result == "IMPLEMENTED_REVIEW_PENDING" and
  .checkpoint == {
    path:"/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-13-p06-zero-auth-preauthenticated-native-session-v1.md",
    sha256:"b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68"
  } and
  .controls == ["ADP-05","AUTH-01","OD-10"] and
  .task_receipts == [
    {task:1,path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json",commit:"2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6",sha256:"a20e8c793f99a6022a96d9e274c8c6685011e8762d90bee3afe1320f445df472"},
    {task:2,path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-2.json",commit:"3f2bda4dadd22395a9313e80b03b029b43e4a5f3",sha256:"81673336fa08bb90a48229779556bb4a1c9ee21d2b8c0b7bd44eef3db583105a"},
    {task:3,path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-3.json",commit:"6a54523b4dbf8869ca7035a7dc42df88ac11c45a",sha256:"8916cd8d268d91988931985ffb952b95fde491445f84ddbfa9a1c22352a68de8"},
    {task:4,path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json",binding:"PARENT_BOUND_REPLACEMENT_RECEIPT_V2_RESOLVED_BY_GATE"}
  ] and
  .transcripts == [
    {mode:"TTY_80",path:"fixtures/cli/first-screen-unqualified-tty.txt",sha256:"5c8bbdea3c033746f4eb9d34327299bd7e24b16ef03226cdcbf3505d76404fd8"},
    {mode:"TTY_40_NARROW",path:"fixtures/cli/first-screen-unqualified-narrow.txt",sha256:"89289d95a67501e446c4eae4debe739b78652ea7e246835c97fc814b1481a2b2"},
    {mode:"PLAIN_TTY_80",path:"fixtures/cli/first-screen-unqualified-plain.txt",sha256:"d57208b7fb3f5a97b8a8c64fa7d874a27aee5e5832d93d7fd146d5a34bfb4371"},
    {mode:"NON_TTY_80",path:"fixtures/cli/first-screen-unqualified-non-tty.txt",sha256:"f54f17b1d9d2576f2fb9f31ca2799dd94b5b69824dfdb610918f9a2e46f9e20b"}
  ] and
  .local_continuity == {
    default_action:"Continue locally",
    local_commands:["status","scan","prepare","check"],
    prohibited_cta_classes:["LOGIN","SETUP_SETTINGS_CONFIGURATION","API_KEY","DOCUMENTATION"],
    prohibited_cta_present:false,
    non_tty_prompt:false,
    real_tty_mode_selected:true,
    real_tty_enter_dispatch:"TASKSEAL_OWNED_STATUS_IN_PROCESS",
    semantic_cta_regression:"tests/cli/first_screen.rs::assert_zero_auth_actions"
  } and
  .ingestion_closure == {
    cli_unread_tail_routes:["GENERIC_EXECUTABLE_POSITION","SELECTOR_PREFIXED_GENERIC_EXECUTABLE_POSITION","UNSUPPORTED_SELECTOR","SELECTOR_PREFIXED_LOCAL","TASKSEAL_OWNED_LOCAL","UNKNOWN_COMMAND"],
    credential_tail_values_consumed:0,
    credential_tail_values_copied:0,
    generic_post_boundary_values_consumed:0,
    generic_post_boundary_values_copied:0,
    saved_start_sensitive_selectors:["--with-access-token","--access-token","--with-access-token=<value>","--access-token=<value>"],
    saved_start_refusal_phase:"BEFORE_DESERIALIZATION_OR_ARGV_HASH",
    saved_start_save_refusal_phase:"BEFORE_LOCK_SERIALIZATION_OR_WRITE",
    valid_local_commands_preserved:["status","scan","prepare","check","starts","start","help","doctor"],
    valid_saved_starts_preserved:true
  } and
  .dispatch_boundary == {
    source:"src/cli/zero_auth.rs",
    phase:"BEFORE_LOCAL_UNKNOWN_OR_SELECTOR_DISPATCH",
    argument_routes:["HELP_ALIASES","EXPLAIN_INSPECT","DOCTOR_START","OUTPUT_FORMAT","OUTPUT_COMMAND","UNKNOWN_COMMAND"],
    credential_shaped_values_consumed:0,
    credential_shaped_values_copied:0,
    credential_shaped_values_echoed:0,
    non_sensitive_local_arguments_preserved:true,
    entrypoints:["src/bin/taskseal.rs","src/bin/tseal.rs"]
  } and
  .provider_birth_boundary == {
    opaque_state:"ProviderNativePreauthenticatedSession",
    available:"ALLOW",
    missing:"REFUSED_PRE_BIRTH",
    unavailable:"REFUSED_PRE_BIRTH",
    ambiguous:"REFUSED_PRE_BIRTH",
    negative_process_birth:false,
    guards:[
      "scripts/probe/provider-capabilities.sh::require_preauthenticated_session",
      "src/adapters/identity.rs::require_preauthenticated_session"
    ],
    inventory:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/entrypoint-inventory.json",
    shell_source_scope:"RECURSIVE_GIT_TRACKED_REGULAR_SOURCES_UNDER_SCRIPTS_PROBE",
    shell_source_inventory:"CLOSED_PATH_AND_MODE_LIST",
    shell_extensions:[".sh",".bash",".zsh"],
    shell_regular_modes:["100644","100755"],
    extensionless_tracked_path:"REFUSED_REGARDLESS_OF_CONTENT",
    shell_symlink_or_mode_violation:"REFUSED_REGARDLESS_OF_EXTENSION",
    provider_command_positions:"ALL_STATIC_DIRECT_WITH_MULTILINE_CONTINUATIONS",
    ambiguous_provider_dispatch:"REFUSED",
    guard_requirement:"SAME_FILE_EXECUTABLE_GUARD_BEFORE_EVERY_BIRTH_OCCURRENCE",
    tracked_probe_shell_source_count:1,
    cli_entrypoint_count:2,
    provider_birth_count:5,
    future_unguarded_route_allowed:false
  } and
  .owner_ssot == {
    worklog_prefix:{
      path:"/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/TASKSEAL-WORKLOG.jsonl",
      line_count:130,
      sha256:"ea68139bbe3ea5505a5120da6bb9ec735f5f63355c36bdc256f5d97b1d8a0a83"
    },
    tail_policy:"APPEND_ONLY_VALID_JSONL_EVENTS",
    mutable_tail_hashed:false,
    terminal_review_event_may_append:true
  } and
  .git.input_head == "6a54523b4dbf8869ca7035a7dc42df88ac11c45a" and
  .git.input_tree == "7f3b2870967dbf8864e65bf15b02e7661b7b1995" and
  .git.history_policy == "SINGLE_PARENT_IMMUTABLE_PREFIX" and
  .git.receipt_topology == "PARENT_BOUND_RECEIPT_ONLY" and
  .git.implementation_write_set == [
    "fixtures/cli/first-screen-unqualified-narrow.txt",
    "fixtures/cli/first-screen-unqualified-non-tty.txt",
    "fixtures/cli/first-screen-unqualified-plain.txt",
    "fixtures/cli/first-screen-unqualified-tty.txt",
    "fixtures/cli/first-screen-unqualified.txt",
    "reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/consolidated.json",
    "reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/entrypoint-inventory.json",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/entrypoint-inventory.rb",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory-allowlist.json",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-entrypoint-inventory.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt-durability.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-verify.sh",
    "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    "scripts/probe/provider-capabilities.sh",
    "src/adapters/codex/identity.rs",
    "src/adapters/environment.rs",
    "src/adapters/identity.rs",
    "src/adapters/mod.rs",
    "src/adapters/session.rs",
    "src/cli/mod.rs",
    "src/cli/screen.rs",
    "src/cli/state.rs",
    "src/cli/zero_auth.rs",
    "tests/adapters/codex/identity.rs",
    "tests/adapters/identity.rs",
    "tests/cli.rs",
    "tests/cli/argv_passthrough.rs",
    "tests/cli/first_screen.rs",
    "tests/cli/saved_start_call_path.rs",
    "tests/cli/saved_starts.rs",
    "tests/contracts/provider_capability_truth.rs"
  ] and
  .verification == {
    sole_gate:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/verify.sh",
    mutation_suite:"scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-verify.sh",
    source_inventory:"CURRENT_TRACKED_INVENTORY",
    public_boundary_inventory:"CURRENT_GIT_GOVERNED_RELEASE_INVENTORY",
    public_boundary_paths:[
      ".gitignore","AGENTS.md","CHANGELOG.md","Cargo.lock","Cargo.toml","GOVERNANCE.md","LICENSE","SECURITY.md",
      "adapters/declarations","controls","deny.toml","fixtures/adapters","fixtures/catalog","fixtures/cli","fixtures/contracts","fixtures/core",
      "reports/contracts","rust-toolchain.toml","schemas/canonical-json-profile.md","schemas/contracts","scripts/check-control-coverage.rb",
      "scripts/check-public-boundary.sh","scripts/probe","src","tests"
    ],
    public_boundary_forbidden_tracked_prefixes:["scripts/runtime/"],
    full_zero_auth_call_paths:true,
    task_receipts_validated:[1,2,3,4]
  } and
  .terminal_review == {state:"PENDING",checkpoint_mutable:false,append_only_event_allowed:true}
' "$report" >/dev/null || refuse CONSOLIDATED_REPORT_CONTRACT

test "$(sha_file "$checkpoint")" = b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68 || refuse CHECKPOINT_DIGEST
test "$(git rev-parse "$input_head^{tree}")" = 7f3b2870967dbf8864e65bf15b02e7661b7b1995 || refuse INPUT_TREE

prefix_lines=$(jq -r '.owner_ssot.worklog_prefix.line_count' "$report")
prefix_digest=$(jq -r '.owner_ssot.worklog_prefix.sha256' "$report")
test "$(wc -l <"$worklog" | tr -d ' ')" -ge "$prefix_lines" || refuse WORKLOG_PREFIX_MISSING
test "$(head -n "$prefix_lines" "$worklog" | shasum -a 256 | awk '{print $1}')" = "$prefix_digest" || refuse WORKLOG_PREFIX_DIGEST
jq -s -e '
  all(.[]; type == "object" and .schema == "taskseal.worklog.event.v1" and (.event_id | type == "string" and length > 0)) and
  ([.[].event_id] | length) == ([.[].event_id] | unique | length)
' "$worklog" >/dev/null || refuse WORKLOG_APPEND_CONTRACT

transcript_index=0
while test "$transcript_index" -lt 4; do
  transcript_path=$(jq -r --argjson index "$transcript_index" '.transcripts[$index].path' "$report")
  transcript_sha=$(jq -r --argjson index "$transcript_index" '.transcripts[$index].sha256' "$report")
  test "$(sha_file "$root/$transcript_path")" = "$transcript_sha" || refuse TRANSCRIPT_DIGEST
  transcript_index=$((transcript_index + 1))
done

for task in 1 2 3; do
  task_index=$((task - 1))
  task_path=$(jq -r --argjson index "$task_index" '.task_receipts[$index].path' "$report")
  task_commit=$(jq -r --argjson index "$task_index" '.task_receipts[$index].commit' "$report")
  task_sha=$(jq -r --argjson index "$task_index" '.task_receipts[$index].sha256' "$report")
  test "$(git show "$task_commit:$task_path" | shasum -a 256 | awk '{print $1}')" = "$task_sha" || refuse TASK_RECEIPT_DIGEST
done

for task in 1 2 3 4; do
  expected="P06_ZERO_AUTH_TASK_${task}_RECEIPT_PASS"
  actual=$("$root/$successor/test-task-$task-receipt.sh" 2>&1) || {
    test -z "$actual" || printf '%s\n' "$actual" >&2
    refuse TASK_RECEIPT
  }
  test "$actual" = "$expected" || refuse TASK_RECEIPT_OUTPUT
done

task_4_receipt="$root/reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json"
implementation_head=$(jq -er '.binding.implementation_result_head' "$task_4_receipt") || refuse IMPLEMENTATION_HEAD
git merge-base --is-ancestor "$input_head" "$implementation_head" || refuse IMPLEMENTATION_LINEAGE
test -z "$(git rev-list --min-parents=2 "$input_head..HEAD")" || refuse MERGE_TOPOLOGY
expected_write_set=$(jq -r '.git.implementation_write_set[]' "$report")
test "$(git diff --name-only "$input_head..$implementation_head")" = "$expected_write_set" || refuse IMPLEMENTATION_WRITE_SET
history_write_set=$(git log --format= --name-only "$input_head..$implementation_head" | sed '/^$/d' | LC_ALL=C sort -u)
test "$history_write_set" = "$expected_write_set" || refuse IMPLEMENTATION_HISTORY_WRITE_SET

if test "${P06_ZERO_AUTH_METADATA_ONLY:-0}" = 1; then
  printf '%s\n' P06_ZERO_AUTH_METADATA_PASS
  exit 0
fi

test -z "$(git status --porcelain=v1 --untracked-files=all)" || refuse DIRTY_TRACKED_OR_UNTRACKED_STATE

owner_root=/Users/ysorokin/Documents/it/5-LVL\ -\ 2026/Temp\ in\ Projects/wisdom/taskseal
control_output=$(ruby "$root/$successor/control-validator.rb" \
  "$owner_root/TASKSEAL-OWNER-DECISIONS.md" \
  "$owner_root/TASKSEAL-MASTER-PROGRAM.md" \
  "$owner_root/TASKSEAL-REQUIREMENTS-TRACEABILITY.md" \
  "$root/controls/v0.1-execution-map.tsv") || refuse CONTROL_VALIDATOR
test "$control_output" = P06_ZERO_AUTH_CONTROL_PASS || refuse CONTROL_OUTPUT

source_output=$(ruby "$root/$successor/source-inventory.rb" "$root") || refuse SOURCE_INVENTORY
test "$source_output" = P06_ZERO_AUTH_SOURCE_INVENTORY_PASS || refuse SOURCE_INVENTORY_OUTPUT

entrypoint_output=$(ruby "$root/$successor/entrypoint-inventory.rb" \
  "$root" "$root/$successor/entrypoint-inventory.json") || refuse ENTRYPOINT_INVENTORY
test "$entrypoint_output" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS || refuse ENTRYPOINT_INVENTORY_OUTPUT

rustfmt --edition 2024 --check --config skip_children=true \
  src/adapters/codex/identity.rs src/adapters/environment.rs src/adapters/identity.rs \
  src/adapters/mod.rs src/adapters/session.rs \
  src/cli/mod.rs src/cli/screen.rs src/cli/state.rs src/cli/zero_auth.rs \
  tests/adapters/codex/identity.rs tests/adapters/identity.rs tests/cli.rs \
  tests/cli/argv_passthrough.rs tests/cli/first_screen.rs \
  tests/cli/saved_start_call_path.rs tests/cli/saved_starts.rs \
  tests/contracts/provider_capability_truth.rs
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline --test cli --test adapters --test trace_metadata
cargo test --locked --offline --test provider_capability_truth \
  provider_probe_requires_opaque_preauthentication_before_process_birth \
  -- --exact --test-threads=1

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-public.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
test -z "$(git ls-files -- 'scripts/runtime/**')" || refuse PUBLIC_BOUNDARY_RUNTIME
public_paths=$(jq -r '.verification.public_boundary_paths[]' "$report")
# The current governed release inventory excludes gate/receipt evidence, which
# is covered separately by the closed executable-source scanner and receipts.
# This avoids interpreting evidence path identifiers as release tokens.
git archive --format=tar HEAD -- $public_paths | tar -xf - -C "$scratch"
public_output=$("$scratch/scripts/check-public-boundary.sh" --root "$scratch") || refuse PUBLIC_BOUNDARY
test "$public_output" = PUBLIC_BOUNDARY_PASS || refuse PUBLIC_BOUNDARY_OUTPUT

durability_output=$("$root/$successor/test-task-4-receipt-durability.sh") || refuse TASK_4_DURABILITY
test "$durability_output" = P06_ZERO_AUTH_TASK_4_RECEIPT_DURABILITY_PASS || refuse TASK_4_DURABILITY_OUTPUT

printf '%s\n' P06_ZERO_AUTH_PREAUTHENTICATED_NATIVE_SESSION_V1_PASS

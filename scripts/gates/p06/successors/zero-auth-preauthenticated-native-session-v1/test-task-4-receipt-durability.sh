#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-task4-descendant.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json
stale_receipt_commit=96687d9d40804d3751263b4a8c93ee4d95c2b4d1
stale_receipt_blob=17b805f4dd2516ba0b412b70a6c45591b4106001
stale_receipt_sha256=98baf6328c73b4196c6d2c7fbec333b4adf61cfad4c2ef65ced745c7aea84dcd

git clone -q --no-hardlinks "$root" "$scratch/stale"
git -C "$scratch/stale" config user.name taskseal-receipt-fixture
git -C "$scratch/stale" config user.email taskseal-receipt-fixture@invalid
cp "$scratch/stale/$receipt_rel" "$scratch/stale-receipt.json"
implementation_head=$(jq -r '.binding.implementation_result_head' "$scratch/stale-receipt.json")
git -C "$scratch/stale" checkout -q --detach "$implementation_head"
jq \
  --arg commit "$stale_receipt_commit" \
  --arg blob "$stale_receipt_blob" \
  --arg sha256 "$stale_receipt_sha256" '
  .binding.replaces_receipt_commit = $commit |
  .binding.predecessor_task_receipt = {
    path:"reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-4.json",
    commit:$commit,
    blob_oid:$blob,
    sha256:$sha256
  }
' "$scratch/stale-receipt.json" > "$scratch/stale/$receipt_rel"
git -C "$scratch/stale" add "$receipt_rel"
git -C "$scratch/stale" commit -q -m 'fixture: stale self-consistent receipt ancestry'

if actual=$("$scratch/stale/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh" 2>&1); then
  printf '%s\n' P06_ZERO_AUTH_EXPECTED_TASK_4_RECEIPT_REFUSAL_MISSING:STALE_SELF_CONSISTENT_96687D9 >&2
  exit 1
fi
test "$actual" = P06_ZERO_AUTH_TASK_4_RECEIPT_REFUSAL:PREDECESSOR_RECEIPT_ANCESTRY

git clone -q --no-hardlinks "$root" "$scratch/descendant"
git -C "$scratch/descendant" config user.name taskseal-receipt-fixture
git -C "$scratch/descendant" config user.email taskseal-receipt-fixture@invalid
git -C "$scratch/descendant" commit -q --allow-empty -m 'fixture: descendant after task 4 receipt'

actual=$("$scratch/descendant/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-4-receipt.sh" 2>&1)
test "$actual" = P06_ZERO_AUTH_TASK_4_RECEIPT_PASS

printf '%s\n' P06_ZERO_AUTH_TASK_4_RECEIPT_DURABILITY_PASS

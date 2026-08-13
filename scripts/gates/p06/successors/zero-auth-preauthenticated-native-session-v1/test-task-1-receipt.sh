#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
receipt_rel=reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json
receipt="$root/$receipt_rel"

# Break caught: Task 1 cannot be accepted without its parent-bound receipt.
if ! test -f "$receipt"; then
  printf '%s\n' P06_ZERO_AUTH_RED_MISSING_TASK_1_RECEIPT
  exit 1
fi

cd "$root"
implementation_head=$(jq -er '.binding.implementation_result_head' "$receipt")
implementation_tree=$(jq -er '.binding.implementation_tree' "$receipt")

jq -e \
  --arg implementation_head "$implementation_head" \
  --arg implementation_tree "$implementation_tree" '
  .schema_version == "taskseal.p06.zero-auth-preauthenticated-native-session-v1.task-receipt.v1" and
  .plan_id == "P06-ZERO-AUTH-PREAUTHENTICATED-NATIVE-SESSION-V1" and
  .task == 1 and
  .result == "accepted" and
  .acceptance.id == "P06-ZERO-AUTH-T1-GOVERNANCE-V1" and
  .acceptance.control_ids == ["ADP-05", "AUTH-01", "OD-10"] and
  .binding.scheme == "parent-bound-receipt.v1" and
  .binding.input_head == "b9a8d2c9dda084d90b984f77faa8a26dd4f03b3a" and
  .binding.implementation_result_head == $implementation_head and
  .binding.implementation_tree == $implementation_tree and
  .binding.receipt_commit_parent == $implementation_head and
  .inputs.plan_checkpoint_sha256 == "b26939c0863cb8760baa89418d2817ddc732ab5cf5b1551d08218ef585f1ed68" and
  .controls.threshold_reconsideration_only == true and
  .controls.runtime_github_or_stargazer_check == false and
  .controls.automatic_auth_enablement == false and
  .controls.credential_ownership == "none" and
  .controls.provider_birth_on_unavailable_or_ambiguous_session == false and
  .controls.local_functionality_continues == true and
  .controls.network_access == "not invoked" and
  .controls.provider_or_auth_process == "not invoked" and
  .controls.credential_or_keychain_read == "not invoked" and
  .controls.main_mutation == false and
  (.subject.sources | length) == 6 and
  ([.subject.sources[].path] | unique | length) == 6 and
  (.evidence | map(.exit)) == [1, 0]
  ' "$receipt" >/dev/null

test "$(git rev-parse "$implementation_head^{tree}")" = "$implementation_tree"

while IFS=$(printf '\t') read -r path expected_sha; do
  test -f "$path"
  test "$(shasum -a 256 "$path" | awk '{print $1}')" = "$expected_sha"
done <<EOF
$(jq -r '.subject.sources[] | [.path, .sha256] | @tsv' "$receipt")
EOF

if git ls-files --error-unmatch "$receipt_rel" >/dev/null 2>&1; then
  test "$(git rev-parse HEAD^)" = "$implementation_head"
  test "$(git diff-tree --no-commit-id --name-only -r HEAD)" = "$receipt_rel"
else
  test "$(git rev-parse HEAD)" = "$implementation_head"
fi

printf '%s\n' P06_ZERO_AUTH_TASK_1_RECEIPT_PASS

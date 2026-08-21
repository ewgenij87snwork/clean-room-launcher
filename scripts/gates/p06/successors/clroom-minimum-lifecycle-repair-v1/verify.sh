#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/../../../../.." && pwd -P)
cd "$root"

authority=.taskseal-dev/execution-authority.json
receipt=reports/gates/p06/successors/clroom-minimum-lifecycle-repair-v1/result.json
plan='/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-21-p06-clroom-minimum-lifecycle-repair-v1.md'

refuse() {
  printf '%s\n' "P06_CLROOM_MINIMUM_LIFECYCLE_REPAIR_V1_REFUSED:$1" >&2
  exit 2
}

test -f "$authority" || refuse AUTHORITY_MISSING
test -f "$receipt" || refuse RESULT_MISSING
test -f "$plan" || refuse PLAN_MISSING

authority_plan=$(jq -r '.plan_id' "$authority")
authority_first=$(jq -r '.allowed_task_first' "$authority")
authority_last=$(jq -r '.allowed_task_last' "$authority")
authority_head=$(jq -r '.head' "$authority")
test "$authority_plan" = P06-CLROOM-MINIMUM-LIFECYCLE-REPAIR-V1 || refuse AUTHORITY_PLAN
test "$authority_first" = 1 && test "$authority_last" = 1 || refuse AUTHORITY_TASK_RANGE
test "$authority_head" = "$(git rev-parse HEAD)" || refuse AUTHORITY_HEAD

plan_sha=$(/usr/bin/shasum -a 256 "$plan" | /usr/bin/awk '{print $1}')
test "$plan_sha" = ac8cfbe2d755ccf6f65a88022f9b2a75cc0712af4dae4e54fc37e83853e7de4f || refuse PLAN_DIGEST

rustfmt --edition 2024 --check --config skip_children=true \
  src/cli/mod.rs \
  src/cli/process.rs \
  src/cli/screen.rs \
  tests/cli.rs \
  tests/cli/argv_passthrough.rs \
  tests/cli/first_screen.rs \
  tests/cli/isolated_codex_launch.rs \
  tests/cli/minimum_real_launch.rs \
  tests/cli/non_tty_json.rs

cargo clippy --offline --bin clroom --test cli -- -D warnings
cargo test --offline --test execution_bootstrap \
  checked_in_authority_schema_accepts_only_the_exact_private_receipt -- --exact
cargo test --offline --test cli

if grep -R -F 'process::launch_codex' src/cli >/dev/null 2>&1; then
  refuse DIRECT_CODEX_FALLBACK
fi
if grep -R -F 'command accepted' src/cli >/dev/null 2>&1; then
  refuse FALSE_LIFECYCLE_SUCCESS
fi
grep -F 'return match launch_isolated_codex(&[])' src/cli/mod.rs >/dev/null \
  || refuse GUIDED_ISOLATED_ROUTE_MISSING
grep -F 'LOCAL_LIFECYCLE_UNAVAILABLE:' src/cli/mod.rs >/dev/null \
  || refuse LIFECYCLE_REFUSAL_MISSING

if ! jq -e '
  (keys | sort) == ([
    "ambient_skill_denied",
    "fake_provider_processes",
    "gate_command",
    "gate_marker",
    "global_agents_denied",
    "input_head",
    "interactive_enter_isolated",
    "lifecycle_refusal_count",
    "missing_codex_class",
    "non_tty_provider_birth",
    "plan_id",
    "plan_sha256",
    "project_canary_read",
    "provider_requests",
    "raw_retained",
    "real_provider_processes",
    "schema_version",
    "subjects"
  ] | sort) and
  .schema_version == "clroom.minimum-lifecycle-repair.result.v1" and
  .plan_id == "P06-CLROOM-MINIMUM-LIFECYCLE-REPAIR-V1" and
  .input_head == "297bf2c1d00c56dc3024b5de5cdefe5817703a75" and
  .plan_sha256 == "ac8cfbe2d755ccf6f65a88022f9b2a75cc0712af4dae4e54fc37e83853e7de4f" and
  .fake_provider_processes == 1 and
  .real_provider_processes == 0 and
  .provider_requests == 0 and
  .interactive_enter_isolated == true and
  .project_canary_read == true and
  .global_agents_denied == true and
  .ambient_skill_denied == true and
  .non_tty_provider_birth == false and
  .missing_codex_class == "LOCAL_CODEX_UNAVAILABLE" and
  .lifecycle_refusal_count == 7 and
  .raw_retained == false and
  .gate_command == "scripts/gates/p06/successors/clroom-minimum-lifecycle-repair-v1/verify.sh" and
  .gate_marker == "P06_CLROOM_MINIMUM_LIFECYCLE_REPAIR_V1_PASS" and
  (.subjects | type) == "object" and
  (.subjects | keys | length) == 14 and
  all(.subjects[]; test("^[0-9a-f]{64}$"))
' "$receipt" >/dev/null; then
  refuse RESULT_INVALID
fi

for subject in $(jq -r '.subjects | keys[]' "$receipt"); do
  test -f "$subject" || refuse SUBJECT_MISSING
  expected=$(jq -r --arg subject "$subject" '.subjects[$subject]' "$receipt")
  actual=$(/usr/bin/shasum -a 256 "$subject" | /usr/bin/awk '{print $1}')
  test "$actual" = "$expected" || refuse SUBJECT_DIGEST
done

git diff --check
printf '%s\n' \
  'P06_CLROOM_MINIMUM_LIFECYCLE_REPAIR_V1_PASS fake_provider_processes=1 real_provider_processes=0 provider_requests=0'

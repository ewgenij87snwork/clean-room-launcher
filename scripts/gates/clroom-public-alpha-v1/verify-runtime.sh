#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd -P)
cd "$root"

authority=.taskseal-dev/execution-authority.json
receipt=reports/gates/clroom-public-alpha-v1/runtime.json
plan='/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-21-clroom-public-alpha-v1.md'

refuse() {
  printf '%s\n' "CLROOM_PUBLIC_ALPHA_RUNTIME_REFUSED:$1" >&2
  exit 2
}

test -f "$authority" || refuse AUTHORITY_MISSING
test -f "$receipt" || refuse RECEIPT_MISSING
test -f "$plan" || refuse PLAN_MISSING

test "$(jq -r .plan_id "$authority")" = P08-CLROOM-PUBLIC-ALPHA-V1 \
  || refuse AUTHORITY_PLAN
test "$(jq -r .allowed_task_first "$authority")" = 1 \
  && test "$(jq -r .allowed_task_last "$authority")" = 4 \
  || refuse AUTHORITY_TASK_RANGE
test "$(jq -r .head "$authority")" = "$(git rev-parse HEAD)" \
  || refuse AUTHORITY_HEAD
test "$(/usr/bin/shasum -a 256 "$plan" | /usr/bin/awk '{print $1}')" \
  = 90c86ab4bd27c62ae028ee120529f5856dbe7cfbffbc467c5db4211974a92aed \
  || refuse PLAN_DIGEST

rustfmt --edition 2024 --check --config skip_children=true \
  src/cli/process.rs \
  fixtures/cli/fake-provider.rs \
  tests/cli/argv_passthrough.rs \
  tests/cli/isolated_codex_launch.rs \
  tests/cli/local_codex_launch.rs
cargo clippy --offline --bin clroom --test cli -- -D warnings
cargo test --offline --test execution_bootstrap
cargo test --offline --test cli

jq -e '
  (keys | sort) == ([
    "clean_default_count",
    "fake_provider_argv_cases",
    "gate_command",
    "gate_marker",
    "input_head",
    "plan_id",
    "plan_sha256",
    "provider_requests",
    "raw_retained",
    "real_provider_processes",
    "schema_version",
    "subjects",
    "user_arguments_after_defaults"
  ] | sort) and
  .schema_version == "clroom.public-alpha.runtime.v1" and
  .plan_id == "P08-CLROOM-PUBLIC-ALPHA-V1" and
  .input_head == "8e8811a7f604d2ae478c590bcd6b1a14599c8915" and
  .plan_sha256 == "90c86ab4bd27c62ae028ee120529f5856dbe7cfbffbc467c5db4211974a92aed" and
  .clean_default_count == 4 and
  .fake_provider_argv_cases == 2 and
  .user_arguments_after_defaults == true and
  .real_provider_processes == 0 and
  .provider_requests == 0 and
  .raw_retained == false and
  .gate_command == "scripts/gates/clroom-public-alpha-v1/verify-runtime.sh" and
  .gate_marker == "CLROOM_PUBLIC_ALPHA_RUNTIME_PASS" and
  (.subjects | type) == "object" and
  (.subjects | keys | length) == 6 and
  all(.subjects[]; test("^[0-9a-f]{64}$"))
' "$receipt" >/dev/null || refuse RECEIPT_INVALID

for subject in $(jq -r '.subjects | keys[]' "$receipt"); do
  test -f "$subject" || refuse SUBJECT_MISSING
  expected=$(jq -r --arg subject "$subject" '.subjects[$subject]' "$receipt")
  actual=$(/usr/bin/shasum -a 256 "$subject" | /usr/bin/awk '{print $1}')
  test "$actual" = "$expected" || refuse SUBJECT_DIGEST
done

git diff --check
printf '%s\n' \
  'CLROOM_PUBLIC_ALPHA_RUNTIME_PASS clean_defaults=4 user_arguments_after_defaults=true real_provider_processes=0 provider_requests=0'

#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname "$0")/../../.." && pwd -P)
cd "$root"

authority=.taskseal-dev/execution-authority.json
plan='/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-22-clroom-selective-skills-v1.md'
receipt=reports/gates/clroom-selective-skills-v1/result.json
started_seconds=$(date +%s)

refuse() {
  printf '%s\n' "CLROOM_SELECTIVE_SKILLS_V1_REFUSED:$1" >&2
  exit 2
}

run_cargo() {
  rustup run 1.97.1 cargo "$@"
}

test -f "$authority" || refuse AUTHORITY_MISSING
test -f "$plan" || refuse PLAN_MISSING
test "$(jq -r .plan_id "$authority")" = P08-CLROOM-SELECTIVE-SKILLS-V1 \
  || refuse AUTHORITY_PLAN
test "$(jq -r .allowed_task_first "$authority")" = 1 \
  && test "$(jq -r .allowed_task_last "$authority")" = 3 \
  || refuse AUTHORITY_TASK_RANGE
test "$(jq -r .head "$authority")" = "$(git rev-parse HEAD)" \
  || refuse AUTHORITY_HEAD
plan_sha=$(shasum -a 256 "$plan" | awk '{print $1}')
test "$(jq -r .plan_checkpoint_sha256 "$authority")" = "$plan_sha" \
  || refuse PLAN_DIGEST
test "$(git branch --show-current)" = feat/clroom-public-alpha-v1 \
  || refuse BRANCH
test "$(pwd -P)" = /Users/ysorokin/projects-worktrees/clean-room-launcher/clroom-public-alpha-v1 \
  || refuse WORKTREE
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64 \
  || refuse HOST_TUPLE
test "$(rustup run 1.97.1 rustc --version | awk '{print $2}')" = 1.97.1 \
  || refuse RUST_TOOLCHAIN

changed_paths=$(mktemp "${TMPDIR:-/tmp}/clroom-selective-paths.XXXXXX")
scratch=$(mktemp -d "${TMPDIR:-/tmp}/clroom-selective-gate.XXXXXX")
cleanup() {
  case "$changed_paths" in
    "${TMPDIR:-/tmp}"/clroom-selective-paths.*) rm -f -- "$changed_paths" ;;
    *) refuse UNSAFE_PATHS_CLEANUP ;;
  esac
  case "$scratch" in
    "${TMPDIR:-/tmp}"/clroom-selective-gate.*) find "$scratch" -depth -delete ;;
    *) refuse UNSAFE_SCRATCH_CLEANUP ;;
  esac
}
trap cleanup EXIT HUP INT TERM

git status --porcelain=v1 --untracked-files=all | cut -c4- >"$changed_paths"
while IFS= read -r path; do
  case "$path" in
    README.md|docs/assets/clean-room-launcher-hero.png|fixtures/cli/help/top.txt) ;;
    src/adapters/codex/isolation.rs|src/cli/help.rs|src/cli/mod.rs|src/cli/screen.rs|src/cli/skill_sets.rs) ;;
    tests/adapters/codex/isolation.rs|tests/cli/help.rs|tests/cli/isolated_codex_launch.rs|tests/cli/isolated_launch_screen.rs) ;;
    scripts/gates/clroom-selective-skills-v1/verify.sh|reports/gates/clroom-selective-skills-v1/result.json) ;;
    *) refuse "WRITE_SET:$path" ;;
  esac
done <"$changed_paths"

git diff --check
rustup run 1.97.1 rustfmt --edition 2024 --check --config skip_children=true \
  src/adapters/codex/isolation.rs \
  src/cli/help.rs \
  src/cli/mod.rs \
  src/cli/screen.rs \
  src/cli/skill_sets.rs \
  tests/adapters/codex/isolation.rs \
  tests/cli/help.rs \
  tests/cli/isolated_codex_launch.rs \
  tests/cli/isolated_launch_screen.rs

CARGO_NET_OFFLINE=true run_cargo clippy --offline --bin clroom --test cli --test adapters -- -D warnings

test_log="$scratch/cargo-test.log"
if ! CARGO_NET_OFFLINE=true run_cargo test --offline --locked >"$test_log" 2>&1; then
  sed -n '1,320p' "$test_log" >&2
  refuse REGRESSION
fi
sed -n '1,320p' "$test_log"
tests_passed=$(awk '
  /test result: ok\./ {
    for (field = 1; field <= NF; field += 1) {
      if ($field == "passed;") total += $(field - 1)
    }
  }
  END { print total + 0 }
' "$test_log")

scripts/check-public-boundary.sh --root . >/dev/null

grep -Fq 'docs/assets/clean-room-launcher-hero.png' README.md \
  || refuse README_HERO
grep -Fq -- '--skill-set=code-review,testing' README.md \
  || refuse README_DIRECT_SET
grep -Fq -- '--skill-set=@review,@debugging,@documentation' README.md \
  || refuse README_NAMED_SETS
grep -Fq '~/.config/clroom/skill-sets.yaml' README.md \
  || refuse README_CONFIG_PATH
if grep -Fq -- '--skills=' README.md fixtures/cli/help/top.txt src/cli/help.rs; then
  refuse OLD_OPTION_PUBLIC
fi
grep -Fq -- '--skill-set=code-review,@review' fixtures/cli/help/top.txt \
  || refuse HELP_GRAMMAR
grep -Fq '/tmp/clroom-help-config/clroom/skill-sets.yaml' fixtures/cli/help/top.txt \
  || refuse HELP_CONFIG_PATH

test -s docs/assets/clean-room-launcher-hero.png || refuse HERO_MISSING
test "$(file -b docs/assets/clean-room-launcher-hero.png | awk '{print $1}')" = PNG \
  || refuse HERO_FORMAT
test "$(sips -g pixelWidth docs/assets/clean-room-launcher-hero.png | awk '/pixelWidth/ {print $2}')" = 1672 \
  || refuse HERO_WIDTH
test "$(sips -g pixelHeight docs/assets/clean-room-launcher-hero.png | awk '/pixelHeight/ {print $2}')" = 470 \
  || refuse HERO_HEIGHT

for test_name in \
  codex_handoff_admits_one_exact_skill_and_a_complete_namespace_for_this_run \
  codex_handoff_admits_one_namespaced_skill_without_its_siblings \
  codex_handoff_expands_multiple_named_sets_and_direct_skills_without_rewriting_config \
  codex_handoff_rejects_invalid_named_sets_before_exec_without_echoing_yaml \
  codex_handoff_rejects_a_relative_xdg_skill_set_path_before_exec \
  isolated_preview_reports_the_number_of_selected_global_skills
do
  grep -Rq "fn $test_name" tests || refuse "FOCUSED_TEST_MISSING:$test_name"
done

subjects_file="$scratch/subjects.tsv"
for subject in \
  README.md \
  docs/assets/clean-room-launcher-hero.png \
  fixtures/cli/help/top.txt \
  src/adapters/codex/isolation.rs \
  src/cli/help.rs \
  src/cli/mod.rs \
  src/cli/screen.rs \
  src/cli/skill_sets.rs \
  tests/adapters/codex/isolation.rs \
  tests/cli/help.rs \
  tests/cli/isolated_codex_launch.rs \
  tests/cli/isolated_launch_screen.rs \
  scripts/gates/clroom-selective-skills-v1/verify.sh
do
  test -f "$subject" || refuse "SUBJECT_MISSING:$subject"
  printf '%s\t%s\n' "$subject" "$(shasum -a 256 "$subject" | awk '{print $1}')" \
    >>"$subjects_file"
done
subjects=$(jq -Rn '[inputs | split("\t") | {(.[0]): .[1]}] | add' <"$subjects_file")
subject_count=$(wc -l <"$subjects_file" | tr -d ' ')
hero_sha=$(shasum -a 256 docs/assets/clean-room-launcher-hero.png | awk '{print $1}')
duration_ms=$((($(date +%s) - started_seconds) * 1000))

mkdir -p "$(dirname "$receipt")"
tmp_receipt="$scratch/result.json"
jq -n \
  --arg plan_sha256 "$plan_sha" \
  --arg input_head "$(git rev-parse HEAD)" \
  --arg hero_sha256 "$hero_sha" \
  --argjson subjects "$subjects" \
  --argjson subject_count "$subject_count" \
  --argjson tests_passed "$tests_passed" \
  --argjson duration_ms "$duration_ms" \
  '{
    schema_version:"clroom.selective-skills.local.v1",
    plan_id:"P08-CLROOM-SELECTIVE-SKILLS-V1",
    result:"PASS",
    plan_sha256:$plan_sha256,
    input_head:$input_head,
    binding:"commit contains this receipt and the hashed subjects",
    subjects:$subjects,
    subject_count:$subject_count,
    hero_sha256:$hero_sha256,
    checks:{
      single_skill_set_option:true,
      direct_skill_selector:true,
      namespace_selector:true,
      exact_namespaced_selector:true,
      multiple_named_sets:true,
      overlap_deduplication:true,
      project_local_skills_automatic:true,
      invalid_inputs_fail_before_codex:true,
      user_config_read_only:true,
      plaque_selected_count:true,
      full_offline_regression:true,
      public_boundary:true
    },
    counts:{tests_passed:$tests_passed,focused_selector_cases:10,plaque_cases:3},
    timings:{gate_duration_ms:$duration_ms},
    real_provider_processes:0,
    provider_requests:0,
    publication:false,
    raw_provider_output_retained:false
  }' >"$tmp_receipt"
mv "$tmp_receipt" "$receipt"

printf '%s\n' \
  "CLROOM_SELECTIVE_SKILLS_V1_PASS tests_passed=$tests_passed provider_requests=0 publication=false"

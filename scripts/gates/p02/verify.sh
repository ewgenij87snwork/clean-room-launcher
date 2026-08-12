#!/bin/sh
set -eu

root=$(cd "$(dirname "$0")/../../.." && pwd -P)
cd "$root"
report="reports/gates/p02/contract-gate.json"
mkdir -p "$(dirname "$report")"

run_gate() {
  name=$1
  shift
  log=$(mktemp "${TMPDIR:-/tmp}/taskseal-p02-gate.XXXXXX")
  if "$@" >"$log" 2>&1; then
    printf '{"name":%s,"exit":0,"status":"PASS"}' "$(jq -Rn --arg value "$name" '$value')"
  else
    code=$?
    printf '{"name":%s,"exit":%s,"status":"FAIL"}' "$(jq -Rn --arg value "$name" '$value')" "$code"
    printf '\n%s\n' "$(cat "$log")" >&2
    case "$log" in
      "${TMPDIR:-/tmp}"/taskseal-p02-gate.*) rm -f -- "$log" ;;
      *) exit 70 ;;
    esac
    exit "$code"
  fi
  case "$log" in
    "${TMPDIR:-/tmp}"/taskseal-p02-gate.*) rm -f -- "$log" ;;
    *) exit 70 ;;
  esac
}

results="$(mktemp "${TMPDIR:-/tmp}/taskseal-p02-results.XXXXXX")"
cleanup() {
  case "$results" in
    "${TMPDIR:-/tmp}"/taskseal-p02-results.*) rm -f -- "$results" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

run_gate "fmt" cargo fmt --all -- --check >>"$results"
run_gate "clippy" cargo clippy --all-targets --locked --offline -- -D warnings >>"$results"
run_gate "contract-tests" cargo test --all-targets --locked --offline >>"$results"
run_gate "public-boundary" scripts/check-public-boundary.sh --root "$root" >>"$results"
run_gate "control-coverage" ruby scripts/check-control-coverage.rb >>"$results"

clean_clone_gate() {
  source_head=$(git rev-parse HEAD)
  clone=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p02-clone.XXXXXX")
  git clone --local --no-hardlinks "$root" "$clone"
  git -C "$clone" checkout --quiet -b gate/p02-clean "$source_head"
  test -f "$root/.taskseal-dev/execution-authority.json"
  mkdir -p "$clone/.taskseal-dev"
  jq --arg worktree "$clone" --arg head "$source_head" \
    '.worktree_realpath=$worktree | .branch="gate/p02-clean" | .head=$head' \
    "$root/.taskseal-dev/execution-authority.json" \
    >"$clone/.taskseal-dev/execution-authority.json"
  (cd "$clone" && cargo test --all-targets --locked --offline)
  case "$clone" in
    "${TMPDIR:-/tmp}"/taskseal-p02-clone.*) rm -rf -- "$clone" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}

run_gate "clean-local-clone" clean_clone_gate >>"$results"

for task in 1 2 3 4 5 6 7 8; do
  receipt="reports/gates/p02/task-$task.json"
  jq -e --argjson task "$task" '.plan == "P02" and .task == $task and (.result_head | test("^[0-9a-f]{40}$")) and .skips_counted_as_pass == 0' "$receipt" >/dev/null || {
    echo "RECEIPT_INVALID:$task" >&2
    exit 78
  }
done
printf '{"name":"receipts","exit":0,"status":"PASS"}\n' >>"$results"

if rg -n 'SKIP|skipped' "$results"; then
  echo "HIDDEN_SKIP" >&2
  exit 79
fi

head=$(git rev-parse HEAD)
status=$(git status --porcelain=v1)
if [ -n "$status" ]; then
  echo "DIRTY_WORKTREE" >&2
  exit 80
fi

commands=$(jq -s . "$results")
jq -n \
  --arg schema "taskseal.p02.contract-gate.v1" \
  --arg plan "P02" \
  --arg result_head "$head" \
  --arg branch "$(git branch --show-current)" \
  --arg worktree '\$TASKSEAL_WORKTREE' \
  --argjson commands "$commands" \
  '{schema_version:$schema,plan:$plan,result:"PASS",result_head:$result_head,branch:$branch,worktree:$worktree,commands:$commands,accepted_tasks:8,skips_counted_as_pass:0,network_or_provider_spend:false}' > "$report"
printf 'P02_CONTRACT_GATE_PASS 8/8 focused tasks; Task 9 gate itself is the next receipt\n'

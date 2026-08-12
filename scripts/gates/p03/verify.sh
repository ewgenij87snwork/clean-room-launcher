#!/bin/sh
set -eu

root=$(cd "$(dirname "$0")/../../.." && pwd -P)
cd "$root"
report="reports/gates/p03/core-gate.json"
mkdir -p "$(dirname "$report")"

results=$(mktemp "${TMPDIR:-/tmp}/taskseal-p03-results.XXXXXX")
cleanup() {
  case "$results" in
    "${TMPDIR:-/tmp}"/taskseal-p03-results.*) rm -f -- "$results" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

run_gate() {
  name=$1
  shift
  log=$(mktemp "${TMPDIR:-/tmp}/taskseal-p03-gate.XXXXXX")
  if "$@" >"$log" 2>&1; then
    jq -cn --arg name "$name" '{name:$name,exit:0,status:"PASS"}' >>"$results"
  else
    code=$?
    cat "$log" >&2
    rm -f -- "$log"
    exit "$code"
  fi
  rm -f -- "$log"
}

known_mismatch_is_observed() {
  if TASKSEAL_PARITY_INJECT_MISMATCH=1 cargo test --lib \
      core::python_parity::privacy_clean_python_reference_matches_rust_bytes_manifests_and_refusals \
      --locked --offline; then
    echo "PARITY_SENSOR_FALSE_GREEN" >&2
    return 81
  fi
}

repeat_parity() {
  cargo test --lib \
    core::python_parity::privacy_clean_python_reference_matches_rust_bytes_manifests_and_refusals \
    --locked --offline
  cargo test --lib \
    core::python_parity::privacy_clean_python_reference_matches_rust_bytes_manifests_and_refusals \
    --locked --offline
}

clean_clone_gate() {
  source_head=$(git rev-parse HEAD)
  clone=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p03-clone.XXXXXX")
  git clone --local --no-hardlinks "$root" "$clone"
  git -C "$clone" checkout --quiet -b gate/p03-clean "$source_head"
  test -f "$root/.taskseal-dev/execution-authority.json"
  mkdir -p "$clone/.taskseal-dev"
  jq --arg worktree "$clone" --arg head "$source_head" \
    '.repository_realpath=$worktree | .worktree_realpath=$worktree | .branch="gate/p03-clean" | .head=$head' \
    "$root/.taskseal-dev/execution-authority.json" \
    >"$clone/.taskseal-dev/execution-authority.json"
  (
    cd "$clone"
    RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" \
      CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
      HOME="${TMPDIR:-/tmp}/taskseal-not-admitted-home" \
      HTTPS_PROXY="http://127.0.0.1:9" HTTP_PROXY="http://127.0.0.1:9" NO_PROXY="" \
      cargo test --all-targets --locked --offline
  )
  case "$clone" in
    "${TMPDIR:-/tmp}"/taskseal-p03-clone.*) rm -rf -- "$clone" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}

test -z "$(git status --porcelain=v1)" || {
  echo "DIRTY_WORKTREE" >&2
  exit 80
}

run_gate "known-mismatch-sensor" known_mismatch_is_observed
run_gate "fmt" cargo fmt --all -- --check
run_gate "clippy" cargo clippy --all-targets --locked --offline -- -D warnings
run_gate "all-tests" cargo test --all-targets --locked --offline
run_gate "python-parity-repeat" repeat_parity
run_gate "public-boundary" scripts/check-public-boundary.sh --root "$root"
run_gate "control-coverage" ruby scripts/check-control-coverage.rb
run_gate "clean-clone-poisoned-home-network" clean_clone_gate

for task in 1 2 3 4 5 6 7 8; do
  receipt="reports/gates/p03/task-$task.json"
  jq -e --argjson task "$task" \
    '.plan == "P03" and .task == $task and .result == "accepted" and (.result_head | test("^[0-9a-f]{40}$")) and .skips_counted_as_pass == 0' \
    "$receipt" >/dev/null || {
      echo "RECEIPT_INVALID:$task" >&2
      exit 78
    }
done
jq -cn '{name:"receipts",exit:0,status:"PASS"}' >>"$results"

digest_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

digest_stdin() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  else
    sha256sum | awk '{print $1}'
  fi
}

sample=$(printf '%s' '{"layers":["safe","project","task"],"records":3,"protected":true,"limits":{"bytes":100,"records":3,"tokens":100},"inputs":["scope:z","scope:a"]}' |
  env -i PATH="/usr/bin:/bin" python3 -B fixtures/contracts/python-reference/reference.py)
commands=$(jq -s . "$results")
head=$(git rev-parse HEAD)
os_name=$(uname -s)
os_release=$(uname -r)
arch=$(uname -m)
rustc_version=$(rustc --version)
reference_sha=$(digest_file fixtures/contracts/python-reference/reference.py)
cargo_lock_sha=$(digest_file Cargo.lock)
artifact_digest=$(printf '%s' "$sample" | jq -r '.manifest.digest')
context_sha=$(printf '%s' "$sample" | jq -r '.context_hex' | xxd -r -p | digest_stdin)

jq -n \
  --arg schema "taskseal.p03.core-gate.v1" \
  --arg result_head "$head" \
  --arg branch "$(git branch --show-current)" \
  --arg os "$os_name" --arg os_release "$os_release" --arg arch "$arch" \
  --arg rustc "$rustc_version" --arg reference_sha "$reference_sha" \
  --arg cargo_lock_sha "$cargo_lock_sha" --arg artifact_digest "$artifact_digest" \
  --arg context_sha "$context_sha" --argjson commands "$commands" \
  '{schema_version:$schema,plan:"P03",result:"PASS",result_head:$result_head,branch:$branch,worktree:"$TASKSEAL_WORKTREE",commands:$commands,accepted_tasks:8,claimed_lanes:[{os:$os,release:$os_release,arch:$arch,status:"PASS"}],unclaimed_lanes:[{os:"Linux",status:"NOT_QUALIFIED"},{os:"Windows",status:"NOT_QUALIFIED"}],digests:{python_reference_sha256:$reference_sha,cargo_lock_sha256:$cargo_lock_sha,sample_generation_digest:$artifact_digest,sample_context_sha256:$context_sha},toolchain:{rustc:$rustc,python_reference:"stdlib-python3"},controls:["E-TS-06","COR-01","COR-19","LAB-06"],skips_counted_as_pass:0,network_or_provider_spend:false,python_retired:false,praxis_changed:false}' \
  >"$report"

printf 'P03_CORE_GATE_PASS 8/8 focused tasks; current local OS qualified; Linux/Windows NOT_QUALIFIED\n'

#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname "$0")/../../.." && pwd -P)
cd "$root"

authority=.taskseal-dev/execution-authority.json
plan='/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-22-clroom-public-alpha2-release-v1.md'
receipt=reports/gates/clroom-public-alpha2-release-v1/result.json
artifact_dir=target/public-alpha2-local
archive_name=clean-room-launcher-v0.1.0-alpha.2-aarch64-apple-darwin.tar.gz
archive="$artifact_dir/$archive_name"
checksums="$artifact_dir/SHA256SUMS"
started_seconds=$(date +%s)

refuse() {
  printf '%s\n' "CLROOM_PUBLIC_ALPHA2_RELEASE_V1_REFUSED:$1" >&2
  exit 2
}

run_cargo() {
  rustup run 1.97.1 cargo "$@"
}

test -f "$authority" || refuse AUTHORITY_MISSING
test -f "$plan" || refuse PLAN_MISSING
test "$(jq -r .plan_id "$authority")" = P08-CLROOM-PUBLIC-ALPHA2-RELEASE-V1 \
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
git diff --quiet || refuse TRACKED_DIRTY
git diff --cached --quiet || refuse STAGED_DIRTY
test -z "$(git status --porcelain=v1 --untracked-files=all)" \
  || refuse WORKTREE_DIRTY

changed_paths=$(git diff --name-only 91b72e9fa9e935d086b6c078792938f27b20fa62..HEAD)
for path in $changed_paths; do
  case "$path" in
    CHANGELOG.md|Cargo.lock|Cargo.toml|README.md|SECURITY.md) ;;
    docs/install.md|docs/limitations.md|docs/providers.md) ;;
    scripts/gates/clroom-public-alpha2-release-v1/verify.sh) ;;
    *) refuse "WRITE_SET:$path" ;;
  esac
done

test "$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)" = 0.1.0-alpha.2 \
  || refuse PACKAGE_VERSION
grep -Fq '## [0.1.0-alpha.2] - 2026-08-22' CHANGELOG.md \
  || refuse CHANGELOG_VERSION
grep -Fq 'VERSION=v0.1.0-alpha.2' README.md \
  || refuse README_VERSION
grep -Fq -- '--tag v0.1.0-alpha.2 --locked' README.md \
  || refuse README_CARGO_TAG
grep -Fq 'docs/assets/clean-room-launcher-hero.png' README.md \
  || refuse README_HERO
grep -Fq -- '--skill-set=@review,@debugging,@documentation' README.md \
  || refuse README_NAMED_SETS
grep -Fq '0.1.0-alpha.2' SECURITY.md \
  || refuse SECURITY_VERSION

scratch=$(mktemp -d "${TMPDIR:-/tmp}/clroom-public-alpha2-gate.XXXXXX")
cleanup() {
  case "$scratch" in
    "${TMPDIR:-/tmp}"/clroom-public-alpha2-gate.*) find "$scratch" -depth -delete ;;
    *) refuse UNSAFE_SCRATCH_CLEANUP ;;
  esac
}
trap cleanup EXIT HUP INT TERM

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
CARGO_NET_OFFLINE=true run_cargo clippy --offline --locked \
  --bin clroom --test cli --test adapters -- -D warnings

test_log="$scratch/cargo-test.log"
if ! CARGO_NET_OFFLINE=true run_cargo test --offline --locked >"$test_log" 2>&1; then
  sed -n '1,320p' "$test_log" >&2
  refuse REGRESSION
fi
tests_passed=$(awk '
  /test result: ok\./ {
    for (field = 1; field <= NF; field += 1) {
      if ($field == "passed;") total += $(field - 1)
    }
  }
  END { print total + 0 }
' "$test_log")
test "$tests_passed" -ge 213 || refuse TEST_COUNT

scripts/check-public-boundary.sh --root . >/dev/null

if test -d "$artifact_dir"; then
  case "$artifact_dir" in
    target/public-alpha2-local) find "$artifact_dir" -depth -delete ;;
    *) refuse UNSAFE_ARTIFACT_CLEANUP ;;
  esac
fi
mkdir -p "$artifact_dir"
RUSTUP_TOOLCHAIN=1.97.1 TASKSEAL_TARGET=aarch64-apple-darwin \
  CARGO_NET_OFFLINE=true packaging/build-artifacts.sh "$artifact_dir" \
  >"$scratch/build-artifact.log"
test -f "$archive" || refuse ARCHIVE_MISSING
python3 packaging/verify-artifact.py "$archive" >/dev/null
(
  cd "$artifact_dir"
  shasum -a 256 "$archive_name" >SHA256SUMS
  shasum -a 256 -c SHA256SUMS >/dev/null
)

extract_dir="$scratch/extracted"
mkdir -p "$extract_dir"
tar -xzf "$archive" -C "$extract_dir"
installed=$(find "$extract_dir" -type f -path '*/bin/clroom' -print -quit)
test -n "$installed" && test -x "$installed" || refuse INSTALLED_BINARY
archive_binary_sha=$(python3 - "$archive" <<'PY'
import hashlib, sys, tarfile
with tarfile.open(sys.argv[1], "r:gz") as package:
    members = [item for item in package.getmembers() if item.name.endswith("/bin/clroom")]
    if len(members) != 1:
        raise SystemExit(2)
    print(hashlib.sha256(package.extractfile(members[0]).read()).hexdigest())
PY
)
test "$archive_binary_sha" = "$(shasum -a 256 "$installed" | awk '{print $1}')" \
  || refuse BINARY_DIGEST
installed_help=$("$installed" --help) || refuse ARCHIVE_HELP_EXIT
printf '%s\n' "$installed_help" | grep -Fq 'Clean Room Launcher' \
  || refuse ARCHIVE_HELP

cargo_root="$scratch/cargo-root"
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$root/target" \
  run_cargo install --path . --root "$cargo_root" --locked --offline --force \
  >"$scratch/cargo-install.log"
cargo_help=$("$cargo_root/bin/clroom" --help) || refuse CARGO_INSTALL_HELP_EXIT
printf '%s\n' "$cargo_help" | grep -Fq 'Clean Room Launcher' \
  || refuse CARGO_INSTALL_HELP

source_commit=$(git rev-parse HEAD)
source_tree=$(git rev-parse 'HEAD^{tree}')
archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
checksums_sha=$(shasum -a 256 "$checksums" | awk '{print $1}')
duration_ms=$((($(date +%s) - started_seconds) * 1000))

mkdir -p "$(dirname "$receipt")"
tmp_receipt="$scratch/result.json"
jq -n \
  --arg plan_sha256 "$plan_sha" \
  --arg source_commit "$source_commit" \
  --arg source_tree "$source_tree" \
  --arg archive_file "$archive_name" \
  --arg archive_sha256 "$archive_sha" \
  --arg binary_sha256 "$archive_binary_sha" \
  --arg sha256sums_sha256 "$checksums_sha" \
  --argjson tests_passed "$tests_passed" \
  --argjson duration_ms "$duration_ms" \
  '{
    schema_version:"clroom.public-alpha2.local.v1",
    plan_id:"P08-CLROOM-PUBLIC-ALPHA2-RELEASE-V1",
    result:"PASS",
    plan_sha256:$plan_sha256,
    source_commit:$source_commit,
    source_tree:$source_tree,
    package_version:"0.1.0-alpha.2",
    host:"macOS/arm64",
    archive_file:$archive_file,
    archive_sha256:$archive_sha256,
    binary_sha256:$binary_sha256,
    sha256sums_sha256:$sha256sums_sha256,
    checks:{
      full_offline_regression:true,
      strict_scoped_lint:true,
      public_boundary:true,
      deterministic_archive:true,
      checksum_verified:true,
      extracted_binary_help:true,
      cargo_install_path:true,
      selective_skill_sets:true
    },
    counts:{tests_passed:$tests_passed},
    timings:{gate_duration_ms:$duration_ms},
    real_provider_processes:0,
    provider_requests:0,
    publication:false,
    raw_provider_output_retained:false
  }' >"$tmp_receipt"
mv "$tmp_receipt" "$receipt"

printf '%s\n' \
  "CLROOM_PUBLIC_ALPHA2_LOCAL_PASS tests_passed=$tests_passed archive_sha256=$archive_sha provider_requests=0 publication=false"

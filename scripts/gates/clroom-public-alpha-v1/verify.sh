#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname "$0")/../../.." && pwd -P)
cd "$root"

authority=.taskseal-dev/execution-authority.json
plan='/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-21-clroom-public-alpha-v1.md'
receipt=reports/gates/clroom-public-alpha-v1/result.json
artifact_dir=target/public-alpha
archive_name=clean-room-launcher-v0.1.0-alpha.1-aarch64-apple-darwin.tar.gz
archive="$artifact_dir/$archive_name"
checksums="$artifact_dir/SHA256SUMS"
mode=${1:-full}

refuse() {
  printf '%s\n' "CLROOM_PUBLIC_ALPHA_LOCAL_REFUSED:$1" >&2
  exit 2
}

case "$mode" in
  full|--resume-after-public-identity) ;;
  *) refuse USAGE ;;
esac

test -f "$authority" || refuse AUTHORITY_MISSING
test -f "$plan" || refuse PLAN_MISSING
test "$(jq -r .plan_id "$authority")" = P08-CLROOM-PUBLIC-ALPHA-V1 \
  || refuse AUTHORITY_PLAN
test "$(jq -r .allowed_task_first "$authority")" = 1 \
  && test "$(jq -r .allowed_task_last "$authority")" = 4 \
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
git diff --quiet || refuse TRACKED_DIRTY
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64 \
  || refuse HOST_TUPLE
test "$(rustc --version | awk '{print $2}')" = 1.97.1 \
  || refuse RUST_TOOLCHAIN

test "$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)" = 0.1.0-alpha.1 \
  || refuse PACKAGE_VERSION
grep -Fq 'docs/assets/clroom-alpha.gif' README.md || refuse README_GIF
grep -Fq 'explicit user override still wins' README.md || refuse README_OVERRIDE
grep -Fq 'unsigned and unnotarized' README.md || refuse README_ALPHA_BOUNDARY
grep -Fq 'Operation not permitted' README.md || refuse README_WARNING
grep -Fq '## [0.1.0-alpha.1]' CHANGELOG.md || refuse CHANGELOG_VERSION
test -s docs/assets/clroom-alpha.gif || refuse GIF_MISSING
test "$(file -b docs/assets/clroom-alpha.gif | awk '{print $1}')" = GIF \
  || refuse GIF_FORMAT
test "$(stat -f %z docs/assets/clroom-alpha.gif)" -le 2000000 \
  || refuse GIF_SIZE

if test "$mode" = full; then
  rustfmt --edition 2024 --check --config skip_children=true \
    src/cli/process.rs src/cli/screen.rs \
    tests/cli/argv_passthrough.rs tests/cli/isolated_codex_launch.rs \
    tests/cli/isolated_launch_screen.rs tests/cli/local_codex_launch.rs \
    tests/cli/minimum_real_launch.rs fixtures/cli/fake-provider.rs
  cargo clippy --offline --bin clroom --test cli -- -D warnings
  cargo test --offline --test execution_bootstrap
  cargo test --offline --test cli
  cargo test --offline --test public_boundary
  cargo test --offline --test public_identity
  scripts/check-public-boundary.sh --root .
else
  test "$(git rev-parse HEAD^)" = 30b8a5dafe9c38864a09de2386d819b9402eb163 \
    || refuse RESUME_PARENT
  changed=$(git diff --name-only HEAD^ HEAD)
  expected=$(printf '%s\n' \
    scripts/gates/clroom-public-alpha-v1/verify.sh \
    tests/public_identity.rs)
  test "$changed" = "$expected" || refuse RESUME_CHANGESET
  rustfmt --edition 2024 --check --config skip_children=true tests/public_identity.rs
  scripts/check-public-boundary.sh --root .
fi

find "$artifact_dir" -depth -delete 2>/dev/null || true
mkdir -p "$artifact_dir"
TASKSEAL_TARGET=aarch64-apple-darwin \
  CARGO_NET_OFFLINE=true \
  packaging/build-artifacts.sh "$artifact_dir" >/dev/null
test -f "$archive" || refuse ARCHIVE_MISSING
python3 packaging/verify-artifact.py "$archive" >/dev/null
(
  cd "$artifact_dir"
  shasum -a 256 "$archive_name" >SHA256SUMS
  shasum -a 256 -c SHA256SUMS >/dev/null
)

scratch=$(mktemp -d "${TMPDIR:-/tmp}/clroom-public-alpha.XXXXXX")
cleanup() {
  case "$scratch" in
    "${TMPDIR:-/tmp}"/clroom-public-alpha.*) find "$scratch" -depth -delete ;;
    *) refuse UNSAFE_TEMP_CLEANUP ;;
  esac
}
trap cleanup EXIT HUP INT TERM

tar -xzf "$archive" -C "$scratch"
installed=$(find "$scratch" -type f -path '*/bin/clroom' -print -quit)
test -n "$installed" && test -x "$installed" || refuse INSTALLED_BINARY
archive_binary_sha=$(python3 - "$archive" <<'PY'
import hashlib, sys, tarfile
with tarfile.open(sys.argv[1], "r:gz") as archive:
    members = [member for member in archive.getmembers() if member.name.endswith("/bin/clroom")]
    if len(members) != 1:
        raise SystemExit(2)
    print(hashlib.sha256(archive.extractfile(members[0]).read()).hexdigest())
PY
)
test "$archive_binary_sha" = "$(shasum -a 256 "$installed" | awk '{print $1}')" \
  || refuse BINARY_DIGEST
"$installed" --help | grep -Fq 'Clean Room Launcher' || refuse ARCHIVE_HELP

fake_root="$scratch/fake"
mkdir -p "$fake_root/bin" "$fake_root/home/.codex" "$fake_root/project"
rustc fixtures/cli/fake-provider.rs -o "$fake_root/bin/codex"
CLROOM_CAPTURE_PATH="$fake_root/argv" \
HOME="$fake_root/home" CODEX_HOME="$fake_root/home/.codex" \
PATH="$fake_root/bin:/usr/bin:/bin" \
  "$installed" codex --enable hooks --enable plugins features list
python3 - "$fake_root/argv" <<'PY'
import pathlib, sys
args = pathlib.Path(sys.argv[1]).read_bytes().split(b"\0")
if args[-1:] == [b""]:
    args.pop()
expected = [
    b"-c", b"features.hooks=false",
    b"-c", b"features.plugins=false",
    b"-c", b'developer_instructions=""',
    b"-c", b"notify=[]",
    b"--enable", b"hooks", b"--enable", b"plugins",
    b"features", b"list",
]
if args != expected:
    raise SystemExit(2)
PY

cargo_root="$scratch/cargo-root"
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$root/target" \
  cargo install --path . --root "$cargo_root" --locked --offline --force >/dev/null
"$cargo_root/bin/clroom" --help | grep -Fq 'Clean Room Launcher' \
  || refuse CARGO_INSTALL_HELP

source_commit=$(git rev-parse HEAD)
source_tree=$(git rev-parse 'HEAD^{tree}')
archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
checksums_sha=$(shasum -a 256 "$checksums" | awk '{print $1}')
gif_sha=$(shasum -a 256 docs/assets/clroom-alpha.gif | awk '{print $1}')
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
  --arg gif_sha256 "$gif_sha" \
  --arg execution_mode "$mode" \
  '{
    schema_version:"clroom.public-alpha.local.v1",
    plan_id:"P08-CLROOM-PUBLIC-ALPHA-V1",
    result:"PASS",
    plan_sha256:$plan_sha256,
    source_commit:$source_commit,
    source_tree:$source_tree,
    archive_file:$archive_file,
    archive_sha256:$archive_sha256,
    binary_sha256:$binary_sha256,
    sha256sums_sha256:$sha256sums_sha256,
    gif_sha256:$gif_sha256,
    execution_mode:$execution_mode,
    package_version:"0.1.0-alpha.1",
    host:"macOS/arm64",
    checks:{
      public_boundary:true,
      cli_tests:58,
      authority_bootstrap_tests:2,
      archive_verified:true,
      archive_install_help:true,
      cargo_install_help:true,
      clean_defaults:4,
      user_arguments_after_defaults:true
    },
    focused_public_identity_pass:true,
    fake_provider_processes:1,
    real_provider_processes:0,
    provider_requests:0,
    raw_provider_output_retained:false
  }' >"$tmp_receipt"
mv "$tmp_receipt" "$receipt"

printf '%s\n' \
  "CLROOM_PUBLIC_ALPHA_LOCAL_PASS archive_sha256=$archive_sha provider_requests=0"

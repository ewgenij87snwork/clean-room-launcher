#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd -P)
cd "$root"

mode=${1:-source}
authority=.taskseal-dev/execution-authority.json
plan='/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/plans/2026-08-21-clroom-public-alpha-v1.md'
checkpoint=cee60c83452b681eeab12a88c1688b799cd736be
gate_path=scripts/gates/clroom-public-alpha3-release-v1/verify.sh
evidence_dir=target/public-alpha3-release
source_receipt=$evidence_dir/source-result.json
archive_name=clean-room-launcher-v0.1.0-alpha.3-aarch64-apple-darwin.tar.gz
started_seconds=$(date +%s)

refuse() {
  printf '%s\n' "CLROOM_PUBLIC_ALPHA3_RELEASE_V1_REFUSED:$1" >&2
  exit 2
}

run_cargo() {
  rustup run 1.97.1 cargo "$@"
}

product_paths() {
  printf '%s\n' \
    CHANGELOG.md \
    Cargo.lock \
    Cargo.toml \
    README.md \
    SECURITY.md \
    docs/assets/clean-room-launcher-hero.png \
    docs/install.md \
    docs/limitations.md \
    docs/providers.md \
    fixtures/cli/first-screen-unqualified-narrow.txt \
    fixtures/cli/first-screen-unqualified-non-tty.txt \
    fixtures/cli/first-screen-unqualified-plain.txt \
    fixtures/cli/first-screen-unqualified-tty.txt \
    fixtures/cli/help/top.txt \
    src/adapters/codex/isolation.rs \
    src/cli/help.rs \
    src/cli/mod.rs \
    src/cli/screen.rs \
    tests/adapters/codex/isolation.rs \
    tests/cli/first_screen.rs \
    tests/cli/help.rs \
    tests/cli/isolated_codex_launch.rs \
    tests/cli/isolated_launch_screen.rs \
    tests/public_identity.rs
}

product_manifest() {
  manifest_root=$1
  product_paths | while IFS= read -r path; do
    test -f "$manifest_root/$path" || refuse "PRODUCT_PATH_MISSING:$path"
    digest=$(shasum -a 256 "$manifest_root/$path" | awk '{print $1}')
    printf '%s  %s\n' "$digest" "$path"
  done
}

verify_release_metadata() {
  metadata_root=$1
  test "$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$metadata_root/Cargo.toml" | head -1)" = 0.1.0-alpha.3 \
    || refuse PACKAGE_VERSION
  grep -Fq '## [0.1.0-alpha.3] - 2026-08-22' "$metadata_root/CHANGELOG.md" \
    || refuse CHANGELOG_VERSION
  grep -Fq 'VERSION=v0.1.0-alpha.3' "$metadata_root/README.md" \
    || refuse README_VERSION
  grep -Fq -- '--tag v0.1.0-alpha.3 --locked' "$metadata_root/README.md" \
    || refuse README_CARGO_TAG
  grep -Fq 'docs/assets/clean-room-launcher-hero.png' "$metadata_root/README.md" \
    || refuse README_HERO
  grep -Fq '0.1.0-alpha.3' "$metadata_root/SECURITY.md" \
    || refuse SECURITY_VERSION
  grep -Fq 'Clean Room Launcher v0.1.0-alpha.3' "$metadata_root/fixtures/cli/help/top.txt" \
    || refuse HELP_VERSION
}

test -f "$authority" || refuse AUTHORITY_MISSING
test -f "$plan" || refuse PLAN_MISSING
test "$(jq -r .plan_id "$authority")" = P08-CLROOM-PUBLIC-ALPHA3-RELEASE-V1 \
  || refuse AUTHORITY_PLAN
test "$(jq -r .allowed_task_first "$authority")" = 1 \
  && test "$(jq -r .allowed_task_last "$authority")" = 3 \
  || refuse AUTHORITY_TASK_RANGE
test "$(jq -r .head "$authority")" = "$checkpoint" || refuse AUTHORITY_HEAD
plan_sha=$(shasum -a 256 "$plan" | awk '{print $1}')
test "$(jq -r .plan_checkpoint_sha256 "$authority")" = "$plan_sha" \
  || refuse PLAN_DIGEST
test "$(git branch --show-current)" = feat/clroom-public-alpha-v1 || refuse BRANCH
test "$(pwd -P)" = /Users/ysorokin/projects-worktrees/clean-room-launcher/clroom-public-alpha-v1 \
  || refuse WORKTREE
git merge-base --is-ancestor "$checkpoint" HEAD || refuse CHECKPOINT_ANCESTRY
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64 || refuse HOST_TUPLE
test "$(rustup run 1.97.1 rustc --version | awk '{print $2}')" = 1.97.1 \
  || refuse RUST_TOOLCHAIN

verify_release_metadata "$root"
test "$(shasum -a 256 docs/assets/clean-room-launcher-hero.png | awk '{print $1}')" \
  = 858366d5d6668fa980e5ab57412c231866141cb0c195727253ca2f38a8f79e57 \
  || refuse HERO_DIGEST
test "$(shasum -a 256 docs/.DS_Store | awk '{print $1}')" \
  = beca1bcf0d1d40d7bc672c3bc754482c9197c42cf5942f3078dd79752996a2c7 \
  || refuse DOCS_DS_STORE_CHANGED
test "$(shasum -a 256 docs/assets/.DS_Store | awk '{print $1}')" \
  = d65165279105ca6773180500688df4bdc69a2c7b771752f0a46ef120b7fd8ec3 \
  || refuse ASSETS_DS_STORE_CHANGED

case "$mode" in
  source)
    test "$(git rev-parse HEAD)" = "$checkpoint" || refuse SOURCE_HEAD_MOVED
    git diff --cached --quiet || refuse STAGED_DIRTY

    scratch=$(mktemp -d "${TMPDIR:-/tmp}/clroom-public-alpha3-source.XXXXXX")
    cleanup() {
      case "$scratch" in
        "${TMPDIR:-/tmp}"/clroom-public-alpha3-source.*) find "$scratch" -depth -delete ;;
        *) refuse UNSAFE_SCRATCH_CLEANUP ;;
      esac
    }
    trap cleanup EXIT HUP INT TERM

    product_paths >"$scratch/expected-product-paths"
    git diff --name-only "$checkpoint" -- | while IFS= read -r path; do
      case "$path" in
        "$gate_path") ;;
        *) grep -Fxq "$path" "$scratch/expected-product-paths" \
          || refuse "WRITE_SET:$path" ;;
      esac
    done
    git ls-files --others --exclude-standard | while IFS= read -r path; do
      case "$path" in
        docs/.DS_Store|docs/assets/.DS_Store|"$gate_path") ;;
        reports/gates/clroom-duplicate-skill-winner-repair-v1/*) ;;
        reports/gates/clroom-help-preflight-copy-v1/*) ;;
        reports/gates/clroom-plaque-feature-state-v1/*) ;;
        reports/gates/clroom-skill-discovery-repair-v1/*) ;;
        scripts/gates/clroom-duplicate-skill-winner-repair-v1/*) ;;
        scripts/gates/clroom-help-preflight-copy-v1/*) ;;
        scripts/gates/clroom-plaque-feature-state-v1/*) ;;
        scripts/gates/clroom-skill-discovery-repair-v1/*) ;;
        *) refuse "UNTRACKED_PATH:$path" ;;
      esac
    done
    test -f "$gate_path" || refuse GATE_MISSING
    git diff --check "$checkpoint" --

    mkdir -p "$evidence_dir"
    product_manifest "$root" >"$scratch/product-manifest"
    product_manifest_sha=$(shasum -a 256 "$scratch/product-manifest" | awk '{print $1}')

    rustup run 1.97.1 rustfmt --edition 2024 --check --config skip_children=true \
      src/adapters/codex/isolation.rs \
      src/cli/help.rs \
      src/cli/mod.rs \
      src/cli/screen.rs \
      tests/adapters/codex/isolation.rs \
      tests/cli/first_screen.rs \
      tests/cli/help.rs \
      tests/cli/isolated_codex_launch.rs \
      tests/cli/isolated_launch_screen.rs \
      tests/public_identity.rs
    CARGO_NET_OFFLINE=true run_cargo clippy --offline --locked --all-targets -- -D warnings
    test_log="$evidence_dir/cargo-test.log"
    if ! CARGO_NET_OFFLINE=true run_cargo test --offline --locked >"$test_log" 2>&1; then
      sed -n '1,360p' "$test_log" >&2
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
    CARGO_NET_OFFLINE=true run_cargo build --offline --locked --bin clroom >/dev/null
    test "$(target/debug/clroom --version)" = 'clroom 0.1.0-alpha.3' \
      || refuse VERSION_SMOKE
    target/debug/clroom --help | grep -Fq 'Clean Room Launcher v0.1.0-alpha.3' \
      || refuse HELP_SMOKE

    duration_ms=$((($(date +%s) - started_seconds) * 1000))
    jq -n \
      --arg plan_sha256 "$plan_sha" \
      --arg checkpoint "$checkpoint" \
      --arg product_manifest_sha256 "$product_manifest_sha" \
      --argjson tests_passed "$tests_passed" \
      --argjson duration_ms "$duration_ms" \
      '{
        schema_version:"clroom.public-alpha3.source.v1",
        plan_id:"P08-CLROOM-PUBLIC-ALPHA3-RELEASE-V1",
        phase:"source",
        result:"PASS",
        plan_sha256:$plan_sha256,
        checkpoint:$checkpoint,
        package_version:"0.1.0-alpha.3",
        product_manifest_sha256:$product_manifest_sha256,
        checks:{
          full_offline_regression:true,
          strict_all_target_lint:true,
          formatting:true,
          public_boundary:true,
          version_surface:true,
          hero_preserved:true,
          ds_store_preserved:true
        },
        counts:{tests_passed:$tests_passed},
        timings:{gate_duration_ms:$duration_ms},
        provider_requests:0,
        publication:false
      }' >"$source_receipt"
    printf '%s\n' \
      "CLROOM_PUBLIC_ALPHA3_SOURCE_PASS tests_passed=$tests_passed product_manifest_sha256=$product_manifest_sha provider_requests=0 publication=false"
    ;;

  public)
    test "$#" -eq 2 || refuse PUBLIC_ROOT_REQUIRED
    test -f "$source_receipt" || refuse SOURCE_RECEIPT_MISSING
    test "$(jq -r .result "$source_receipt")" = PASS || refuse SOURCE_NOT_PASS
    public_root=$(cd "$2" && pwd -P)
    test "$public_root" != "$root" || refuse PUBLIC_ROOT_IS_INTERNAL_ROOT
    git -C "$public_root" diff --quiet || refuse PUBLIC_TRACKED_DIRTY
    git -C "$public_root" diff --cached --quiet || refuse PUBLIC_STAGED_DIRTY
    test -z "$(git -C "$public_root" status --porcelain=v1 --untracked-files=all)" \
      || refuse PUBLIC_WORKTREE_DIRTY
    verify_release_metadata "$public_root"
    "$public_root/scripts/check-public-boundary.sh" --root "$public_root" >/dev/null

    scratch=$(mktemp -d "${TMPDIR:-/tmp}/clroom-public-alpha3-public.XXXXXX")
    cleanup() {
      case "$scratch" in
        "${TMPDIR:-/tmp}"/clroom-public-alpha3-public.*) find "$scratch" -depth -delete ;;
        *) refuse UNSAFE_SCRATCH_CLEANUP ;;
      esac
    }
    trap cleanup EXIT HUP INT TERM

    product_manifest "$public_root" >"$scratch/product-manifest"
    product_manifest_sha=$(shasum -a 256 "$scratch/product-manifest" | awk '{print $1}')
    test "$product_manifest_sha" = "$(jq -r .product_manifest_sha256 "$source_receipt")" \
      || refuse PUBLIC_PRODUCT_DRIFT

    artifact_dir="$public_root/target/public-alpha3-artifacts"
    if test -d "$artifact_dir"; then
      case "$artifact_dir" in
        "$public_root"/target/public-alpha3-artifacts) find "$artifact_dir" -depth -delete ;;
        *) refuse UNSAFE_ARTIFACT_CLEANUP ;;
      esac
    fi
    mkdir -p "$artifact_dir"
    (
      cd "$public_root"
      RUSTUP_TOOLCHAIN=1.97.1 TASKSEAL_TARGET=aarch64-apple-darwin \
        CARGO_NET_OFFLINE=true packaging/build-artifacts.sh "$artifact_dir" \
        >"$scratch/build-artifact.log"
    )
    archive="$artifact_dir/$archive_name"
    checksums="$artifact_dir/SHA256SUMS"
    test -f "$archive" || refuse ARCHIVE_MISSING
    python3 "$public_root/packaging/verify-artifact.py" "$archive" >/dev/null
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
    test "$("$installed" --version)" = 'clroom 0.1.0-alpha.3' \
      || refuse ARCHIVE_VERSION
    "$installed" --help | grep -Fq 'Clean Room Launcher v0.1.0-alpha.3' \
      || refuse ARCHIVE_HELP
    public_commit=$(git -C "$public_root" rev-parse HEAD)
    tar -xOf "$archive" '*/VERSION' | grep -Fq "source_commit=$public_commit" \
      || refuse ARCHIVE_SOURCE_COMMIT

    cargo_root="$scratch/cargo-root"
    (
      cd "$public_root"
      CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$public_root/target" \
        run_cargo install --path . --root "$cargo_root" --locked --offline --force \
        >"$scratch/cargo-install.log"
    )
    test "$("$cargo_root/bin/clroom" --version)" = 'clroom 0.1.0-alpha.3' \
      || refuse CARGO_INSTALL_VERSION

    archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
    binary_sha=$(shasum -a 256 "$installed" | awk '{print $1}')
    checksums_sha=$(shasum -a 256 "$checksums" | awk '{print $1}')
    duration_ms=$((($(date +%s) - started_seconds) * 1000))
    jq -n \
      --arg plan_sha256 "$plan_sha" \
      --arg public_commit "$public_commit" \
      --arg public_tree "$(git -C "$public_root" rev-parse 'HEAD^{tree}')" \
      --arg product_manifest_sha256 "$product_manifest_sha" \
      --arg archive_file "$archive_name" \
      --arg archive_sha256 "$archive_sha" \
      --arg binary_sha256 "$binary_sha" \
      --arg sha256sums_sha256 "$checksums_sha" \
      --argjson tests_passed "$(jq -r .counts.tests_passed "$source_receipt")" \
      --argjson duration_ms "$duration_ms" \
      '{
        schema_version:"clroom.public-alpha3.final.v1",
        plan_id:"P08-CLROOM-PUBLIC-ALPHA3-RELEASE-V1",
        phase:"public-artifact",
        result:"PASS",
        plan_sha256:$plan_sha256,
        public_commit:$public_commit,
        public_tree:$public_tree,
        package_version:"0.1.0-alpha.3",
        product_manifest_sha256:$product_manifest_sha256,
        host:"macOS/arm64",
        archive_file:$archive_file,
        archive_sha256:$archive_sha256,
        binary_sha256:$binary_sha256,
        sha256sums_sha256:$sha256sums_sha256,
        checks:{
          source_gate_bound:true,
          public_boundary:true,
          exact_public_commit_archive:true,
          checksum_verified:true,
          extracted_binary_version_help:true,
          cargo_install_path:true
        },
        counts:{tests_passed:$tests_passed},
        timings:{gate_duration_ms:$duration_ms},
        provider_requests:0,
        publication:false
      }' >"$evidence_dir/public-result.json"
    printf '%s\n' \
      "CLROOM_PUBLIC_ALPHA3_FINAL_PASS public_commit=$public_commit archive_sha256=$archive_sha provider_requests=0 publication=false artifact_dir=$artifact_dir"
    ;;

  *) refuse "UNKNOWN_MODE:$mode" ;;
esac

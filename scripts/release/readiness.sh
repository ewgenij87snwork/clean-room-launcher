#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
version=${CLROOM_RELEASE_VERSION:-0.2.0}
artifact_dir=${CLROOM_ARTIFACT_DIR:-"$root/target/release-candidate"}

fail() {
  printf 'RELEASE_READINESS_BLOCKED:%s\n' "$1" >&2
  exit 1
}

cd "$root"
[[ "$version" == "0.2.0" ]] || fail "VERSION_EXPECTED_0.2.0"
git diff --check || fail "DIFF_CHECK"
git diff --quiet || fail "CLEAN_TREE_REQUIRED"

legacy_upper=$(printf '%s%s' TASK SEAL)
legacy_lower=$(printf '%s%s' task seal)
legacy_preview=$(printf '%s-%s' unsigned preview-only)
legacy_identity_pattern=$(printf '%s|%s|P0[678]|%s|/workspace/%s|local://%s' \
  "$legacy_upper" "$legacy_lower" "$legacy_preview" "$legacy_lower" "$legacy_lower")
legacy_files=(
  packaging/build-artifacts.sh packaging/targets.toml
  packaging/supply-chain/generate.sh packaging/supply-chain/policy.toml
  packaging/signing/policy.md .github/workflows/release-candidate.yml
)
if command -v rg >/dev/null 2>&1; then
  if rg -n "$legacy_identity_pattern" "${legacy_files[@]}"; then
    fail "LEGACY_RELEASE_IDENTITY"
  fi
else
  if grep -En "$legacy_identity_pattern" "${legacy_files[@]}"; then
    fail "LEGACY_RELEASE_IDENTITY"
  fi
fi

./scripts/check-public-boundary.sh --root "$root" || fail "PUBLIC_BOUNDARY"
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck packaging/build-artifacts.sh scripts/release/readiness.sh || fail "SHELLCHECK"
else
  bash -n packaging/build-artifacts.sh scripts/release/readiness.sh || fail "SHELL_SYNTAX"
fi
cargo test --locked --all-targets || fail "FULL_LOCKED_TESTS"

if ! command -v cargo-deny >/dev/null 2>&1; then
  fail "SCA_TOOL_NOT_AVAILABLE"
fi
cargo deny --config deny.toml --locked check || fail "DEPENDENCY_SCA_REVIEW"

rm -rf "$artifact_dir"
mkdir -p "$artifact_dir"
CLROOM_ARTIFACT_QUALIFICATION=QUALIFIED \
CLROOM_SOURCE_COMMIT=$(git rev-parse HEAD) \
CLROOM_TARGET='' \
  ./packaging/build-artifacts.sh "$artifact_dir" >/tmp/clroom-release-build.log
artifact=$(sed -n 's/^ARTIFACT=//p' /tmp/clroom-release-build.log)
[[ -n "$artifact" && -f "$artifact" ]] || fail "ARTIFACT_MISSING"
python3 packaging/verify-artifact.py "$artifact" || fail "ARTIFACT_METADATA"
python3 - "$artifact" <<'PY' || fail "ARTIFACT_METADATA"
import sys
import tarfile

with tarfile.open(sys.argv[1], "r:gz") as archive:
    versions = [member for member in archive.getmembers() if member.isfile() and member.name.endswith("/VERSION")]
    if len(versions) != 1:
        raise SystemExit("expected exactly one VERSION")
    body = archive.extractfile(versions[0]).read().decode("utf-8")
    if "qualification=QUALIFIED\n" not in body:
        raise SystemExit("archive is not qualified")
    if "signing=unsigned\n" not in body:
        raise SystemExit("archive signing metadata is unexpected")
PY

metadata_dir="$artifact_dir/metadata"
./packaging/supply-chain/generate.sh generate \
  --artifact "$artifact" --source-commit "$(git rev-parse HEAD)" \
  --target "$(rustc -vV | sed -n 's/^host: //p')" \
  --builder-id local://clroom/release --output "$metadata_dir" >/tmp/clroom-release-metadata.log
./packaging/supply-chain/generate.sh verify --artifact "$artifact" --output "$metadata_dir" >/tmp/clroom-release-metadata-verify.log
shasum -a 256 "$artifact" "$metadata_dir"/sbom.cdx.json "$metadata_dir"/provenance.intoto.json
printf 'RELEASE_READINESS_PASS version=%s artifact=%s\n' "$version" "$artifact"

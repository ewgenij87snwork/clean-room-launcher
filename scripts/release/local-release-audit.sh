#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
cd "$root"

version=$(python3 - <<'PY'
import tomllib
with open("Cargo.toml", "rb") as handle:
    print(tomllib.load(handle)["package"]["version"])
PY
)
review="release/reviews/v${version}.json"
[[ -f "$review" ]] || {
  echo "LOCAL_RELEASE_AUDIT_BLOCKED:missing $review" >&2
  exit 1
}

baseline=${CLROOM_RELEASE_BASELINE_TAG:-}
if [[ -z "$baseline" ]]; then
  command -v gh >/dev/null 2>&1 || {
    echo "LOCAL_RELEASE_AUDIT_BLOCKED:gh required unless CLROOM_RELEASE_BASELINE_TAG is set" >&2
    exit 1
  }
  baseline=$(gh api "repos/y-sor/clean-room-launcher/releases/latest" --jq .tag_name)
fi
[[ -n "$baseline" ]] || {
  echo "LOCAL_RELEASE_AUDIT_BLOCKED:published baseline unresolved" >&2
  exit 1
}

git fetch --quiet --no-tags origin "refs/tags/$baseline:refs/tags/$baseline"

echo "=== Published baseline → candidate ==="
python3 scripts/release/check-release-delta.py   --baseline "$baseline"   --candidate HEAD   --review "$review"

echo
echo "=== Changelog section ==="
python3 - "$version" <<'PY'
import sys
from pathlib import Path
version=sys.argv[1]
inside=False
for line in Path("CHANGELOG.md").read_text(encoding="utf-8").splitlines():
    if line.startswith(f"## [{version}] - "):
        inside=True
    elif inside and line.startswith("## ["):
        break
    if inside:
        print(line)
PY

echo
echo "=== Exact release readiness ==="
CLROOM_RELEASE_VERSION="$version" scripts/release/readiness.sh

artifact="target/release-candidate/clean-room-launcher-v${version}-aarch64-apple-darwin.tar.gz"
[[ -f "$artifact" ]] || {
  echo "LOCAL_RELEASE_AUDIT_BLOCKED:artifact not found: $artifact" >&2
  exit 1
}

echo
echo "=== Artifact contents ==="
tar -tzf "$artifact"

echo
echo "=== Artifact checksum ==="
shasum -a 256 "$artifact"

echo
echo "LOCAL_RELEASE_AUDIT_PASS version=$version baseline=$baseline candidate=$(git rev-parse HEAD)"

#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
cd "$root"

verify=false
case "${1:-}" in
  "") ;;
  --verify) verify=true ;;
  *) echo "usage: $0 [--verify]" >&2; exit 2 ;;
esac

previous=$(python3 - <<'PY'
import tomllib
with open("release/review.toml", "rb") as handle:
    print(tomllib.load(handle)["previous_published_release"])
PY
)
version=$(python3 - <<'PY'
import tomllib
with open("release/review.toml", "rb") as handle:
    print(tomllib.load(handle)["version"])
PY
)
head=$(git rev-parse HEAD)

echo "RELEASE_VERSION=$version"
echo "PREVIOUS_PUBLISHED_RELEASE=$previous"
echo "CANDIDATE_HEAD=$head"
echo
echo "=== Commits since published release ==="
git log --oneline --decorate "$previous..$head"
echo
echo "=== Changed files ==="
git diff --name-status "$previous..$head"
echo
echo "=== Delta summary ==="
git diff --stat "$previous..$head"
echo
echo "=== Machine release contract ==="
python3 scripts/release/check-release-delta.py --candidate "$head" --previous-tag "$previous"

if [[ "$verify" == true ]]; then
  echo
  echo "=== Full local release readiness ==="
  CLROOM_RELEASE_VERSION="$version" scripts/release/readiness.sh
fi

echo
echo "LOCAL_RELEASE_AUDIT_PASS version=$version candidate=$head verify=$verify"

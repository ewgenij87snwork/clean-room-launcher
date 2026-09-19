#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
cd "$root"

full=false
if [[ ${1:-} == "--full" ]]; then full=true; shift; fi
[[ $# -eq 0 ]] || { echo "usage: scripts/release/local-release-audit.sh [--full]" >&2; exit 2; }

release_meta=$(python3 - <<'PY'
import json
import urllib.request

request = urllib.request.Request(
    "https://api.github.com/repos/y-sor/clean-room-launcher/releases/latest",
    headers={"Accept": "application/vnd.github+json", "User-Agent": "clroom-local-release-audit"},
)
with urllib.request.urlopen(request, timeout=20) as response:
    release = json.load(response)
print(
    release.get("tag_name", ""),
    str(bool(release.get("draft"))).lower(),
    str(bool(release.get("prerelease"))).lower(),
    str(release.get("immutable") is True).lower(),
    sep="\t",
)
PY
)
IFS=$'\t' read -r baseline draft prerelease immutable <<<"$release_meta"
[[ -n "$baseline" ]] || { echo "LOCAL_RELEASE_AUDIT_BLOCKED:PUBLISHED_BASELINE" >&2; exit 1; }
[[ "$draft" == false && "$prerelease" == false && "$immutable" == true ]] || {
  echo "LOCAL_RELEASE_AUDIT_BLOCKED:PUBLISHED_BASELINE_NOT_STABLE_IMMUTABLE" >&2
  exit 1
}
if ! git rev-parse --verify "refs/tags/$baseline^{commit}" >/dev/null 2>&1; then
  git fetch --quiet --no-tags origin "refs/tags/$baseline:refs/tags/$baseline"
fi

version=$(python3 - <<'PY'
import tomllib
with open("Cargo.toml", "rb") as handle:
    print(tomllib.load(handle)["package"]["version"])
PY
)
python3 scripts/release/resolve-release-lifecycle.py --self-test >/dev/null
lifecycle=$(python3 scripts/release/resolve-release-lifecycle.py \
  --candidate-version "$version" \
  --published-tag "$baseline")

if [[ "$lifecycle" == "ACTIVE_CANDIDATE" ]]; then
  python3 scripts/release/check-release-contract.py --report
else
  printf 'RELEASE_CONTRACT_SKIPPED lifecycle=POST_PUBLISH baseline=%s version=%s\n' "$baseline" "$version"
fi

echo
echo "=== RELEASE VERSION / DEPENDENCY DELTA ==="
git diff "$baseline..HEAD" -- Cargo.toml Cargo.lock

if ! $full; then
  echo
  if [[ "$lifecycle" == "POST_PUBLISH" ]]; then
    echo "SUMMARY_PASS lifecycle=POST_PUBLISH. Re-run with --full for local regression/security checks."
  else
    echo "SUMMARY_PASS lifecycle=ACTIVE_CANDIDATE. Re-run with --full for local tests + artifact verification."
  fi
  exit 0
fi

echo
echo "=== FULL LOCAL RELEASE CHECK ==="
git diff --check
./scripts/check-public-boundary.sh --root "$root"
sh install.sh --self-test
cargo test --locked --all-targets

if [[ "$lifecycle" == "POST_PUBLISH" ]]; then
  echo "LOCAL_RELEASE_AUDIT_PASS lifecycle=POST_PUBLISH head=$(git rev-parse HEAD) baseline=$baseline"
  exit 0
fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/clroom-local-release-audit.XXXXXX")
trap 'rm -rf -- "$tmp"' EXIT HUP INT TERM
CLROOM_SOURCE_COMMIT=$(git rev-parse HEAD) CLROOM_TARGET='' ./packaging/build-artifacts.sh "$tmp"
artifact=$(find "$tmp" -maxdepth 1 -type f -name 'clean-room-launcher-*.tar.gz' -print -quit)
test -n "$artifact"
python3 packaging/verify-artifact.py "$artifact"
shasum -a 256 "$artifact"
echo "LOCAL_RELEASE_AUDIT_PASS lifecycle=ACTIVE_CANDIDATE head=$(git rev-parse HEAD) baseline=$baseline artifact=$artifact"

#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 vMAJOR.MINOR.PATCH" >&2
  exit 2
}

[[ $# -eq 1 ]] || usage
tag=$1
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
command -v gh >/dev/null 2>&1 || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:GH_REQUIRED" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:JQ_REQUIRED" >&2; exit 1; }

repo=y-sor/clean-room-launcher
json=$(gh api "repos/$repo/releases/tags/$tag") || {
  echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:RELEASE_NOT_FOUND" >&2
  exit 1
}

test "$(jq -r .draft <<<"$json")" = false || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:STILL_DRAFT" >&2; exit 1; }
test "$(jq -r .prerelease <<<"$json")" = false || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:UNEXPECTED_PRERELEASE" >&2; exit 1; }
test "$(jq -r .tag_name <<<"$json")" = "$tag" || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:TAG_MISMATCH" >&2; exit 1; }
test "$(jq -r .name <<<"$json")" = "$tag — Clean Room Launcher" || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:TITLE_MISMATCH" >&2; exit 1; }
test "$(jq -r .immutable <<<"$json")" = true || { echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:NOT_IMMUTABLE" >&2; exit 1; }

artifact="clean-room-launcher-${tag}-aarch64-apple-darwin.tar.gz"
expected=(
  "$artifact"
  "$artifact.provenance.sigstore.json"
  "$artifact.sbom.sigstore.json"
  "SHA256SUMS"
  "install.sh"
  "sbom.cdx.json"
)
mapfile -t actual < <(jq -r '.assets[].name' <<<"$json" | sort)
mapfile -t wanted < <(printf "%s\n" "${expected[@]}" | sort)
[[ "${actual[*]}" == "${wanted[*]}" ]] || {
  echo "PUBLISHED_RELEASE_VERIFY_BLOCKED:ASSET_SET_MISMATCH" >&2
  printf "expected: %s\n" "${wanted[*]}" >&2
  printf "actual:   %s\n" "${actual[*]}" >&2
  exit 1
}

tmp=$(mktemp -d "${TMPDIR:-/tmp}/clroom-published-release.XXXXXX")
trap 'rm -rf -- "$tmp"' EXIT HUP INT TERM
gh release download "$tag" -R "$repo" --dir "$tmp" --clobber
(
  cd "$tmp"
  shasum -a 256 -c SHA256SUMS
  test -x install.sh || chmod 0755 install.sh
  sh install.sh --self-test
  gh attestation verify "$artifact" \
    -R "$repo" \
    --bundle "$artifact.provenance.sigstore.json" \
    --signer-workflow "$repo/.github/workflows/release.yml"
)

echo "PUBLISHED_RELEASE_VERIFY=PASS tag=$tag immutable=true assets=${#expected[@]}"

#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 vX.Y.Z EXPECTED_MAIN_SHA" >&2
  exit 2
}

fail() {
  printf 'RELEASE_TAG_PUSH_BLOCKED:%s\n' "$1" >&2
  exit "${2:-1}"
}

[[ $# -eq 2 ]] || usage
tag=$1
expected_main=$2
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "STABLE_TAG_REQUIRED"
[[ "$expected_main" =~ ^[0-9a-f]{40}$ ]] || fail "EXPECTED_MAIN_SHA"

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
cd "$root"

for command_name in git gh python3; do
  command -v "$command_name" >/dev/null 2>&1 || fail "COMMAND_MISSING:$command_name"
done
gh auth status >/dev/null 2>&1 || fail "GH_AUTH_REQUIRED"
[[ -z "$(git status --porcelain)" ]] || fail "WORKTREE_NOT_CLEAN"

git remote get-url origin >/dev/null 2>&1 || fail "ORIGIN_MISSING"
git fetch --quiet --no-tags origin main
local_head=$(git rev-parse HEAD)
remote_main=$(git rev-parse FETCH_HEAD)
[[ "$local_head" == "$expected_main" ]] || {
  printf 'LOCAL_HEAD=%s EXPECTED=%s\n' "$local_head" "$expected_main" >&2
  fail "LOCAL_HEAD_MOVED"
}
[[ "$remote_main" == "$expected_main" ]] || {
  printf 'REMOTE_MAIN=%s EXPECTED=%s\n' "$remote_main" "$expected_main" >&2
  fail "REMOTE_MAIN_MOVED"
}

python3 scripts/release/check-repository-release-policy.py >/dev/null || fail "REPOSITORY_TAG_POLICY"

if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  fail "REMOTE_TAG_ALREADY_EXISTS"
fi
version=${tag#v}
manifest_version=$(python3 - <<'PY'
import tomllib
with open("Cargo.toml", "rb") as handle:
    print(tomllib.load(handle)["package"]["version"])
PY
)
[[ "$manifest_version" == "$version" ]] || fail "PACKAGE_VERSION_MISMATCH"

title="$tag — Clean Room Launcher"
if git show-ref --verify --quiet "refs/tags/$tag"; then
  [[ "$(git cat-file -t "refs/tags/$tag")" == tag ]] || fail "LOCAL_TAG_NOT_ANNOTATED"
  [[ "$(git rev-list -n1 "$tag")" == "$expected_main" ]] || fail "LOCAL_TAG_TARGET_MISMATCH"
  release_date=$(git for-each-ref --format='%(taggerdate:short)' "refs/tags/$tag")
  [[ -n "$release_date" ]] || fail "LOCAL_TAG_DATE_MISSING"
  [[ "$(git for-each-ref --format='%(subject)' "refs/tags/$tag")" == "$title" ]] || fail "LOCAL_TAG_MESSAGE_MISMATCH"
else
  utc_now=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
  release_date=${utc_now%%T*}
  grep -Fqx "## [$version] - $release_date" CHANGELOG.md || fail "CHANGELOG_UTC_DATE_MISMATCH"
  GIT_COMMITTER_DATE="$utc_now" git tag -a "$tag" "$expected_main" -m "$title" || fail "ANNOTATED_TAG_CREATE"
fi

grep -Fqx "## [$version] - $release_date" CHANGELOG.md || fail "CHANGELOG_TAG_DATE_MISMATCH"
[[ "$(git cat-file -t "refs/tags/$tag")" == tag ]] || fail "ANNOTATED_TAG_TYPE"
[[ "$(git rev-list -n1 "$tag")" == "$expected_main" ]] || fail "TAG_TARGET"
[[ "$(git for-each-ref --format='%(taggerdate:short)' "refs/tags/$tag")" == "$release_date" ]] || fail "TAG_DATE"
[[ "$(git for-each-ref --format='%(subject)' "refs/tags/$tag")" == "$title" ]] || fail "TAG_MESSAGE"

# The protected remote v* tag is created only after every local invariant above passes.
git push origin "refs/tags/$tag:refs/tags/$tag" || fail "TAG_PUSH"

remote_object=$(git ls-remote --tags origin "refs/tags/$tag" | awk 'NR==1 {print $1}')
[[ -n "$remote_object" ]] || fail "REMOTE_TAG_NOT_VISIBLE"
printf 'RELEASE_TAG_PUSH_PASS tag=%s target=%s remote_tag_object=%s\n' "$tag" "$expected_main" "$remote_object"

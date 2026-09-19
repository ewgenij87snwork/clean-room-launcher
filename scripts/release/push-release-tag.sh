#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: CLROOM_OWNER_TAG_APPROVED=YES:<tag>:<sha> bash scripts/release/push-release-tag.sh <tag> <expected-main-sha>" >&2
  exit 64
}

[[ $# -eq 2 ]] || usage
tag=$1
expected=$2
[[ $tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$ ]] || usage
[[ $expected =~ ^[0-9a-f]{40}$ ]] || usage
[[ ${CLROOM_OWNER_TAG_APPROVED:-} == "YES:$tag:$expected" ]] || {
  echo "TAG_GATE_BLOCKED:OWNER_APPROVAL_TOKEN" >&2
  exit 65
}

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
cd "$root"

git diff --quiet
git diff --cached --quiet
git fetch --quiet origin main
actual_main=$(git rev-parse FETCH_HEAD)
[[ $actual_main == "$expected" ]] || {
  echo "TAG_GATE_BLOCKED:MAIN_DRIFT expected=$expected actual=$actual_main" >&2
  exit 66
}
[[ $(git rev-parse HEAD) == "$expected" ]] || {
  echo "TAG_GATE_BLOCKED:LOCAL_HEAD_NOT_ACCEPTED_MAIN" >&2
  exit 67
}

if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "TAG_GATE_BLOCKED:TAG_ALREADY_EXISTS" >&2
  exit 68
fi

python3 scripts/release/check-release-contract.py --report

version=${tag#v}
title="$tag — Clean Room Launcher"
git tag -a "$tag" "$expected" -m "$title"

cleanup_local_tag() {
  git tag -d "$tag" >/dev/null 2>&1 || true
}

[[ $(git cat-file -t "refs/tags/$tag") == tag ]] || {
  cleanup_local_tag
  echo "TAG_GATE_BLOCKED:ANNOTATED_TAG_REQUIRED" >&2
  exit 70
}
[[ $(git rev-parse "$tag^{}") == "$expected" ]] || {
  cleanup_local_tag
  echo "TAG_GATE_BLOCKED:TAG_TARGET_MISMATCH" >&2
  exit 71
}
[[ $(git for-each-ref --format='%(contents:subject)' "refs/tags/$tag") == "$title" ]] || {
  cleanup_local_tag
  echo "TAG_GATE_BLOCKED:TAG_TITLE_MISMATCH" >&2
  exit 72
}

tag_date=$(python3 - "$tag" <<'PY'
import datetime
import re
import subprocess
import sys

tag = sys.argv[1]
raw = subprocess.check_output(["git", "cat-file", "-p", f"refs/tags/{tag}"], text=True)
line = next((line for line in raw.splitlines() if line.startswith("tagger ")), None)
if line is None:
    raise SystemExit("tagger line missing")
match = re.search(r" (\\d+) ([+-])(\\d{2})(\\d{2})$", line)
if match is None:
    raise SystemExit("tagger timestamp malformed")
epoch = int(match.group(1))
minutes = int(match.group(3)) * 60 + int(match.group(4))
if match.group(2) == "-":
    minutes = -minutes
tz = datetime.timezone(datetime.timedelta(minutes=minutes))
print(datetime.datetime.fromtimestamp(epoch, tz=tz).date().isoformat())
PY
)
grep -Fxq "## [$version] - $tag_date" CHANGELOG.md || {
  cleanup_local_tag
  echo "TAG_GATE_BLOCKED:CHANGELOG_DATE expected=$tag_date" >&2
  exit 69
}

set +e
git push origin "refs/tags/$tag"
push_rc=$?
set -e

remote_peeled=$(git ls-remote --tags origin "refs/tags/$tag^{}" | awk '{print $1}')
if [[ $remote_peeled == "$expected" ]]; then
  echo "TAG_PUSH_PASS tag=$tag target=$expected"
  exit 0
fi

if [[ $push_rc -ne 0 ]]; then
  cleanup_local_tag
  echo "TAG_PUSH_OUTCOME_RECONCILED_NOT_PRESENT tag=$tag" >&2
  exit "$push_rc"
fi

echo "TAG_PUSH_BLOCKED:REMOTE_TARGET_NOT_RECONCILED" >&2
exit 73

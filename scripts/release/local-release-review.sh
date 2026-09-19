#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
usage: scripts/release/local-release-review.sh [--full] [--head REF]

Local release review helper for a CLROOM checkout.

Default:
  - resolves the latest published GitHub Release
  - fetches only that immutable release tag
  - audits the complete delta to HEAD
  - prints commits, changed files, change classes, and required gates

--full additionally runs:
  cargo test --locked --all-targets --target aarch64-apple-darwin
  sh install.sh --self-test

It does not create a tag, release, PR, install/update providers, or publish anything.
EOF
}

full=false
head_ref=HEAD
while [[ $# -gt 0 ]]; do
  case "$1" in
    --full) full=true; shift ;;
    --head)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      head_ref=$2
      shift 2
      ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done

root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "LOCAL_RELEASE_REVIEW_BLOCKED:NOT_A_GIT_CHECKOUT" >&2
  exit 1
}
cd "$root"

origin=$(git remote get-url origin 2>/dev/null || true)
case "$origin" in
  git@github.com:y-sor/clean-room-launcher.git|https://github.com/y-sor/clean-room-launcher|https://github.com/y-sor/clean-room-launcher.git) ;;
  *) echo "LOCAL_RELEASE_REVIEW_BLOCKED:UNEXPECTED_ORIGIN:$origin" >&2; exit 1 ;;
esac

command -v gh >/dev/null 2>&1 || { echo "LOCAL_RELEASE_REVIEW_BLOCKED:GH_REQUIRED" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "LOCAL_RELEASE_REVIEW_BLOCKED:PYTHON3_REQUIRED" >&2; exit 1; }

version=$(python3 - <<'PY'
import tomllib
with open("Cargo.toml", "rb") as handle:
    print(tomllib.load(handle)["package"]["version"])
PY
)
review="release/reviews/v${version}.json"
[[ -f "$review" ]] || { echo "LOCAL_RELEASE_REVIEW_BLOCKED:REVIEW_MANIFEST_MISSING:$review" >&2; exit 1; }

base_tag=$(gh api repos/y-sor/clean-room-launcher/releases/latest --jq .tag_name)
[[ -n "$base_tag" ]] || { echo "LOCAL_RELEASE_REVIEW_BLOCKED:PUBLISHED_BASELINE_MISSING" >&2; exit 1; }

review_base=$(python3 - "$review" <<'PY'
import json, sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["baseline_release"])
PY
)
[[ "$base_tag" == "$review_base" ]] || {
  echo "LOCAL_RELEASE_REVIEW_BLOCKED:BASELINE_DRIFT published=$base_tag reviewed=$review_base" >&2
  exit 1
}

git fetch --force --quiet origin "refs/tags/$base_tag:refs/tags/$base_tag"
git rev-parse "$base_tag^{commit}" >/dev/null
candidate=$(git rev-parse "$head_ref^{commit}")

output_dir=target/local-release-review
rm -rf "$output_dir"
mkdir -p "$output_dir"

python3 scripts/release/audit-release-delta.py --self-test
python3 scripts/release/audit-release-delta.py \
  --contract packaging/release-contract.json \
  --review "$review" \
  --base-ref "$base_tag" \
  --head-ref "$head_ref" \
  --version "$version" \
  --output-json "$output_dir/release-delta.json" \
  --output-markdown "$output_dir/release-delta.md"

echo
echo "=== RELEASE IDENTITY ==="
echo "VERSION=$version"
echo "PUBLISHED_BASELINE=$base_tag"
echo "CANDIDATE=$candidate"

echo
echo "=== COMMITS SINCE $base_tag ==="
git log --reverse --oneline "$base_tag..$head_ref"

echo
echo "=== FILE DELTA ==="
git diff --stat "$base_tag..$head_ref"
echo
git diff --name-status "$base_tag..$head_ref"

echo
echo "=== MACHINE RELEASE CONTRACT ==="
cat "$output_dir/release-delta.md"

if [[ "$full" == true ]]; then
  [[ "$(uname -s)" == Darwin && "$(uname -m)" == arm64 ]] || {
    echo "LOCAL_RELEASE_REVIEW_BLOCKED:FULL_REQUIRES_MACOS_ARM64" >&2
    exit 1
  }
  echo
  echo "=== FULL LOCAL VERIFICATION ==="
  rustup target list --installed | grep -Fxq aarch64-apple-darwin || {
    echo "LOCAL_RELEASE_REVIEW_BLOCKED:RUST_TARGET_MISSING:aarch64-apple-darwin" >&2
    exit 1
  }
  cargo test --locked --all-targets --target aarch64-apple-darwin
  sh install.sh --self-test
  git diff --check "$base_tag..$head_ref"
  echo "LOCAL_RELEASE_FULL_VERIFY=PASS"
fi

echo
echo "LOCAL_RELEASE_REVIEW=PASS"
echo "REPORT=$output_dir/release-delta.md"

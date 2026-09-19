#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 vX.Y.Z EXPECTED_TAG_SOURCE_SHA" >&2
  exit 2
}

fail() {
  printf 'RELEASE_PUBLISH_BLOCKED:%s\n' "$1" >&2
  exit "${2:-1}"
}

[[ $# -eq 2 ]] || usage
tag=$1
expected_source=$2
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "STABLE_TAG_REQUIRED"
[[ "$expected_source" =~ ^[0-9a-f]{40}$ ]] || fail "EXPECTED_SOURCE_SHA"

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
cd "$root"

for command_name in git gh python3 shasum; do
  command -v "$command_name" >/dev/null 2>&1 || fail "COMMAND_MISSING:$command_name"
done
gh auth status >/dev/null 2>&1 || fail "GH_AUTH_REQUIRED"

python3 scripts/release/check-repository-release-policy.py --mode strict >/dev/null || fail "REPOSITORY_TAG_POLICY"
immutable_enabled=$(gh api "repos/y-sor/clean-room-launcher/immutable-releases" --jq .enabled 2>/dev/null) \
  || fail "IMMUTABLE_RELEASE_POLICY_UNVERIFIED"
[[ "$immutable_enabled" == true ]] || fail "IMMUTABLE_RELEASE_POLICY_DISABLED"

remote_source=$(gh api "repos/y-sor/clean-room-launcher/commits/$tag" --jq .sha)
[[ "$remote_source" == "$expected_source" ]] || {
  printf 'REMOTE_TAG_SOURCE=%s EXPECTED=%s\n' "$remote_source" "$expected_source" >&2
  fail "TAG_SOURCE_MISMATCH"
}

release_json=$(gh release view "$tag" --json isDraft,isImmutable,isPrerelease,tagName,name,assets)
is_draft=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isDraft"]).lower())' <<<"$release_json")
is_immutable=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isImmutable"]).lower())' <<<"$release_json")
is_prerelease=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isPrerelease"]).lower())' <<<"$release_json")
tag_name=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["tagName"])' <<<"$release_json")
title=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["name"])' <<<"$release_json")
asset_count=$(python3 -c 'import json,sys; print(len(json.load(sys.stdin)["assets"]))' <<<"$release_json")
[[ "$is_draft" == true ]] || fail "RELEASE_NOT_DRAFT"
[[ "$is_immutable" == false ]] || fail "DRAFT_ALREADY_IMMUTABLE"
[[ "$is_prerelease" == false ]] || fail "UNEXPECTED_PRERELEASE"
[[ "$tag_name" == "$tag" ]] || fail "DRAFT_TAG_MISMATCH"
[[ "$title" == "$tag — Clean Room Launcher" ]] || fail "DRAFT_TITLE_MISMATCH"
[[ "$asset_count" -eq 6 ]] || fail "DRAFT_ASSET_COUNT"

version=${tag#v}
short_head=${expected_source:0:12}
evidence="target/release-evidence/draft-v${version}-${short_head}.json"
[[ -f "$evidence" ]] || fail "DRAFT_SMOKE_EVIDENCE_MISSING"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/clroom-publish-guard.XXXXXX")
cleanup() { rm -rf -- "$tmp"; }
trap cleanup EXIT HUP INT TERM
artifact="clean-room-launcher-v${version}-aarch64-apple-darwin.tar.gz"
gh release download "$tag" --pattern "$artifact" --dir "$tmp" >/dev/null || fail "DRAFT_ARCHIVE_DOWNLOAD"
current_sha=$(shasum -a 256 "$tmp/$artifact" | awk '{print $1}')

python3 - "$evidence" "$version" "$expected_source" "$tag" "$current_sha" <<'PY' || fail "DRAFT_SMOKE_EVIDENCE_INVALID"
import json
import sys
from pathlib import Path

path, version, source, tag, current_sha = sys.argv[1:]
record = json.loads(Path(path).read_text(encoding="utf-8"))
if record.get("schema_version") != "clroom.local-release-smoke.v1":
    raise SystemExit("schema")
if record.get("result") != "PASS" or record.get("phase") != "draft":
    raise SystemExit("result-phase")
if record.get("release_version") != version or record.get("source_head") != source:
    raise SystemExit("identity")
if record.get("release_tag") != tag:
    raise SystemExit("tag")
if record.get("artifact_sha256") != current_sha:
    raise SystemExit("artifact-drift")
auto = record.get("automated") or {}
human = record.get("human") or {}
for key in (
    "clean_init_only",
    "selected_init_only",
    "plugin_inventory_qualified",
    "persistent_config_unchanged",
):
    if auto.get(key) is not True:
        raise SystemExit("automated:" + key)
for key in (
    "codex_tui_confirmed",
    "claude_clean_tui_confirmed",
    "claude_clean_selected_skill_absent_confirmed",
    "claude_selected_plugin_tui_confirmed",
    "claude_selected_skill_visible_confirmed",
):
    if human.get(key) is not True:
        raise SystemExit("human:" + key)
if auto.get("model_prompt_sent") is not False or human.get("model_prompt_sent") is not False:
    raise SystemExit("model-prompt")
PY

# Publication is intentionally the last state-changing command after every guard above.
gh release edit "$tag" --draft=false --latest --verify-tag >/dev/null || fail "PUBLISH_ACTION"

after=$(gh release view "$tag" --json isDraft,isImmutable,isPrerelease,tagName,name)
after_draft=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isDraft"]).lower())' <<<"$after")
after_immutable=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isImmutable"]).lower())' <<<"$after")
after_tag=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["tagName"])' <<<"$after")
after_title=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["name"])' <<<"$after")
[[ "$after_draft" == false ]] || fail "PUBLISH_NOT_VISIBLE"
[[ "$after_immutable" == true ]] || fail "PUBLISHED_RELEASE_NOT_IMMUTABLE"
[[ "$after_tag" == "$tag" ]] || fail "PUBLISHED_TAG_MISMATCH"
[[ "$after_title" == "$tag — Clean Room Launcher" ]] || fail "PUBLISHED_TITLE_MISMATCH"

printf 'RELEASE_PUBLISH_PASS tag=%s source=%s artifact_sha256=%s immutable=true\n' "$tag" "$expected_source" "$current_sha"

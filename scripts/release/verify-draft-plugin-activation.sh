#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --tag vMAJOR.MINOR.PATCH --plugin-id name@marketplace" >&2
  exit 2
}

tag=
plugin_id=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag) tag=${2:-}; shift 2 ;;
    --plugin-id) plugin_id=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
[[ "$plugin_id" == *@* ]] || usage

[[ "$(uname -s)" == Darwin && "$(uname -m)" == arm64 ]] || {
  echo "DRAFT_PLUGIN_E2E_BLOCKED:MACOS_ARM64_REQUIRED" >&2
  exit 1
}
for cmd in gh python3 claude shasum tar; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "DRAFT_PLUGIN_E2E_BLOCKED:MISSING_$cmd" >&2; exit 1; }
done

claude_version=$(claude --version 2>&1 | sed -nE 's/.*([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -1)
[[ "$claude_version" == "2.1.273" ]] || {
  echo "DRAFT_PLUGIN_E2E_BLOCKED:CLAUDE_VERSION_EXPECTED_2.1.273 actual=$claude_version" >&2
  exit 1
}

tmp=$(mktemp -d "${TMPDIR:-/tmp}/clroom-draft-plugin-e2e.XXXXXX")
trap 'rm -rf -- "$tmp"' EXIT HUP INT TERM
repo=y-sor/clean-room-launcher
artifact="clean-room-launcher-${tag}-aarch64-apple-darwin.tar.gz"

gh release download "$tag" -R "$repo" --dir "$tmp" --pattern "$artifact" --pattern SHA256SUMS --clobber
(
  cd "$tmp"
  expected=$(awk -v asset="$artifact" '$2 == asset {print $1}' SHA256SUMS)
  actual=$(shasum -a 256 "$artifact" | awk '{print $1}')
  [[ -n "$expected" && "$actual" == "$expected" ]] || {
    echo "DRAFT_PLUGIN_E2E_BLOCKED:ARCHIVE_CHECKSUM_MISMATCH" >&2
    exit 1
  }
)

python3 - "$tmp/$artifact" "$tmp/extract" <<'PY'
import pathlib, sys, tarfile
archive, destination = map(pathlib.Path, sys.argv[1:])
destination.mkdir()
root = destination.resolve()
with tarfile.open(archive, "r:gz") as handle:
    members = handle.getmembers()
    for member in members:
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit("unsafe archive path")
        candidate = (root / pathlib.Path(*path.parts)).resolve(strict=False)
        if candidate != root and root not in candidate.parents:
            raise SystemExit("archive path escapes extraction root")
    handle.extractall(root)
PY

bin=$(find "$tmp/extract" -type f -path "*/bin/clroom" -print -quit)
[[ -n "$bin" && -x "$bin" ]] || { echo "DRAFT_PLUGIN_E2E_BLOCKED:CLROOM_BINARY_MISSING" >&2; exit 1; }

fingerprint() {
python3 <<'PY'
import hashlib, os
paths = [
    "~/.claude/settings.json",
    "~/.claude/settings.local.json",
    "~/.claude/plugins/installed_plugins.json",
    "~/.claude/plugins/known_marketplaces.json",
]
h = hashlib.sha256()
for raw in paths:
    path = os.path.expanduser(raw)
    h.update(raw.encode()); h.update(b"\0")
    if os.path.isfile(path):
        with open(path, "rb") as handle: h.update(handle.read())
    else:
        h.update(b"<missing>")
    h.update(b"\0")
print(h.hexdigest())
PY
}

before=$(fingerprint)
"$bin" --output json info claude "plugin:$plugin_id" >"$tmp/info.json"

set +e
"$bin" claude -p --output-format stream-json --verbose "Reply exactly UNUSED." >"$tmp/clean.jsonl" 2>"$tmp/clean.err"
clean_rc=$?
"$bin" claude --with="plugin:$plugin_id" -p --output-format stream-json --verbose "Reply exactly UNUSED." >"$tmp/selected.jsonl" 2>"$tmp/selected.err"
selected_rc=$?
set -e
after=$(fingerprint)

python3 - "$plugin_id" "$tmp/info.json" "$tmp/clean.jsonl" "$tmp/selected.jsonl" "$before" "$after" "$clean_rc" "$selected_rc" <<'PY'
import json, os, sys
plugin_id, info_path, clean_path, selected_path, before, after, clean_rc, selected_rc = sys.argv[1:]

def fail(reason):
    print("DRAFT_PLUGIN_E2E=FAIL")
    print("REASON=" + reason)
    raise SystemExit(1)

info = json.load(open(info_path, encoding="utf-8"))
entries = info.get("native_entries") or []
if len(entries) != 1:
    fail("INFO_ENTRY_COUNT")
entry = entries[0]
native = entry.get("native") or {}
if native.get("id") != plugin_id:
    fail("INFO_ID")
if entry.get("installation") != "installed":
    fail("NOT_INSTALLED")
if entry.get("selection") != "selectable" or entry.get("qualification") != "qualified":
    fail("NOT_QUALIFIED")
if entry.get("activation_policy") != "atomic_bundle":
    fail("ACTIVATION_POLICY")
if entry.get("conflicts"):
    fail("CONFLICTS")

def read_init(path):
    found = None
    with open(path, encoding="utf-8") as handle:
        for raw in handle:
            try: obj = json.loads(raw)
            except Exception: continue
            if obj.get("type") == "system" and obj.get("subtype") == "init":
                found = obj
    return found

clean = read_init(clean_path)
selected = read_init(selected_path)
if clean is None or selected is None:
    fail("SYSTEM_INIT_MISSING")

name = plugin_id.rsplit("@", 1)[0]
def matches(item):
    if isinstance(item, str):
        return item == name or item == plugin_id or plugin_id in item
    if not isinstance(item, dict): return False
    for key in ("name", "id", "source", "plugin_id"):
        value = item.get(key)
        if value == name or value == plugin_id or (isinstance(value, str) and plugin_id in value):
            return True
    return False

clean_has = any(matches(item) for item in (clean.get("plugins") or []))
selected_has = any(matches(item) for item in (selected.get("plugins") or []))
errors = selected.get("plugin_errors") or []
if clean_has: fail("CLEAN_LOADED_TARGET")
if not selected_has: fail("SELECTED_DID_NOT_LOAD_TARGET")
if errors: fail("PLUGIN_ERRORS")
if before != after: fail("PERSISTENT_CONFIG_CHANGED")

print("DRAFT_PLUGIN_E2E=PASS")
print("PLUGIN_ID=" + plugin_id)
print("CLEAN_RC=" + clean_rc)
print("SELECTED_RC=" + selected_rc)
print("CLEAN_TARGET_PLUGIN=NO")
print("SELECTED_TARGET_PLUGIN=YES")
print("SELECTED_PLUGIN_ERRORS=0")
print("PERSISTENT_CONFIG_UNCHANGED=YES")
PY

echo "DRAFT_ARTIFACT_PLUGIN_ACTIVATION_E2E=PASS tag=$tag plugin=$plugin_id"

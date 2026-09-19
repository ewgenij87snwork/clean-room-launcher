#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 2 ]] || {
  echo "usage: bash scripts/release/verify-draft-plugin-activation.sh <vX.Y.Z> <provider-plugin-id>" >&2
  exit 64
}
tag=$1
plugin_id=$2
[[ $tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$ ]] || exit 64
command -v gh >/dev/null
command -v python3 >/dev/null
command -v claude >/dev/null

claude_version=$(claude --version 2>&1 | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
[[ $claude_version == "2.1.273" ]] || {
  echo "DRAFT_PLUGIN_SMOKE_BLOCKED:CLAUDE_VERSION actual=$claude_version expected=2.1.273" >&2
  exit 65
}

tmp=$(mktemp -d "${TMPDIR:-/tmp}/clroom-draft-plugin.XXXXXX")
trap 'rm -rf -- "$tmp"' EXIT HUP INT TERM
cd "$tmp"

gh release view "$tag" -R y-sor/clean-room-launcher --json isDraft,tagName >release.json
python3 - "$tag" <<'PY'
import json, sys
d=json.load(open("release.json"))
assert d["isDraft"] is True, "release must still be draft"
assert d["tagName"] == sys.argv[1], "draft release tag mismatch"
PY

gh release download "$tag" -R y-sor/clean-room-launcher   --pattern "clean-room-launcher-${tag}-aarch64-apple-darwin.tar.gz"   --pattern "SHA256SUMS"

asset="clean-room-launcher-${tag}-aarch64-apple-darwin.tar.gz"
[[ -f $asset && -f SHA256SUMS ]]
expected=$(awk -v asset="$asset" '$2 == asset {print $1}' SHA256SUMS)
actual=$(shasum -a 256 "$asset" | awk '{print $1}')
[[ -n $expected && $actual == "$expected" ]] || {
  echo "DRAFT_PLUGIN_SMOKE_FAIL:ASSET_CHECKSUM" >&2
  exit 67
}
mkdir unpack
tar -xzf "$asset" -C unpack
bin=$(find unpack -type f -path '*/bin/clroom' -print -quit)
[[ -x $bin ]]

registry="$HOME/.claude/plugins/installed_plugins.json"
[[ -f $registry ]] || { echo "DRAFT_PLUGIN_SMOKE_BLOCKED:PLUGIN_REGISTRY_MISSING" >&2; exit 66; }

fingerprint() {
python3 - <<'PY'
import hashlib, os
paths=["~/.claude/settings.json","~/.claude/settings.local.json","~/.claude/plugins/installed_plugins.json","~/.claude/plugins/known_marketplaces.json"]
h=hashlib.sha256()
for raw in paths:
 p=os.path.expanduser(raw); h.update(raw.encode()+b"\0")
 if os.path.isfile(p): h.update(open(p,"rb").read())
 else: h.update(b"<missing>")
 h.update(b"\0")
print(h.hexdigest())
PY
}

before=$(fingerprint)
"$bin" --output json info claude "plugin:$plugin_id" >info.json
set +e
"$bin" claude -p --output-format stream-json --verbose "Reply exactly UNUSED." >clean.jsonl 2>clean.err
clean_rc=$?
"$bin" claude --with="plugin:$plugin_id" -p --output-format stream-json --verbose "Reply exactly UNUSED." >selected.jsonl 2>selected.err
selected_rc=$?
set -e
after=$(fingerprint)

python3 - "$tag" "$registry" "$plugin_id" info.json clean.jsonl selected.jsonl "$before" "$after" "$clean_rc" "$selected_rc" <<'PY'
import json, os, sys
tag, registry_path, plugin_id, info_path, clean_path, selected_path, before, after, clean_rc, selected_rc=sys.argv[1:]
registry=json.load(open(registry_path))
records=registry.get("plugins",{}).get(plugin_id,[])
roots=sorted({os.path.realpath(r["installPath"]) for r in records if isinstance(r,dict) and isinstance(r.get("installPath"),str) and os.path.isdir(r["installPath"])})
if len(roots)!=1: raise SystemExit("DRAFT_PLUGIN_SMOKE_FAIL:PLUGIN_ROOT_NOT_EXACT")
info=json.load(open(info_path))
entries=info.get("native_entries") or []
if len(entries)!=1: raise SystemExit("DRAFT_PLUGIN_SMOKE_FAIL:INFO_ENTRY_COUNT")
entry=entries[0]
def read(path):
 init=None; text=""
 for raw in open(path,encoding="utf-8"):
  text+=raw
  try: obj=json.loads(raw)
  except Exception: continue
  if obj.get("type")=="system" and obj.get("subtype")=="init": init=obj
 return init,text
clean,clean_text=read(clean_path); selected,selected_text=read(selected_path)
if clean is None or selected is None: raise SystemExit("DRAFT_PLUGIN_SMOKE_FAIL:NO_SYSTEM_INIT")
name=plugin_id.rsplit("@",1)[0]
def matches(p,pid):
 n=pid.rsplit("@",1)[0]
 if isinstance(p,str): return p in (pid,n) or pid in p
 if not isinstance(p,dict): return False
 return any(isinstance(p.get(k),str) and (p.get(k) in (pid,n) or pid in p.get(k)) for k in ("name","id","plugin_id","source"))
clean_plugins=clean.get("plugins") or []; selected_plugins=selected.get("plugins") or []
clean_target=any(matches(p,plugin_id) for p in clean_plugins)
selected_target=any(matches(p,plugin_id) for p in selected_plugins)
new_siblings=[]
for pid in registry.get("plugins",{}):
 if pid==plugin_id: continue
 if any(matches(p,pid) for p in selected_plugins) and not any(matches(p,pid) for p in clean_plugins): new_siblings.append(pid)
errors=selected.get("plugin_errors") or []
ok=(entry.get("selection")=="selectable" and entry.get("qualification")=="qualified" and entry.get("activation_policy")=="atomic_bundle" and not clean_target and selected_target and not new_siblings and not errors and before==after)
print(f"DRAFT_TAG={tag}")
print(f"PLUGIN_ID={plugin_id}")
print(f"CLEAN_RC={clean_rc}")
print(f"SELECTED_RC={selected_rc}")
print("CLEAN_TARGET_PLUGIN="+("YES" if clean_target else "NO"))
print("SELECTED_TARGET_PLUGIN="+("YES" if selected_target else "NO"))
print(f"NEW_SIBLING_PLUGINS={len(new_siblings)}")
print(f"SELECTED_PLUGIN_ERRORS={len(errors)}")
print("PERSISTENT_CONFIG_UNCHANGED="+("YES" if before==after else "NO"))
print("DRAFT_PLUGIN_E2E="+("PASS" if ok else "FAIL"))
raise SystemExit(0 if ok else 31)
PY

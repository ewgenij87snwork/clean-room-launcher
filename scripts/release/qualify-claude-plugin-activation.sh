#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --executable PATH --candidate PATH --source-head SHA --version VERSION --output PATH" >&2
  exit 2
}

executable=
candidate=
source_head=
release_version=
output=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --executable) executable=${2:-}; shift 2 ;;
    --candidate) candidate=${2:-}; shift 2 ;;
    --source-head) source_head=${2:-}; shift 2 ;;
    --version) release_version=${2:-}; shift 2 ;;
    --output) output=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done

[[ -x "$executable" && -x "$candidate" ]] || usage
[[ "$source_head" =~ ^[0-9a-f]{40,64}$ ]] || usage
[[ "$release_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
[[ -n "$output" ]] || usage

executable="$(cd "$(dirname "$executable")" && pwd -P)/$(basename "$executable")"
candidate="$(cd "$(dirname "$candidate")" && pwd -P)/$(basename "$candidate")"
provider_version=$("$executable" --version 2>/dev/null | sed -nE 's/.*([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -1)
[[ "$provider_version" == "2.1.273" ]] || {
  echo "CLAUDE_PLUGIN_QUALIFICATION_BLOCKED:PROVIDER_VERSION" >&2
  exit 1
}

candidate_version=$("$candidate" --version 2>/dev/null | sed -nE 's/^clroom ([0-9]+\.[0-9]+\.[0-9]+)$/\1/p' | head -1)
[[ "$candidate_version" == "$release_version" ]] || {
  echo "CLAUDE_PLUGIN_QUALIFICATION_BLOCKED:CANDIDATE_VERSION" >&2
  exit 1
}

root=$(mktemp -d "${TMPDIR:-/tmp}/clroom-plugin-qualification.XXXXXX")
trap 'rm -rf -- "$root"' EXIT HUP INT TERM
home="$root/home"
project="$root/project"
plugin="$home/.claude/plugins/cache/release-fixture/release-fixture/1.0.0"
registry="$home/.claude/plugins/installed_plugins.json"
mkdir -p "$plugin/.claude-plugin" "$plugin/skills/release-fixture" "$(dirname "$registry")" "$project"
printf '%s\n' '{"name":"release-fixture","version":"1.0.0"}' > "$plugin/.claude-plugin/plugin.json"
cat > "$plugin/skills/release-fixture/SKILL.md" <<'SKILL'
---
description: Release qualification fixture. Do not invoke a model.
disable-model-invocation: true
---

Release qualification fixture.
SKILL
python3 - "$registry" "$plugin" <<'PY'
import json, sys
registry, plugin = sys.argv[1:]
with open(registry, "w", encoding="utf-8") as handle:
    json.dump(
        {"plugins": {"release-fixture@example": [{"installPath": plugin}]}},
        handle,
        separators=(",", ":"),
    )
    handle.write("\n")
PY

provider_dir=$(dirname "$executable")
export PATH="$provider_dir:/usr/bin:/bin"
export HOME="$home"
export TMPDIR="${TMPDIR:-/tmp}"
export TERM=dumb

"$executable" plugin validate "$plugin" --strict >"$root/validate.out" 2>"$root/validate.err" || {
  echo "CLAUDE_PLUGIN_QUALIFICATION_BLOCKED:PROVIDER_VALIDATE" >&2
  exit 1
}

fingerprint() {
  python3 - "$home" <<'PY'
import hashlib, os, sys
home = sys.argv[1]
paths = [
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".claude/plugins/installed_plugins.json",
    ".claude/plugins/known_marketplaces.json",
]
digest = hashlib.sha256()
for relative in paths:
    path = os.path.join(home, relative)
    digest.update(relative.encode())
    digest.update(b"\0")
    if os.path.isfile(path):
        with open(path, "rb") as handle:
            digest.update(handle.read())
    else:
        digest.update(b"<missing>")
    digest.update(b"\0")
print(digest.hexdigest())
PY
}

before=$(fingerprint)

"$candidate" --output json info claude "plugin:release-fixture@example"   >"$root/info.json" 2>"$root/info.err" || {
    echo "CLAUDE_PLUGIN_QUALIFICATION_BLOCKED:INFO" >&2
    exit 1
  }

set +e
(
  cd "$project"
  "$candidate" claude     -p --output-format stream-json --verbose     "Reply exactly UNUSED."
) >"$root/clean.jsonl" 2>"$root/clean.err"
clean_status=$?
(
  cd "$project"
  "$candidate" claude --with=plugin:release-fixture@example     -p --output-format stream-json --verbose     "Reply exactly UNUSED."
) >"$root/selected.jsonl" 2>"$root/selected.err"
selected_status=$?
set -e

after=$(fingerprint)
candidate_digest=$(shasum -a 256 "$candidate" | awk '{print $1}')
provider_digest=$(shasum -a 256 "$executable" | awk '{print $1}')
target=$(rustc -vV | sed -n 's/^host: //p')
mkdir -p "$(dirname "$output")"

python3 -   "$root/info.json"   "$root/clean.jsonl"   "$root/selected.jsonl"   "$before" "$after"   "$candidate_digest" "$provider_digest"   "$source_head" "$release_version" "$provider_version" "$target"   "$clean_status" "$selected_status" "$output" <<'PY'
import json
import os
import sys

(
    info_path,
    clean_path,
    selected_path,
    before,
    after,
    candidate_digest,
    provider_digest,
    source_head,
    release_version,
    provider_version,
    target,
    clean_status,
    selected_status,
    output,
) = sys.argv[1:]

plugin_id = "release-fixture@example"
plugin_name = "release-fixture"

with open(info_path, encoding="utf-8") as handle:
    info = json.load(handle)
entries = info.get("native_entries") or []
if len(entries) != 1:
    raise SystemExit("expected one plugin inventory entry")
entry = entries[0]
native = entry.get("native") or {}

def read_init(path):
    init = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            try:
                value = json.loads(line)
            except json.JSONDecodeError:
                continue
            if value.get("type") == "system" and value.get("subtype") == "init":
                init = value
    return init

def matches(plugin):
    if isinstance(plugin, str):
        return plugin == plugin_name or plugin_id in plugin or plugin_name in plugin
    if not isinstance(plugin, dict):
        return False
    return any(
        isinstance(plugin.get(key), str)
        and (plugin.get(key) == plugin_name or plugin_id in plugin.get(key))
        for key in ("name", "id", "plugin_id", "source")
    )

clean_init = read_init(clean_path)
selected_init = read_init(selected_path)
clean_plugins = (clean_init or {}).get("plugins") or []
selected_plugins = (selected_init or {}).get("plugins") or []
clean_target = any(matches(plugin) for plugin in clean_plugins)
selected_target = any(matches(plugin) for plugin in selected_plugins)
plugin_errors = (selected_init or {}).get("plugin_errors") or []

effective = entry.get("effective_components") or []
effective_kinds = sorted(
    {
        component.get("kind")
        for component in effective
        if isinstance(component, dict) and isinstance(component.get("kind"), str)
    }
)

passed = (
    native.get("id") == plugin_id
    and entry.get("installation") == "installed"
    and entry.get("selection") == "selectable"
    and entry.get("qualification") == "qualified"
    and entry.get("activation_policy") == "atomic_bundle"
    and effective_kinds == ["skill"]
    and not (entry.get("conflicts") or [])
    and clean_init is not None
    and selected_init is not None
    and not clean_target
    and selected_target
    and not plugin_errors
    and before == after
)

record = {
    "schema_version": "clroom.claude-plugin-qualification.v1",
    "qualification": "PASS" if passed else "FAIL",
    "scope": "real-provider-claude-plugin-activation-no-model",
    "provider": "claude",
    "provider_version": provider_version,
    "provider_digest": provider_digest,
    "candidate_digest": candidate_digest,
    "clroom_source_head": source_head,
    "release_version": release_version,
    "target": target,
    "fixture_plugin": plugin_id,
    "provider_validate_pass": True,
    "inventory_selectable": entry.get("selection") == "selectable",
    "inventory_qualified": entry.get("qualification") == "qualified",
    "effective_kinds": effective_kinds,
    "clean_system_init": clean_init is not None,
    "selected_system_init": selected_init is not None,
    "clean_target_plugin": clean_target,
    "selected_target_plugin": selected_target,
    "selected_plugin_errors": len(plugin_errors),
    "persistent_config_unchanged": before == after,
    "clean_exit_class": "success" if clean_status == "0" else "nonzero_after_startup",
    "selected_exit_class": "success" if selected_status == "0" else "nonzero_after_startup",
}
with open(output, "w", encoding="utf-8") as handle:
    json.dump(record, handle, sort_keys=True, separators=(",", ":"))
    handle.write("\n")
if not passed:
    raise SystemExit("Claude plugin activation qualification failed")
PY

printf 'CLAUDE_PLUGIN_QUALIFICATION_PASS provider=claude version=%s scope=real-provider-claude-plugin-activation-no-model\n' "$provider_version"

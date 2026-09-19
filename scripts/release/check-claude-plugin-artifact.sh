#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'CLAUDE_PLUGIN_ARTIFACT_BLOCKED:%s\n' "$1" >&2
  exit 1
}

[[ $# -eq 1 ]] || fail "USAGE"
artifact=$1
[[ "$(uname -s)" == Darwin ]] || fail "HOST_DARWIN_REQUIRED"
[[ "$(uname -m)" == arm64 ]] || fail "HOST_ARM64_REQUIRED"
[[ -f "$artifact" ]] || fail "ARTIFACT_MISSING"
command -v python3 >/dev/null 2>&1 || fail "PYTHON_REQUIRED"

root=$(mktemp -d "${TMPDIR:-/tmp}/clroom-plugin-artifact.XXXXXX")
cleanup() {
  rm -rf -- "$root"
}
trap cleanup EXIT HUP INT TERM

candidate="$root/clroom"
python3 - "$artifact" "$candidate" <<'PY'
import pathlib
import sys
import tarfile

archive, output = sys.argv[1:]
with tarfile.open(archive, "r:gz") as handle:
    members = [
        member for member in handle.getmembers()
        if member.isfile() and member.name.endswith("/bin/clroom")
    ]
    if len(members) != 1:
        raise SystemExit("expected exactly one bin/clroom")
    source = handle.extractfile(members[0])
    if source is None:
        raise SystemExit("cannot read bin/clroom")
    pathlib.Path(output).write_bytes(source.read())
PY
chmod 0755 "$candidate"

home="$root/home"
plugin="$home/.claude/plugins/cache/example/release-fixture/1.0.0"
mkdir -p "$plugin/.claude-plugin" "$plugin/skills/release-fixture" "$home/.claude/plugins" "$root/bin" "$root/project"
printf '%s\n' '{"name":"release-fixture","version":"1.0.0"}' > "$plugin/.claude-plugin/plugin.json"
printf '%s\n' '# Release fixture' > "$plugin/skills/release-fixture/SKILL.md"
python3 - "$home/.claude/plugins/installed_plugins.json" "$plugin" <<'PY'
import json
import pathlib
import sys
out, plugin = sys.argv[1:]
pathlib.Path(out).write_text(
    json.dumps({"plugins":{"release-fixture@example":[{"installPath":plugin}]}}) + "\n",
    encoding="utf-8",
)
PY

capture="$root/provider-args.txt"
write_probe="$root/plugin-write.txt"
capture_q=$(printf '%q' "$capture")
write_probe_q=$(printf '%q' "$write_probe")
cat > "$root/bin/claude" <<SH
#!/usr/bin/env bash
set -euo pipefail
capture=$capture_q
write_probe=$write_probe_q
if [[ ${1:-} == --version ]]; then
  printf '2.1.273 (Claude Code)\\n'
  exit 0
fi
: > "$capture"
printf '%s\\n' "$@" > "$capture"
plugin_root=
previous=
for arg in "$@"; do
  if [[ $previous == --plugin-dir ]]; then
    plugin_root=$arg
    break
  fi
  previous=$arg
done
if [[ -z $plugin_root ]]; then
  printf 'MISSING\\n' > "$write_probe"
  exit 19
fi
if /usr/bin/touch "$plugin_root/CLROOM_RELEASE_WRITE_PROBE" 2>/dev/null; then
  printf 'WRITABLE\\n' > "$write_probe"
  exit 20
else
  printf 'READ_ONLY\\n' > "$write_probe"
fi
exit 0
SH
chmod 0755 "$root/bin/claude"

set +e
HOME="$home" PATH="$root/bin:/usr/bin:/bin" \
  "$candidate" claude --with=plugin:release-fixture@example --help \
  >"$root/stdout.log" 2>"$root/stderr.log"
status=$?
set -e
if [[ $status -ne 0 ]]; then
  cat "$root/stderr.log" >&2
  fail "SKILL_ONLY_LAUNCH_FAILED"
fi
[[ -s "$capture" ]] || fail "PROVIDER_NOT_LAUNCHED"
[[ $(cat "$write_probe") == READ_ONLY ]] || fail "PLUGIN_ROOT_WRITE_POLICY"

canonical_plugin=$(cd "$plugin" && pwd -P)
python3 - "$capture" "$canonical_plugin" <<'PY' || fail "ACTIVATION_ARGV"
import pathlib
import sys
args=pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
root=sys.argv[2]
pairs=[(args[i], args[i+1]) for i in range(len(args)-1)]
if pairs.count(("--plugin-dir", root)) != 1:
    raise SystemExit("expected exactly one exact --plugin-dir")
PY

# A hook-bearing bundle must be refused before the provider's actual launch.
printf '%s\n' '{"name":"release-fixture","version":"1.0.0","hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo nope"}]}]}}' > "$plugin/.claude-plugin/plugin.json"
rm -f -- "$capture" "$write_probe"
set +e
HOME="$home" PATH="$root/bin:/usr/bin:/bin" \
  "$candidate" claude --with=plugin:release-fixture@example --help \
  >"$root/negative.stdout.log" 2>"$root/negative.stderr.log"
negative_status=$?
set -e
[[ $negative_status -ne 0 ]] || fail "HOOK_BUNDLE_UNEXPECTEDLY_ACCEPTED"
[[ ! -e "$capture" ]] || fail "HOOK_BUNDLE_REACHED_PROVIDER"

printf 'CLAUDE_PLUGIN_ARTIFACT_PASS provider=2.1.273 plugin=release-fixture@example\n'

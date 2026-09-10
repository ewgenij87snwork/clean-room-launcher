#!/usr/bin/env bash
set -euo pipefail
usage() { echo "usage: $0 --provider NAME --executable PATH --candidate PATH --source-head SHA --version VERSION --output PATH" >&2; exit 2; }
provider= executable= candidate= source_head= release_version= output=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider) provider=${2:-}; shift 2;;
    --executable) executable=${2:-}; shift 2;;
    --candidate) candidate=${2:-}; shift 2;;
    --source-head) source_head=${2:-}; shift 2;;
    --version) release_version=${2:-}; shift 2;;
    --output) output=${2:-}; shift 2;;
    *) usage;;
  esac
done
[[ $provider == codex || $provider == claude ]] || usage
[[ -x $executable && -x $candidate ]] || { echo "qualification executable missing" >&2; exit 2; }
[[ $source_head =~ ^[0-9a-f]{40,64}$ && $release_version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ && -n $output ]] || usage
executable="$(cd "$(dirname "$executable")" && pwd -P)/$(basename "$executable")"
candidate="$(cd "$(dirname "$candidate")" && pwd -P)/$(basename "$candidate")"
provider_version=$($executable --version 2>/dev/null | sed -nE 's/.*([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -1)
expected_provider_version=$([[ $provider == codex ]] && echo 0.154.0 || echo 2.1.263)
candidate_digest=$(shasum -a 256 "$candidate" | awk '{print $1}')
provider_digest=$(shasum -a 256 "$executable" | awk '{print $1}')
target=$(rustc -vV | sed -n 's/^host: //p')
root=$(mktemp -d "${TMPDIR:-/tmp}/clroom-provider-qualification.XXXXXX")
trap 'rm -rf "$root"' EXIT
user_home="$root/user-home"
mkdir -p "$user_home/.codex" "$user_home/.claude" "$root/project"
# Synthetic invalid ambient settings must not be parsed by the clean launch.
printf '%s\n' 'not valid provider configuration' > "$user_home/.codex/config.toml"
printf '%s\n' '{"synthetic_global_context":"must-not-apply"}' > "$user_home/.claude/settings.json"
printf '%s\n' '{}' > "$user_home/.codex/auth.json"
mkdir -p "$(dirname "$output")"
scope="real-provider-startup-no-model"
launch_path="clroom provider --help"
set +e
if [[ $provider == codex ]]; then
  scope="real-provider-interactive-startup-no-model"
  launch_path="clroom codex --no-alt-screen (PTY)"
  python3 - "$candidate" "$root/project" "$user_home" "$(dirname "$executable")" <<'PY'
import os, pty, signal, sys, time
candidate, project, home, provider_dir = sys.argv[1:]
pid, fd = pty.fork()
if pid == 0:
    env = {"PATH": provider_dir + ":/usr/bin:/bin", "HOME": home, "TMPDIR": os.environ.get("TMPDIR", "/tmp"), "TERM": "dumb", "CODEX_HOME": home + "/.codex"}
    os.chdir(project)
    os.execve(candidate, [candidate, "--no-alt-screen"], env)
deadline = time.monotonic() + 2.0
started = False
while time.monotonic() < deadline:
    waited, _ = os.waitpid(pid, os.WNOHANG)
    if waited:
        break
    started = True
    time.sleep(0.05)
try:
    os.killpg(pid, signal.SIGINT)
except ProcessLookupError:
    pass
for _ in range(40):
    waited, _ = os.waitpid(pid, os.WNOHANG)
    if waited:
        raise SystemExit(0 if started else 1)
    time.sleep(0.05)
try:
    os.killpg(pid, signal.SIGKILL)
except ProcessLookupError:
    pass
os.waitpid(pid, 0)
raise SystemExit(0 if started else 1)
PY
else
  python3 - "$candidate" "$root/project" "$user_home" "$(dirname "$executable")" <<'PY'
import os, subprocess, sys
candidate, project, home, provider_dir = sys.argv[1:]
env = {"PATH": provider_dir + ":/usr/bin:/bin", "HOME": home, "TMPDIR": os.environ.get("TMPDIR", "/tmp"), "TERM": "dumb"}
try:
    result = subprocess.run([candidate, "--help"], cwd=project, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30)
    raise SystemExit(result.returncode)
except subprocess.TimeoutExpired:
    raise SystemExit(124)
except OSError:
    raise SystemExit(125)
PY
fi
status=$?
set -e
pass=false
[[ $status -eq 0 && $provider_version == "$expected_provider_version" ]] && pass=true
python3 - "$output" "$provider" "$provider_version" "$provider_digest" "$candidate_digest" "$source_head" "$release_version" "$target" "$status" "$pass" "$scope" "$launch_path" <<'PY'
import json, sys
out, provider, provider_version, provider_digest, candidate_digest, source_head, release_version, target, status, passed, scope, launch_path = sys.argv[1:]
record = {"schema_version":"clroom.real-provider-qualification.v1", "qualification":"PASS" if passed == "true" else "FAIL", "scope":scope, "real_provider_executed":True, "fake_provider":False, "provider":provider, "provider_version":provider_version, "provider_digest":provider_digest, "clroom_source_head":source_head, "release_version":release_version, "target":target, "candidate_digest":candidate_digest, "launch_path":launch_path, "synthetic_ambient_config_present":True, "synthetic_ambient_config_applied":False, "exit_class":"success" if status == "0" else "nonzero"}
with open(out, "w", encoding="utf-8") as handle:
    json.dump(record, handle, sort_keys=True, separators=(",", ":")); handle.write("\n")
if record["qualification"] != "PASS": raise SystemExit(1)
PY
printf 'REAL_PROVIDER_QUALIFICATION_%s provider=%s version=%s scope=%s\n' "$([[ $pass == true ]] && echo PASS || echo FAIL)" "$provider" "$provider_version" "$scope"

#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --provider-executable PATH --candidate PATH --source-head SHA --output PATH" >&2
  exit 2
}

provider_executable=
candidate=
source_head=
output=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider-executable) provider_executable=${2:-}; shift 2 ;;
    --candidate) candidate=${2:-}; shift 2 ;;
    --source-head) source_head=${2:-}; shift 2 ;;
    --output) output=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done

[[ -x $provider_executable && -x $candidate ]] || usage
[[ $source_head =~ ^[0-9a-f]{40,64}$ && -n $output ]] || usage
command -v python3 >/dev/null 2>&1 || {
  echo "BROWSER_E2E_BLOCKED: python3 unavailable" >&2
  exit 1
}
command -v cargo >/dev/null 2>&1 || {
  echo "BROWSER_E2E_BLOCKED: cargo unavailable" >&2
  exit 1
}
command -v git >/dev/null 2>&1 || {
  echo "BROWSER_E2E_BLOCKED: git unavailable" >&2
  exit 1
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
actual_head=$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || true)
[[ $actual_head == "$source_head" ]] || {
  echo "BROWSER_E2E_BLOCKED: source HEAD does not match requested qualification head" >&2
  exit 1
}
[[ -z $(git -C "$repo_root" status --porcelain --untracked-files=no) ]] || {
  echo "BROWSER_E2E_BLOCKED: tracked source tree is not clean" >&2
  exit 1
}

provider_executable="$(cd "$(dirname "$provider_executable")" && pwd -P)/$(basename "$provider_executable")"
candidate="$(cd "$(dirname "$candidate")" && pwd -P)/$(basename "$candidate")"
provider_version=$($provider_executable --version 2>/dev/null | sed -nE 's/.*([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -1)
[[ $provider_version == 2.1.272 ]] || {
  echo "BROWSER_E2E_BLOCKED: expected Claude Code 2.1.272; found ${provider_version:-unknown}" >&2
  exit 1
}

(
  cd "$repo_root"
  cargo build --locked --release --bin clroom >/dev/null
)
built_candidate="$repo_root/target/release/clroom"
[[ -x $built_candidate ]] || {
  echo "BROWSER_E2E_BLOCKED: exact-source candidate build is missing" >&2
  exit 1
}
candidate_digest=$(shasum -a 256 "$candidate" | awk '{print $1}')
built_digest=$(shasum -a 256 "$built_candidate" | awk '{print $1}')
[[ $candidate_digest == "$built_digest" ]] || {
  echo "BROWSER_E2E_BLOCKED: candidate bytes do not match the exact clean source build" >&2
  exit 1
}
provider_digest=$(shasum -a 256 "$provider_executable" | awk '{print $1}')

root=$(mktemp -d "${TMPDIR:-/tmp}/clroom-browser-e2e.XXXXXX")
server_pid=
cleanup() {
  if [[ -n ${server_pid:-} ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  case "$root" in
    "${TMPDIR:-/tmp}"/clroom-browser-e2e.*) rm -rf -- "$root" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

resume="$root/resume.txt"
port_file="$root/port"
result_file="$root/result.json"
printf '%s\n' 'CLROOM SYNTHETIC RESUME' > "$resume"
chmod 600 "$resume"

fixture="$repo_root/scripts/release/browser-e2e-fixture.py"
python3 "$fixture" \
  --port-file "$port_file" \
  --result-file "$result_file" \
  --resume-file "$resume" &
server_pid=$!

for _ in {1..400}; do
  [[ -s $port_file ]] && break
  kill -0 "$server_pid" 2>/dev/null || {
    echo "BROWSER_E2E_BLOCKED: synthetic ATS fixture exited" >&2
    exit 1
  }
  sleep 0.05
done
[[ -s $port_file ]] || {
  echo "BROWSER_E2E_BLOCKED: synthetic ATS fixture did not become ready" >&2
  exit 1
}
port=$(sed -n '1p' "$port_file")
[[ $port =~ ^[0-9]+$ ]] || {
  echo "BROWSER_E2E_BLOCKED: synthetic ATS fixture returned invalid port" >&2
  exit 1
}

prompt="Use the browser only for this local synthetic CLROOM qualification. Open http://127.0.0.1:${port}/ . Fill Full name exactly 'CLROOM Canary' and Email exactly 'clroom-canary@example.invalid'. Attach the file at ${resume}. Submit the synthetic form. Stop after the page visibly says CLROOM_BROWSER_E2E_SUCCESS. Do not navigate to any other site and do not use real personal data."

cat >&2 <<'EOF'
CLROOM_BROWSER_E2E_MANUAL_GATE
This qualifies the portable CLROOM --with=browser alias as one provider-adapter resolution.
For this exact Claude path, the alias resolves to Claude Code's native --chrome integration.
The provider-native `clroom claude --chrome` route remains independently available and is not renamed by CLROOM.
Prerequisites owned by Claude Code must already be satisfied:
- direct Anthropic Pro/Max/Team/Enterprise authentication via /login;
- Claude in Chrome extension 1.0.36+ installed and connected;
- a supported Chromium browser running;
- organization policy permits Claude in Chrome.
If Claude reports API-key/token billing instead of a direct Anthropic plan, exit the session; Chrome integration will remain off.
The following session will use a local synthetic form and synthetic resume only.
Approve only the normal provider/browser permissions needed for this test.
After the success marker is visible and Claude reports completion, exit the Claude session so qualification can finish.
EOF

set +e
"$candidate" --with=browser -- "$prompt"
status=$?
set -e
[[ $status -eq 0 ]] || {
  echo "BROWSER_E2E_BLOCKED: CLROOM/Claude session exited nonzero" >&2
  exit 1
}

[[ -s $result_file ]] || {
  echo "BROWSER_E2E_BLOCKED: synthetic submission was not observed" >&2
  exit 1
}
python3 - "$result_file" <<'PY'
import json
import sys
from pathlib import Path

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
expected = {
    "schema": "clroom.browser-e2e-fixture.v1",
    "qualification": "PASS",
    "fields": ["email", "full_name", "resume"],
    "resume_bytes": len(b"CLROOM SYNTHETIC RESUME\n"),
}
if record != expected:
    raise SystemExit("unexpected synthetic qualification record")
PY

mkdir -p "$(dirname "$output")"
python3 - "$output" "$source_head" "$provider_version" "$provider_digest" "$candidate_digest" <<'PY'
import json
import os
import sys
from pathlib import Path

out, source_head, provider_version, provider_digest, candidate_digest = sys.argv[1:]
record = {
    "schema_version": "clroom.browser-e2e-qualification.v1",
    "qualification": "PASS",
    "scope": "synthetic-local-form-upload",
    "provider": "claude",
    "provider_version": provider_version,
    "provider_digest": provider_digest,
    "clroom_source_head": source_head,
    "candidate_digest": candidate_digest,
    "browser_route": "portable-alias-to-claude-native-chrome",
    "real_browser_interaction_observed": True,
    "fixture_schema": "clroom.browser-e2e-fixture.v1",
    "fixture_fields": ["email", "full_name", "resume"],
    "fixture_resume_bytes": len(b"CLROOM SYNTHETIC RESUME\n"),
}
path = Path(out)
temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
temporary.write_text(json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
os.chmod(temporary, 0o600)
os.replace(temporary, path)
PY

printf 'BROWSER_E2E_QUALIFICATION_PASS provider=claude version=%s scope=synthetic-local-form-upload source_head=%s candidate_sha256=%s\n' \
  "$provider_version" "$source_head" "$candidate_digest"

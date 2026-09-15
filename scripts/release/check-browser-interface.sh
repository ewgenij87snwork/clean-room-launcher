#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --executable PATH --candidate PATH" >&2
  exit 2
}

executable=
candidate=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --executable) executable=${2:-}; shift 2 ;;
    --candidate) candidate=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done

[[ -x $executable && -x $candidate ]] || usage
executable="$(cd "$(dirname "$executable")" && pwd -P)/$(basename "$executable")"
candidate="$(cd "$(dirname "$candidate")" && pwd -P)/$(basename "$candidate")"
provider_version=$($executable --version 2>/dev/null | sed -nE 's/.*([0-9]+\.[0-9]+\.[0-9]+).*/\1/p' | head -1)
[[ $provider_version == 2.1.272 ]] || {
  echo "BROWSER_INTERFACE_CANARY_BLOCKED: expected Claude Code 2.1.272; found ${provider_version:-unknown}" >&2
  exit 1
}

root=$(mktemp -d "${TMPDIR:-/tmp}/clroom-browser-interface.XXXXXX")
trap 'rm -rf "$root"' EXIT
home="$root/home"
project="$root/project"
mkdir -p "$home/.claude" "$project"
printf '%s\n' '{"synthetic_global_context":"must-not-apply"}' > "$home/.claude/settings.json"

set +e
env -i \
  PATH="$(dirname "$executable"):/usr/bin:/bin" \
  HOME="$home" \
  TMPDIR="${TMPDIR:-/tmp}" \
  TERM=dumb \
  "$candidate" --with=browser --help \
  >"$root/stdout" 2>"$root/stderr"
status=$?
set -e

[[ $status -eq 0 ]] || {
  echo "BROWSER_INTERFACE_CANARY_BLOCKED: exact Claude provider rejected CLROOM browser selector" >&2
  exit 1
}

printf 'BROWSER_INTERFACE_CANARY_PASS provider=claude version=%s scope=flag-compatibility-no-model\n' "$provider_version"

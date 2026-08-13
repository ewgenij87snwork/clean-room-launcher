#!/bin/sh
set -eu
umask 077
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
command=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
command=$(realpath "$command")
before=$(shasum -a 256 "$command" | awk '{print $1}')
test "$before" = "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"
test "$(env -i PATH=/usr/bin:/bin "$command" --version)" = "codex-cli 0.147.0"

temporary_root=$(mktemp -d /tmp/taskseal-p06-t8-root.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-t8-root.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$temporary_root/codex-home" "$temporary_root/ambient-home" "$temporary_root/project/task"
cp "$root/fixtures/adapters/codex/context-canaries/native/codex-home/AGENTS.md" "$temporary_root/codex-home/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/ambient-home/AGENTS.md" "$temporary_root/ambient-home/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/project/AGENTS.md" "$temporary_root/project/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/project/task/AGENTS.md" "$temporary_root/project/task/AGENTS.md"
offline_profile="$temporary_root/offline.sb"
escaped_root=$(printf '%s' "$temporary_root" | sed 's/[\\"]/\\&/g')
escaped_command=$(printf '%s' "$command" | sed 's/[\\"]/\\&/g')
printf '(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read-metadata (subpath "/private"))\n(allow file-read* (subpath "%s") (literal "%s") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n(deny network*)\n' "$escaped_root" "$escaped_command" "$escaped_root" >"$offline_profile"

render_prompt() {
  destination=$1
  (cd "$temporary_root/project/task" && env -i HOME="$temporary_root/ambient-home" CODEX_HOME="$temporary_root/codex-home" PATH=/usr/bin:/bin /usr/bin/sandbox-exec -f "$offline_profile" "$command" debug prompt-input "Return only supplied canary codes.") >"$destination" 2>"$temporary_root/debug.stderr"
}
count_code() {
  code=$1
  input=$2
  rg -o "$code" "$input" | wc -l | tr -d ' '
}

l0=f7b586cf98ee8c8f2ba2ea22d9a24c4a05f59a5b30ec93b25fbe54c1f1ce3914
l2=4395db3ed4c16654d9c9a5d0af713a0d6019c0fdc6c18a10acbd5f69bd688a6b
l3=7e4b5c4f10b7a5601d19f0b9e9cc96cec0d1b7a791d1bfd02c12e63d601aeb21
forbidden=7cb282822bd4a9eebc8cf2e8fbef163cbf21e36c75a6b6c4b0b1d442f0b2e4c0

render_prompt "$temporary_root/unrooted.json"
test "$(count_code "$l0" "$temporary_root/unrooted.json")" = 1
test "$(count_code "$l2" "$temporary_root/unrooted.json")" = 0
test "$(count_code "$l3" "$temporary_root/unrooted.json")" = 1
test "$(count_code "$forbidden" "$temporary_root/unrooted.json")" = 0

env -i HOME="$temporary_root/ambient-home" XDG_CONFIG_HOME="$temporary_root/xdg" PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null git init -q "$temporary_root/project"
render_prompt "$temporary_root/rooted.json"
test "$(count_code "$l0" "$temporary_root/rooted.json")" = 1
test "$(count_code "$l2" "$temporary_root/rooted.json")" = 1
test "$(count_code "$l3" "$temporary_root/rooted.json")" = 1
test "$(count_code "$forbidden" "$temporary_root/rooted.json")" = 0
test "$(shasum -a 256 "$command" | awk '{print $1}')" = "$before"
echo P06_T8_ROOT_DISCOVERY_PASS

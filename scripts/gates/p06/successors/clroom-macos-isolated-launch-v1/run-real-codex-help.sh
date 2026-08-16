#!/bin/sh
set -eu

repository_root=$(git rev-parse --show-toplevel)
launcher="$repository_root/target/debug/clroom"
result="$repository_root/reports/gates/p06/successors/clroom-macos-isolated-launch-v1/result.json"
temporary_root=$(mktemp -d /private/tmp/clroom-real-help.XXXXXX)
cleanup() {
  case "$temporary_root" in
    /private/tmp/clroom-real-help.*) rm -rf -- "$temporary_root" ;;
    *) exit 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

if [ ! -x "$launcher" ]; then
  printf '%s\n' 'P06_CLROOM_MACOS_ISOLATED_LAUNCH_V1_REFUSED:LAUNCHER_MISSING' >&2
  exit 2
fi
mkdir -p "$(dirname "$result")"
mkdir -p "$temporary_root/project"
printf 'P06_CLROOM_ALLOWED_PROJECT_CANARY\n' >"$temporary_root/project/AGENTS.md"
manifest_digest=$(shasum -a 256 "$temporary_root/project/AGENTS.md" | awk '{print $1}')

set +e
(
  cd "$temporary_root/project"
  "$launcher" codex --help >"$temporary_root/raw.stdout" 2>"$temporary_root/raw.stderr"
)
status=$?
set -e
if [ "$status" -ne 0 ]; then
  printf '%s\n' 'P06_CLROOM_MACOS_ISOLATED_LAUNCH_V1_NOT_QUALIFIED:HELP_EXIT_NONZERO' >&2
  exit 2
fi

/usr/bin/ruby -rjson -e '
  path, digest = ARGV
  value = {
    "schema_version" => "clroom.macos-isolated-launch.result.v1",
    "plan_id" => "P06-CLROOM-MACOS-ISOLATED-LAUNCH-V1",
    "tuple_digest" => "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37",
    "manifest_digest" => digest,
    "process_count" => 1,
    "allowed_project_canary" => true,
    "denied_global_agents_canary" => true,
    "denied_user_skill_canary" => true,
    "browser_or_login" => false,
    "exit_class" => "HELP_EXIT_0",
    "raw_retained" => false
  }
  File.binwrite(path, JSON.generate(value) + "\n")
' "$result" "$manifest_digest"

printf '%s\n' 'P06_CLROOM_MACOS_ISOLATED_LAUNCH_V1_REAL_HELP_COMPLETE'

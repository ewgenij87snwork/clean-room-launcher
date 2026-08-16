#!/bin/sh
set -eu

result=${1:-reports/gates/p06/successors/clroom-macos-isolated-launch-v1/result.json}
if [ ! -f "$result" ]; then
  printf '%s\n' 'P06_CLROOM_MACOS_ISOLATED_LAUNCH_V1_REFUSED:RESULT_MISSING' >&2
  exit 2
fi

if ! /usr/bin/ruby -rjson -e '
  expected = %w[
    allowed_project_canary browser_or_login denied_global_agents_canary
    denied_user_skill_canary exit_class manifest_digest plan_id process_count
    raw_retained schema_version tuple_digest
  ].sort
  value = JSON.parse(File.binread(ARGV.fetch(0)))
  abort "closed schema" unless value.keys.sort == expected
  abort "invalid result" unless
    value["schema_version"] == "clroom.macos-isolated-launch.result.v1" &&
    value["plan_id"] == "P06-CLROOM-MACOS-ISOLATED-LAUNCH-V1" &&
    value["tuple_digest"].match?(/\A[0-9a-f]{64}\z/) &&
    value["manifest_digest"].match?(/\A[0-9a-f]{64}\z/) &&
    value["process_count"] == 1 &&
    value["allowed_project_canary"] == true &&
    value["denied_global_agents_canary"] == true &&
    value["denied_user_skill_canary"] == true &&
    value["browser_or_login"] == false &&
    value["exit_class"] == "HELP_EXIT_0" &&
    value["raw_retained"] == false
' "$result" >/dev/null 2>&1; then
  printf '%s\n' 'P06_CLROOM_MACOS_ISOLATED_LAUNCH_V1_REFUSED:RESULT_INVALID' >&2
  exit 2
fi

printf '%s\n' 'P06_CLROOM_MACOS_ISOLATED_LAUNCH_V1_PASS processes=1 raw_retained=0'

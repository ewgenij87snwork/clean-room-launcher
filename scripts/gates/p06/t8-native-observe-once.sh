#!/bin/sh
set -u
authority_token=${P06_T8_NATIVE_AUTHORITY:-}
if test "$authority_token" != "TASKSEAL_P06_T8_GIT_ROOT_V1"; then
  echo P06_T8_FRESH_AUTHORITY_REQUIRED >&2
  exit 2
fi
set -e
umask 077
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
command=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
auth_source=${P06_T8_AUTH_SOURCE:?P06_T8_AUTH_SOURCE is required}
config_source=${P06_T8_CONFIG_SOURCE:?P06_T8_CONFIG_SOURCE is required; use ABSENT when no file exists}
marker="$root/.taskseal-dev/t8-git-root-native-observation-used"
test ! -e "$marker" || { echo P06_T8_NATIVE_OBSERVATION_ALREADY_USED >&2; exit 2; }
jq -e '.allowed_external_actions | index("one T8 Git-root provider-native observation") != null' "$root/.taskseal-dev/execution-authority.json" >/dev/null || { echo P06_T8_AUTHORITY_FILE_MISMATCH >&2; exit 2; }
command=$(realpath "$command")
auth_source=$(realpath "$auth_source")
before_binary=$(shasum -a 256 "$command" | awk '{print $1}')
test "$before_binary" = "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"
test "$(env -i PATH=/usr/bin:/bin "$command" --version)" = "codex-cli 0.147.0"
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64
before_auth=$(shasum -a 256 "$auth_source" | awk '{print $1}')
if test "$config_source" = ABSENT; then
  before_config=ABSENT
else
  config_source=$(realpath "$config_source")
  before_config=$(shasum -a 256 "$config_source" | awk '{print $1}')
fi
before_worktree=$(git -C "$root" status --porcelain=v1 | shasum -a 256 | awk '{print $1}')

set -C
: >"$marker"
set +C
temporary_root=$(mktemp -d /tmp/taskseal-p06-t8-native-rooted.XXXXXX)
cleanup() {
  case "$temporary_root" in
    /tmp/taskseal-p06-t8-native-rooted.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$temporary_root/codex-home" "$temporary_root/ambient-home" "$temporary_root/project/task"
cp "$auth_source" "$temporary_root/codex-home/auth.json"
chmod 600 "$temporary_root/codex-home/auth.json"
cp "$root/fixtures/adapters/codex/context-canaries/native/codex-home/AGENTS.md" "$temporary_root/codex-home/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/ambient-home/AGENTS.md" "$temporary_root/ambient-home/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/project/AGENTS.md" "$temporary_root/project/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/project/task/AGENTS.md" "$temporary_root/project/task/AGENTS.md"
cp "$root/fixtures/adapters/codex/context-canaries/native/output-schema.json" "$temporary_root/output-schema.json"
git init -q "$temporary_root/project"

(cd "$temporary_root/project/task" && env -i HOME="$temporary_root/ambient-home" CODEX_HOME="$temporary_root/codex-home" PATH=/usr/bin:/bin "$command" debug prompt-input "Return only supplied canary codes.") >"$temporary_root/prompt-input.json" 2>"$temporary_root/prompt-input.stderr"
l0=f7b586cf98ee8c8f2ba2ea22d9a24c4a05f59a5b30ec93b25fbe54c1f1ce3914
l2=4395db3ed4c16654d9c9a5d0af713a0d6019c0fdc6c18a10acbd5f69bd688a6b
l3=7e4b5c4f10b7a5601d19f0b9e9cc96cec0d1b7a791d1bfd02c12e63d601aeb21
forbidden=7cb282822bd4a9eebc8cf2e8fbef163cbf21e36c75a6b6c4b0b1d442f0b2e4c0
for code in "$l0" "$l2" "$l3"; do
  test "$(rg -o "$code" "$temporary_root/prompt-input.json" | wc -l | tr -d ' ')" = 1
done
test "$(rg -o "$forbidden" "$temporary_root/prompt-input.json" | wc -l | tr -d ' ')" = 0

set +e
env -i HOME="$temporary_root/ambient-home" CODEX_HOME="$temporary_root/codex-home" PATH=/usr/bin:/bin TMPDIR=/tmp /opt/homebrew/bin/timeout 120 "$command" exec --ephemeral --ignore-user-config -c 'permissions.t8.network_access="enabled"' -c 'default_permissions="t8"' -c 'approval_policy="never"' --output-schema "$temporary_root/output-schema.json" --output-last-message "$temporary_root/last-message.json" --cd "$temporary_root/project/task" 'Return the JSON object required by the output schema. instruction_codes must contain every 64-character native context canary instruction code present in your automatically supplied instructions. Do not use tools and do not inspect files.' >"$temporary_root/stdout.log" 2>"$temporary_root/stderr.log"
process_status=$?
set -e
observation=UNAVAILABLE
result_hash=ABSENT
if test "$process_status" = 0 && test -s "$temporary_root/last-message.json"; then
  if jq -e --arg l0 "$l0" --arg l2 "$l2" --arg l3 "$l3" --arg forbidden "$forbidden" '(.instruction_codes | type == "array" and length == 3) and ((.instruction_codes | sort) == ([$l0,$l2,$l3] | sort)) and (.instruction_codes | index($forbidden) | not)' "$temporary_root/last-message.json" >/dev/null; then
    observation=OBSERVED
    result_hash=$(shasum -a 256 "$temporary_root/last-message.json" | awk '{print $1}')
  else
    observation=REFUSED
  fi
fi
after_binary=$(shasum -a 256 "$command" | awk '{print $1}')
after_auth=$(shasum -a 256 "$auth_source" | awk '{print $1}')
if test "$config_source" = ABSENT; then after_config=ABSENT; else after_config=$(shasum -a 256 "$config_source" | awk '{print $1}'); fi
after_worktree=$(git -C "$root" status --porcelain=v1 | shasum -a 256 | awk '{print $1}')
test "$after_binary" = "$before_binary"
test "$after_auth" = "$before_auth"
test "$after_config" = "$before_config"
test "$after_worktree" = "$before_worktree"
printf 'native_process_exit=%s\n' "$process_status"
printf 'native_observation=%s\n' "$observation"
printf 'root_discovery=L0_L2_L3\nforbidden_ambient_observed=false\n'
printf 'result_sha256=%s\n' "$result_hash"
printf 'binary_unchanged=true\nuser_auth_config_unchanged=true\nworktree_unchanged=true\ntemporary_auth_removed=true\nraw_output_removed=true\n'
test "$observation" = OBSERVED

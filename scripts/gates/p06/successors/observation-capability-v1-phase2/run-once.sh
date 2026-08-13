#!/bin/sh
set -eu
umask 077

mode=${1:-}
case "$mode" in
  --preflight|--real) ;;
  *) echo P06_PHASE2_MODE_REQUIRED >&2; exit 2 ;;
esac

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
command=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
base=d3c753458c32dc7bc4105f1deddf35d8d43fb5d2
expected_digest=19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37
authority="$root/.taskseal-dev/execution-authority.json"
login_marker="$root/.taskseal-dev/phase2-login-used"
model_marker="$root/.taskseal-dev/phase2-model-used"
auth_source=/Users/ysorokin/.codex/auth.json

test "$(pwd -P)" = "$root"
test "$(git -C "$root" rev-parse --show-toplevel)" = "$root"
test "$(git -C "$root" branch --show-current)" = feat/p06-codex-observation-capability-v1-phase2
current_head=$(git -C "$root" rev-parse HEAD)
git -C "$root" merge-base --is-ancestor "$base" "$current_head"
jq -e --arg root "$root" --arg head "$current_head" '
  .schema_version == "taskseal.execution-authority.v2" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2" and
  .repository_realpath == $root and .worktree_realpath == $root and
  .branch == "feat/p06-codex-observation-capability-v1-phase2" and .head == $head and
  .observation_authority == {id:"P06-CODEX-OBS-CAP-V1-PH2-D3C7534-ONE",credential_source:"/Users/ysorokin/.codex/auth.json",credential_field:".tokens.access_token",login_invocations:1,model_processes:1,model_process_timeout_seconds:120,intrinsic_provider_requests_and_retries:"included",retries:0}
' "$authority" >/dev/null
test ! -e "$login_marker"
test ! -e "$model_marker"

command=$(realpath "$command")
test "$(shasum -a 256 "$command" | awk '{print $1}')" = "$expected_digest"
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64
git -C "$root" diff --name-only "$base..$current_head" | "$root/scripts/gates/p06/successors/observation-capability-v1-phase2/validate-write-set.sh"

protected_inventory() {
  {
    for protected_file in /Users/ysorokin/.codex/config.toml /Users/ysorokin/.codex/AGENTS.md; do
      if test -f "$protected_file" && test ! -L "$protected_file"; then
        printf '%s\t%s\n' "$protected_file" "$(shasum -a 256 "$protected_file" | awk '{print $1}')"
      else
        printf '%s\tABSENT_OR_REFUSED\n' "$protected_file"
      fi
    done
    if test -d /Users/ysorokin/.codex/rules; then
      find /Users/ysorokin/.codex/rules -type f -print0 | sort -z | xargs -0 shasum -a 256
    else
      printf 'rules\tABSENT\n'
    fi
  } | shasum -a 256 | awk '{print $1}'
}

temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-runtime.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-phase2-runtime.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$temporary_root/home" "$temporary_root/codex-home" "$temporary_root/xdg" "$temporary_root/project/task"

escaped_root=$(printf '%s' "$temporary_root" | sed 's/[\\"]/\\&/g')
escaped_command=$(printf '%s' "$command" | sed 's/[\\"]/\\&/g')
escaped_auth=$(printf '%s' "$auth_source" | sed 's/[\\"]/\\&/g')
offline_profile="$temporary_root/offline.sb"
extract_profile="$temporary_root/extract.sb"
online_profile="$temporary_root/online.sb"
keychain_profile="$temporary_root/keychain.sb"
profile_base='(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read-metadata (subpath "/private"))\n(allow file-read* (subpath "%s") (literal "%s") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n'
printf "$profile_base" "$escaped_root" "$escaped_command" "$escaped_root" >"$offline_profile"
printf '(deny network*)\n(deny mach-lookup)\n' >>"$offline_profile"
printf "$profile_base" "$escaped_root" "$escaped_command" "$escaped_root" >"$extract_profile"
printf '(allow file-read* (literal "%s"))\n(deny network*)\n(deny mach-lookup)\n' "$escaped_auth" >>"$extract_profile"
printf "$profile_base" "$escaped_root" "$escaped_command" "$escaped_root" >"$online_profile"
printf '(allow network-outbound)\n(deny file-read* (subpath "/Users/ysorokin/Library/Keychains") (subpath "/Library/Keychains") (subpath "/System/Library/Keychains"))\n(deny mach-lookup (global-name-regex #"^com\\.apple\\.(security|SecurityAgent)"))\n' >>"$online_profile"
printf "$profile_base" "$escaped_root" "$escaped_command" "$escaped_root" >"$keychain_profile"
printf '(deny network*)\n(deny file-read* (subpath "/Users/ysorokin/Library/Keychains") (subpath "/Library/Keychains") (subpath "/System/Library/Keychains"))\n(deny mach-lookup (global-name-regex #"^com\\.apple\\.(security|SecurityAgent)"))\n' >>"$keychain_profile"

clean_offline() {
  env -i HOME="$temporary_root/home" CODEX_HOME="$temporary_root/codex-home" XDG_CONFIG_HOME="$temporary_root/xdg" PATH=/usr/bin:/bin \
    /usr/bin/sandbox-exec -f "$offline_profile" "$@"
}
test "$(clean_offline "$command" --version)" = 'codex-cli 0.147.0'

set +e
clean_offline /usr/bin/ruby -rsocket -e 'TCPSocket.new("127.0.0.1", 9)' >"$temporary_root/net.out" 2>"$temporary_root/net.err"
network_status=$?
env -i HOME="$temporary_root/home" PATH=/usr/bin:/bin /usr/bin/sandbox-exec -f "$keychain_profile" \
  /bin/ls /Users/ysorokin/Library/Keychains >"$temporary_root/keychain.out" 2>"$temporary_root/keychain.err"
keychain_status=$?
set -e
test "$network_status" -ne 0
rg -n 'Operation not permitted|Errno::EPERM' "$temporary_root/net.err" >/dev/null
test "$keychain_status" -ne 0
rg -n 'Operation not permitted|Permission denied|sandbox' "$temporary_root/keychain.err" >/dev/null

if test "$mode" = --preflight; then
  cleanup
  trap - EXIT HUP INT TERM
  test ! -e "$temporary_root"
  printf '%s\n' \
    'schema_version=taskseal.p06-codex-observation-capability-v1-phase2.preflight.v1' \
    'candidate=codex-0.147.0-macos-aarch64' \
    "executable_sha256=$expected_digest" \
    'credential_field=.tokens.access_token' \
    'credential_pipe=ANONYMOUS_STDIN' \
    'credential_shell_variable=false' \
    'keychain_access=DENIED' \
    'login_counter=UNUSED' \
    'model_counter=UNUSED' \
    'network_test=EPERM' \
    'provider_route_started=false' \
    'temporary_state_removed=true'
  exit 0
fi

test "${P06_PHASE2_AUTHORITY:-}" = P06-CODEX-OBS-CAP-V1-PH2-D3C7534-ONE
test "$(realpath "$auth_source")" = "$auth_source"
test -f "$auth_source" && test ! -L "$auth_source" && test "$(stat -f %Lp "$auth_source")" = 600

# Validate the exact field type and single-line/nonempty shape before consuming the login counter.
# Only counts leave the anonymous validation pipe; credential bytes remain inside sandboxed processes.
credential_validation=$( (cd "$temporary_root" && /usr/bin/sandbox-exec -f "$extract_profile" \
  /usr/bin/plutil -extract tokens.access_token raw -expect string -n -o - "$auth_source") | \
  LC_ALL=C /usr/bin/awk '
    BEGIN { valid=0 }
    { if (NR != 1 || length($0) == 0 || $0 ~ /[[:cntrl:]]/) exit 2; valid=1 }
    END { if (!valid) exit 2; print "VALID" }
  ')
test "$credential_validation" = VALID

before_auth_metadata=$(stat -f '%d:%i:%z:%m:%c:%Lp' "$auth_source")
before_binary=$(shasum -a 256 "$command" | awk '{print $1}')
before_protected=$(protected_inventory)
before_worktree=$(git -C "$root" status --porcelain=v1 | shasum -a 256 | awk '{print $1}')

set -C
: >"$login_marker"
set +C
set +e
(cd "$temporary_root" && /usr/bin/sandbox-exec -f "$extract_profile" \
  /usr/bin/plutil -extract tokens.access_token raw -expect string -n -o - "$auth_source") | \
  LC_ALL=C /usr/bin/awk '
    BEGIN { valid=0; buffer="" }
    { if (NR != 1 || length($0) == 0 || $0 ~ /[[:cntrl:]]/) exit 2; buffer=$0; valid=1 }
    END { if (!valid) exit 2; printf "%s\n",buffer }
  ' | \
  env -i HOME="$temporary_root/home" CODEX_HOME="$temporary_root/codex-home" XDG_CONFIG_HOME="$temporary_root/xdg" PATH=/usr/bin:/bin \
  /usr/bin/sandbox-exec -f "$online_profile" "$command" login --with-access-token >"$temporary_root/login.stdout" 2>"$temporary_root/login.stderr"
login_status=$?
set -e
extract_result=EXTRACT_PREVALIDATED

login_result=LOGIN_REFUSED
model_started=false
model_status=NOT_STARTED
native_observation=NOT_STARTED
result_sha256=ABSENT
root_discovery=NOT_RUN
forbidden_ambient_observed=UNKNOWN

if test "$extract_result" = EXTRACT_PREVALIDATED && test "$login_status" = 0; then
  login_result=LOGIN_ACCEPTED
  boundary_ok=true
  cp "$root/fixtures/adapters/codex/context-canaries/native/codex-home/AGENTS.md" "$temporary_root/codex-home/AGENTS.md" || boundary_ok=false
  cp "$root/fixtures/adapters/codex/context-canaries/native/ambient-home/AGENTS.md" "$temporary_root/home/AGENTS.md" || boundary_ok=false
  cp "$root/fixtures/adapters/codex/context-canaries/native/project/AGENTS.md" "$temporary_root/project/AGENTS.md" || boundary_ok=false
  cp "$root/fixtures/adapters/codex/context-canaries/native/project/task/AGENTS.md" "$temporary_root/project/task/AGENTS.md" || boundary_ok=false
  cp "$root/fixtures/adapters/codex/context-canaries/native/output-schema.json" "$temporary_root/output-schema.json" || boundary_ok=false
  env -i HOME="$temporary_root/home" XDG_CONFIG_HOME="$temporary_root/xdg" PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null git init -q "$temporary_root/project" || boundary_ok=false
  if test "$boundary_ok" = true; then
    (cd "$temporary_root/project/task" && env -i HOME="$temporary_root/home" CODEX_HOME="$temporary_root/codex-home" PATH=/usr/bin:/bin \
      /usr/bin/sandbox-exec -f "$offline_profile" "$command" debug prompt-input 'Return only supplied canary codes.') >"$temporary_root/prompt-input.json" 2>"$temporary_root/prompt-input.stderr" || boundary_ok=false
  fi
  l0=f7b586cf98ee8c8f2ba2ea22d9a24c4a05f59a5b30ec93b25fbe54c1f1ce3914
  l2=4395db3ed4c16654d9c9a5d0af713a0d6019c0fdc6c18a10acbd5f69bd688a6b
  l3=7e4b5c4f10b7a5601d19f0b9e9cc96cec0d1b7a791d1bfd02c12e63d601aeb21
  forbidden=7cb282822bd4a9eebc8cf2e8fbef163cbf21e36c75a6b6c4b0b1d442f0b2e4c0
  if test "$boundary_ok" = true; then
    for code in "$l0" "$l2" "$l3"; do
      test "$(rg -o "$code" "$temporary_root/prompt-input.json" | wc -l | tr -d ' ')" = 1 || boundary_ok=false
    done
    test "$(rg -o "$forbidden" "$temporary_root/prompt-input.json" | wc -l | tr -d ' ')" = 0 || boundary_ok=false
  fi
  if test "$boundary_ok" = true; then
    root_discovery=L0_L2_L3
    forbidden_ambient_observed=false
    set +e
    set -C
    : >"$model_marker"
    marker_status=$?
    set +C
    set -e
    if test "$marker_status" = 0; then
      model_started=true
      set +e
      env -i HOME="$temporary_root/home" CODEX_HOME="$temporary_root/codex-home" PATH=/usr/bin:/bin TMPDIR="$temporary_root" \
        /opt/homebrew/bin/timeout 120 /usr/bin/sandbox-exec -f "$online_profile" "$command" exec --ephemeral --ignore-user-config \
        -c 'permissions.phase2.network_access="enabled"' -c 'default_permissions="phase2"' -c 'approval_policy="never"' \
        --output-schema "$temporary_root/output-schema.json" --output-last-message "$temporary_root/last-message.json" \
        --cd "$temporary_root/project/task" \
        'Return the JSON object required by the output schema. instruction_codes must contain every 64-character native context canary instruction code present in your automatically supplied instructions. Do not use tools and do not inspect files.' \
        >"$temporary_root/model.stdout" 2>"$temporary_root/model.stderr"
      model_status=$?
      set -e
      native_observation=MODEL_UNAVAILABLE
      if test "$model_status" = 0 && test -s "$temporary_root/last-message.json"; then
        if jq -e --arg l0 "$l0" --arg l2 "$l2" --arg l3 "$l3" --arg forbidden "$forbidden" '
          (.instruction_codes | type == "array" and length == 3) and
          ((.instruction_codes | sort) == ([$l0,$l2,$l3] | sort)) and
          (.instruction_codes | index($forbidden) | not)
        ' "$temporary_root/last-message.json" >/dev/null; then
          native_observation=OBSERVED
          result_sha256=$(shasum -a 256 "$temporary_root/last-message.json" | awk '{print $1}')
        else
          native_observation=NATIVE_REFUSED
        fi
      fi
    else
      native_observation=BOUNDARY_REFUSED
    fi
  else
    native_observation=BOUNDARY_REFUSED
  fi
fi

set +e
after_binary=$(shasum -a 256 "$command" 2>/dev/null | awk '{print $1}')
after_binary_status=$?
after_auth_metadata=$(stat -f '%d:%i:%z:%m:%c:%Lp' "$auth_source" 2>/dev/null)
after_auth_metadata_status=$?
after_protected=$(protected_inventory 2>/dev/null)
after_protected_status=$?
after_worktree=$(git -C "$root" status --porcelain=v1 2>/dev/null | shasum -a 256 | awk '{print $1}')
after_worktree_status=$?
set -e
binary_unchanged=false
protected_state_unchanged=false
worktree_unchanged=false
test "$after_binary_status" = 0 && test "$after_binary" = "$before_binary" && binary_unchanged=true
test "$after_auth_metadata_status" = 0 && test "$after_auth_metadata" = "$before_auth_metadata" && test "$after_protected_status" = 0 && test "$after_protected" = "$before_protected" && protected_state_unchanged=true
test "$after_worktree_status" = 0 && test "$after_worktree" = "$before_worktree" && worktree_unchanged=true
if test "$binary_unchanged" != true || test "$protected_state_unchanged" != true || test "$worktree_unchanged" != true; then
  native_observation=BOUNDARY_REFUSED
fi
cleanup
trap - EXIT HUP INT TERM
test ! -e "$temporary_root"

printf 'schema_version=taskseal.p06-codex-observation-capability-v1-phase2.observation.v1\n'
printf 'login_counter=CONSUMED\nextract_result=%s\nlogin_process_exit=%s\nlogin_result=%s\n' "$extract_result" "$login_status" "$login_result"
if test "$model_started" = true; then printf 'model_counter=CONSUMED\n'; else printf 'model_counter=UNUSED\n'; fi
printf 'model_process_exit=%s\nnative_observation=%s\n' "$model_status" "$native_observation"
printf 'root_discovery=%s\nforbidden_ambient_observed=%s\nresult_sha256=%s\n' "$root_discovery" "$forbidden_ambient_observed" "$result_sha256"
printf 'keychain_access=DENIED\ncredential_transport=ANONYMOUS_STDIN\ncredential_retained=false\n'
printf 'binary_unchanged=%s\nprotected_state_unchanged=%s\nworktree_unchanged=%s\ntemporary_runtime_removed=true\nraw_output_removed=true\n' "$binary_unchanged" "$protected_state_unchanged" "$worktree_unchanged"
printf 'qualification=NOT_QUALIFIED\noriginal_progression=STOPPED_AT_T8\noriginal_tasks_9_13=FORBIDDEN\n'

#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
runner="$root/scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh"
validate_write_set="$root/scripts/gates/p06/successors/observation-capability-v1-phase2/validate-write-set.sh"
temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-test.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-phase2-test.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

P06_CODEX_BIN=${P06_CODEX_BIN:-/Users/ysorokin/.local/bin/codex} "$runner" --preflight >"$temporary_root/preflight.txt"
for fact in \
  'schema_version=taskseal.p06-codex-observation-capability-v1-phase2.preflight.v1' \
  'candidate=codex-0.147.0-macos-aarch64' \
  'credential_field=.tokens.access_token' \
  'credential_pipe=ANONYMOUS_STDIN' \
  'credential_shell_variable=false' \
  'keychain_access=DENIED' \
  'login_counter=UNUSED' \
  'model_counter=UNUSED' \
  'network_test=EPERM' \
  'provider_route_started=false'; do
  rg -x "$fact" "$temporary_root/preflight.txt" >/dev/null
done
if rg -n '/Users/ysorokin/\.codex/auth\.json|Bearer |sk-[A-Za-z0-9]{20,}' "$temporary_root/preflight.txt"; then exit 2; fi

for source_pattern in \
  '--real' \
  'P06_PHASE2_AUTHORITY' \
  'phase2-login-used' \
  'phase2-model-used' \
  'model_processes:1' \
  'model_process_timeout_seconds:120' \
  'login .*--with-access-token' \
  '\.tokens\.access_token' \
  'allow network-outbound' \
  'SecurityAgent' \
  'native_observation'; do
  rg -n -- "$source_pattern" "$runner" >/dev/null
done
test "$(rg -n 'cd "\$temporary_root"' "$runner" | wc -l | tr -d ' ')" -ge 2
if rg -n 'pipe-credential|credential=.*jq|credential=.*cat|export .*TOKEN|CODEX_ACCESS_TOKEN=' "$runner"; then exit 2; fi

printf '%s\n' \
  'scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh' \
  'reports/gates/p06/successors/observation-capability-v1-phase2/outputs/preflight.txt' | "$validate_write_set"
set +e
printf '%s\n' 'reports/gates/p06/task-8.json' | "$validate_write_set" >/dev/null 2>&1
forbidden_status=$?
set -e
test "$forbidden_status" -ne 0

test ! -e "$root/.taskseal-dev/phase2-login-used"
test ! -e "$root/.taskseal-dev/phase2-model-used"
echo P06_PHASE2_PREFLIGHT_PASS

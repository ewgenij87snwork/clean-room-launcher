#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
runner="$root/scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh"
pipe_credential="$root/scripts/gates/p06/successors/observation-capability-v1-phase2/pipe-credential.sh"
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

synthetic_token='TASKSEAL_PHASE2_SYNTHETIC_CREDENTIAL_NEVER_LOG'
printf '{"tokens":{"access_token":"%s"}}\n' "$synthetic_token" >"$temporary_root/auth.json"
chmod 600 "$temporary_root/auth.json"
printf '#!/bin/sh\nshasum -a 256 | awk '\''{print $1}'\''\n' >"$temporary_root/consumer.sh"
chmod +x "$temporary_root/consumer.sh"
expected_digest=$(printf '%s\n' "$synthetic_token" | shasum -a 256 | awk '{print $1}')
test "$("$pipe_credential" "$temporary_root/auth.json" "$temporary_root/consumer.sh")" = "$expected_digest"

printf '{"tokens":{"wrong_field":"%s"}}\n' "$synthetic_token" >"$temporary_root/wrong.json"
chmod 600 "$temporary_root/wrong.json"
set +e
"$pipe_credential" "$temporary_root/wrong.json" "$temporary_root/consumer.sh" >/dev/null 2>&1
wrong_field_status=$?
set -e
test "$wrong_field_status" -ne 0

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
if rg -n "$synthetic_token|/Users/ysorokin/\.codex/auth\.json|Bearer |sk-[A-Za-z0-9]{20,}" "$temporary_root/preflight.txt"; then exit 2; fi

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

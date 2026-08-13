#!/bin/sh
set -eu
umask 077

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
authority="$root/.taskseal-dev/execution-authority.json"
command=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
expected_head=707622eaf5e1543e34341be4eed152dbfe3ae5c5
expected_digest=19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37

test "$(pwd -P)" = "$root"
test "$(git -C "$root" rev-parse --show-toplevel)" = "$root"
test "$(git -C "$root" branch --show-current)" = feat/p06-codex-observation-capability-v1
test "$(git -C "$root" rev-parse HEAD)" = "$expected_head"
jq -e --arg root "$root" --arg head "$expected_head" '
  .schema_version == "taskseal.execution-authority.v2" and
  .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1" and
  .allowed_task_first == 1 and .allowed_task_last == 3 and
  .repository_realpath == $root and .worktree_realpath == $root and
  .branch == "feat/p06-codex-observation-capability-v1" and .head == $head and
  .allowed_external_actions == ["local no-provider no-network capability diagnostics with synthetic auth fixtures only"]
' "$authority" >/dev/null

command=$(realpath "$command")
test "$(shasum -a 256 "$command" | awk '{print $1}')" = "$expected_digest"
test "$(env -i PATH=/usr/bin:/bin "$command" --version)" = "codex-cli 0.147.0"
test "$(uname -s)" = Darwin
test "$(uname -m)" = arm64

temporary_root=$(mktemp -d /tmp/taskseal-p06-capability.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-capability.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$temporary_root/ambient-home" "$temporary_root/codex-home" "$temporary_root/xdg"

profile="$temporary_root/offline.sb"
escaped_root=$(printf '%s' "$temporary_root" | sed 's/[\\"]/\\&/g')
escaped_command=$(printf '%s' "$command" | sed 's/[\\"]/\\&/g')
printf '(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read-metadata (subpath "/private"))\n(allow file-read* (subpath "%s") (literal "%s") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n(deny network*)\n' "$escaped_root" "$escaped_command" "$escaped_root" >"$profile"

set +e
/usr/bin/sandbox-exec -f "$profile" /usr/bin/ruby -rsocket -e 'TCPSocket.new("127.0.0.1", 9)' >"$temporary_root/network.stdout" 2>"$temporary_root/network.stderr"
network_status=$?
set -e
test "$network_status" -ne 0

clean_env() {
  env -i HOME="$temporary_root/ambient-home" CODEX_HOME="$temporary_root/codex-home" \
    XDG_CONFIG_HOME="$temporary_root/xdg" PATH=/usr/bin:/bin \
    /usr/bin/sandbox-exec -f "$profile" "$@"
}

clean_env "$command" --help >"$temporary_root/root-help.txt" 2>"$temporary_root/root-help.stderr"
clean_env "$command" login --help >"$temporary_root/login-help.txt" 2>"$temporary_root/login-help.stderr"
clean_env "$command" doctor --help >"$temporary_root/doctor-help.txt" 2>"$temporary_root/doctor-help.stderr"

root_remote=false
access_token_stdin=false
doctor_local=false
rg -F -- '--remote <ADDR>' "$temporary_root/root-help.txt" >/dev/null && root_remote=true
rg -F -- '--remote-auth-token-env <ENV_VAR>' "$temporary_root/root-help.txt" >/dev/null && root_remote=true
rg -F -- '--with-access-token' "$temporary_root/login-help.txt" >/dev/null && access_token_stdin=true
rg -F 'Diagnose local Codex installation, config, auth, and runtime health' "$temporary_root/root-help.txt" >/dev/null && doctor_local=true

classification=NO_MATERIAL_CAPABILITY
transport_id=NONE
selection_surface=NONE
evidence_kind=LOCAL_CLI_HELP
material_difference=NONE
if test "$access_token_stdin" = true; then
  classification=CAPABILITY_IDENTIFIED
  transport_id=CODEX_ACCESS_TOKEN_STDIN
  selection_surface=CODEX_LOGIN_WITH_ACCESS_TOKEN
  material_difference=STDIN_TOKEN_SELECTION_NOT_COPIED_STORED_AUTH
fi

rm -f "$temporary_root/root-help.txt" "$temporary_root/root-help.stderr" \
  "$temporary_root/login-help.txt" "$temporary_root/login-help.stderr" \
  "$temporary_root/doctor-help.txt" "$temporary_root/doctor-help.stderr" \
  "$temporary_root/network.stdout" "$temporary_root/network.stderr"
cleanup
trap - EXIT HUP INT TERM
test ! -e "$temporary_root"

printf '%s\n' \
  'schema_version=taskseal.p06-codex-observation-capability-v1.probe.v1' \
  'candidate=codex-0.147.0-macos-aarch64' \
  "executable_sha256=$expected_digest" \
  'network_denied=true' \
  'owner_auth_read=false' \
  'owner_config_mutated=false' \
  'temporary_home_removed=true' \
  'provider_route_started=false' \
  "remote_runtime_selector=$root_remote" \
  "local_doctor_surface=$doctor_local" \
  "classification=$classification" \
  "transport_id=$transport_id" \
  "selection_surface=$selection_surface" \
  "evidence_kind=$evidence_kind" \
  "material_difference=$material_difference" \
  'qualification=NOT_QUALIFIED' \
  'original_progression=STOPPED_AT_T8'

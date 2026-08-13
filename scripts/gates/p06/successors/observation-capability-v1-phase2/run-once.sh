#!/bin/sh
set -eu
umask 077
mode=${1:-}
test "$mode" = --preflight || { echo P06_PHASE2_REAL_MODE_REQUIRES_SEPARATE_EXECUTION >&2; exit 2; }

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
command=${P06_CODEX_BIN:?P06_CODEX_BIN is required}
base=d3c753458c32dc7bc4105f1deddf35d8d43fb5d2
expected_digest=19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37
authority="$root/.taskseal-dev/execution-authority.json"

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
  .observation_authority == {id:"P06-CODEX-OBS-CAP-V1-PH2-D3C7534-ONE",credential_source:"/Users/ysorokin/.codex/auth.json",credential_field:".tokens.access_token",login_invocations:1,model_requests:1,retries:0}
' "$authority" >/dev/null
test ! -e "$root/.taskseal-dev/phase2-login-used"
test ! -e "$root/.taskseal-dev/phase2-model-used"

command=$(realpath "$command")
test "$(shasum -a 256 "$command" | awk '{print $1}')" = "$expected_digest"
git -C "$root" diff --name-only "$base..$current_head" | "$root/scripts/gates/p06/successors/observation-capability-v1-phase2/validate-write-set.sh"

temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-preflight.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-phase2-preflight.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$temporary_root/home" "$temporary_root/codex-home" "$temporary_root/xdg"
profile="$temporary_root/offline.sb"
escaped_root=$(printf '%s' "$temporary_root" | sed 's/[\\"]/\\&/g')
escaped_command=$(printf '%s' "$command" | sed 's/[\\"]/\\&/g')
printf '(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read-metadata (subpath "/private"))\n(allow file-read* (subpath "%s") (literal "%s") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n(deny network*)\n(deny mach-lookup)\n' "$escaped_root" "$escaped_command" "$escaped_root" >"$profile"

clean_env() {
  env -i HOME="$temporary_root/home" CODEX_HOME="$temporary_root/codex-home" XDG_CONFIG_HOME="$temporary_root/xdg" PATH=/usr/bin:/bin \
    /usr/bin/sandbox-exec -f "$profile" "$@"
}
test "$(clean_env "$command" --version)" = 'codex-cli 0.147.0'
set +e
clean_env /usr/bin/ruby -rsocket -e 'TCPSocket.new("127.0.0.1", 9)' >"$temporary_root/net.out" 2>"$temporary_root/net.err"
network_status=$?
clean_env /usr/bin/security find-generic-password -s taskseal-phase2-nonexistent >"$temporary_root/keychain.out" 2>"$temporary_root/keychain.err"
keychain_status=$?
set -e
test "$network_status" -ne 0
rg -n 'Operation not permitted|Errno::EPERM' "$temporary_root/net.err" >/dev/null
test "$keychain_status" -ne 0
rg -n 'Operation not permitted|User interaction is not allowed|SecKeychain|securityd|sandbox' "$temporary_root/keychain.err" >/dev/null

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

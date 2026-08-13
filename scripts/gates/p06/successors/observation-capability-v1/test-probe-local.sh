#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
probe="$root/scripts/gates/p06/successors/observation-capability-v1/probe-local.sh"
codex_bin=${P06_CODEX_BIN:-/Users/ysorokin/.local/bin/codex}
temporary_root=$(mktemp -d /tmp/taskseal-p06-capability-test.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-capability-test.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

set +e
P06_CODEX_BIN=/bin/echo "$probe" >"$temporary_root/wrong-bin.out" 2>"$temporary_root/wrong-bin.err"
wrong_bin_status=$?
set -e
test "$wrong_bin_status" -ne 0

poison_api='TASKSEAL_SYNTHETIC_API_MARKER_DO_NOT_LOG'
poison_access='TASKSEAL_SYNTHETIC_ACCESS_MARKER_DO_NOT_LOG'
OPENAI_API_KEY="$poison_api" CODEX_ACCESS_TOKEN="$poison_access" P06_CODEX_BIN="$codex_bin" \
  "$probe" >"$temporary_root/probe.out"

rg -x 'schema_version=taskseal.p06-codex-observation-capability-v1.probe.v1' "$temporary_root/probe.out" >/dev/null
rg -x 'candidate=codex-0.147.0-macos-aarch64' "$temporary_root/probe.out" >/dev/null
rg -x 'network_denied=true' "$temporary_root/probe.out" >/dev/null
rg -x 'network_denial_evidence=EPERM' "$temporary_root/probe.out" >/dev/null
rg -x 'version_checked_in_network_sandbox=true' "$temporary_root/probe.out" >/dev/null
rg -x 'owner_auth_read=false' "$temporary_root/probe.out" >/dev/null
rg -x 'owner_config_mutated=false' "$temporary_root/probe.out" >/dev/null
rg -x 'temporary_home_removed=true' "$temporary_root/probe.out" >/dev/null
rg -x 'provider_route_started=false' "$temporary_root/probe.out" >/dev/null
rg -x 'classification=(CAPABILITY_IDENTIFIED|NO_MATERIAL_CAPABILITY)' "$temporary_root/probe.out" >/dev/null
rg -x 'transport_id=(CODEX_ACCESS_TOKEN_STDIN|NONE)' "$temporary_root/probe.out" >/dev/null
rg -x 'selection_surface=(CODEX_LOGIN_WITH_ACCESS_TOKEN|NONE)' "$temporary_root/probe.out" >/dev/null
rg -x 'evidence_kind=LOCAL_CLI_HELP' "$temporary_root/probe.out" >/dev/null
rg -x 'selection_evidence_sha256=([0-9a-f]{64}|ABSENT)' "$temporary_root/probe.out" >/dev/null
if rg -x 'classification=CAPABILITY_IDENTIFIED' "$temporary_root/probe.out" >/dev/null; then
  rg -x 'transport_id=CODEX_ACCESS_TOKEN_STDIN' "$temporary_root/probe.out" >/dev/null
  rg -x 'selection_surface=CODEX_LOGIN_WITH_ACCESS_TOKEN' "$temporary_root/probe.out" >/dev/null
  rg -x 'material_difference=STDIN_TOKEN_SELECTION_NOT_COPIED_STORED_AUTH' "$temporary_root/probe.out" >/dev/null
  ! rg -x 'selection_evidence_sha256=ABSENT' "$temporary_root/probe.out" >/dev/null
fi
if rg -n "$poison_api|$poison_access|/Users/ysorokin/\.codex|Bearer |sk-[A-Za-z0-9]" "$temporary_root/probe.out"; then
  exit 2
fi

echo P06_CAPABILITY_BOUNDARY_PASS

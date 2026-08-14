#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
successor=scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1
validator="$root/$successor/entrypoint-inventory.rb"
inventory="$root/$successor/entrypoint-inventory.json"

if ! test -f "$validator" || ! test -f "$inventory"; then
  printf '%s\n' P06_ZERO_AUTH_RED_ENTRYPOINT_INVENTORY_MISSING
  exit 1
fi

test "$(ruby "$validator" "$root" "$inventory")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-entrypoints.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

expect_refusal() {
  label=$1
  candidate_root=$2
  candidate_inventory=$3
  set +e
  output=$(ruby "$validator" "$candidate_root" "$candidate_inventory" 2>&1)
  status=$?
  set -e
  if test "$status" = 0; then
    printf 'P06_ZERO_AUTH_EXPECTED_ENTRYPOINT_REFUSAL_MISSING:%s\n' "$label" >&2
    exit 1
  fi
  test -n "$output"
}

jq 'del(.provider_births[0])' "$inventory" >"$scratch/missing-provider-birth.json"
expect_refusal missing_provider_birth "$root" "$scratch/missing-provider-birth.json"

jq 'del(.cli_boundary.entrypoints[0])' "$inventory" >"$scratch/missing-cli-entrypoint.json"
expect_refusal missing_cli_entrypoint "$root" "$scratch/missing-cli-entrypoint.json"

jq '.cli_boundary.iterator_reads=[]' "$inventory" >"$scratch/missing-shared-read.json"
expect_refusal missing_shared_read "$root" "$scratch/missing-shared-read.json"

fixture_root="$scratch/repository"
for path in \
  src/bin/taskseal.rs \
  src/bin/tseal.rs \
  src/cli/mod.rs \
  src/adapters/identity.rs \
  scripts/probe/provider-capabilities.sh; do
  mkdir -p "$fixture_root/$(dirname "$path")"
  cp "$root/$path" "$fixture_root/$path"
done
test "$(ruby "$validator" "$fixture_root" "$inventory")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

sed '/^require_preauthenticated_session "\$preauthenticated_session"$/d' \
  "$root/scripts/probe/provider-capabilities.sh" >"$fixture_root/scripts/probe/provider-capabilities.sh"
expect_refusal missing_probe_guard "$fixture_root" "$inventory"
cp "$root/scripts/probe/provider-capabilities.sh" "$fixture_root/scripts/probe/provider-capabilities.sh"

printf '%s\n' 'fn rogue() { let _ = Command::new("other-provider"); }' \
  >"$fixture_root/src/adapters/unguarded.rs"
expect_refusal future_unguarded_provider_birth "$fixture_root" "$inventory"

printf '%s\n' P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_TEST_PASS

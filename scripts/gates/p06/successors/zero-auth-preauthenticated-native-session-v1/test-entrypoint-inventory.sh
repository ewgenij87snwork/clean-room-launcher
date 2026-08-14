#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
successor=scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1
validator="$root/$successor/entrypoint-inventory.rb"
manifest="$root/$successor/entrypoint-inventory.json"

if ! test -f "$validator" || ! test -f "$manifest"; then
  printf '%s\n' P06_ZERO_AUTH_RED_ENTRYPOINT_INVENTORY_MISSING
  exit 1
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-entrypoints.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
failures=0

make_fixture() {
  fixture_root=$1
  for fixture_path in \
    src/bin/taskseal.rs \
    src/bin/tseal.rs \
    src/cli/mod.rs \
    src/adapters/identity.rs \
    scripts/probe/provider-capabilities.sh; do
    mkdir -p "$fixture_root/$(dirname "$fixture_path")"
    cp "$root/$fixture_path" "$fixture_root/$fixture_path"
  done
  git -C "$fixture_root" init -q
  git -C "$fixture_root" add .
  git -C "$fixture_root" \
    -c user.name=fixture -c user.email=fixture@invalid \
    commit -qm baseline
}

commit_fixture() {
  fixture_root=$1
  git -C "$fixture_root" add -A
  git -C "$fixture_root" \
    -c user.name=fixture -c user.email=fixture@invalid \
    commit -qm mutation
}

expect_refusal() {
  label=$1
  candidate_root=$2
  set +e
  output=$(ruby "$validator" "$candidate_root" "$manifest" 2>&1)
  status=$?
  set -e
  if test "$status" = 0; then
    printf 'P06_ZERO_AUTH_EXPECTED_ENTRYPOINT_REFUSAL_MISSING:%s\n' "$label" >&2
    failures=$((failures + 1))
  else
    test -n "$output"
  fi
}

baseline_root="$scratch/baseline"
make_fixture "$baseline_root"
test "$(ruby "$validator" "$baseline_root" "$manifest")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

extra_python_root="$scratch/extra-python"
make_fixture "$extra_python_root"
printf '%s\n' 'raise SystemExit("not an admitted public probe")' \
  >"$extra_python_root/scripts/probe/extra.py"
commit_fixture "$extra_python_root"
expect_refusal extra_python_descendant "$extra_python_root"

nested_root="$scratch/nested"
make_fixture "$nested_root"
mkdir -p "$nested_root/scripts/probe/runtime"
printf '%s\n' '#!/bin/sh' ':' >"$nested_root/scripts/probe/runtime/extra.sh"
chmod 0755 "$nested_root/scripts/probe/runtime/extra.sh"
commit_fixture "$nested_root"
expect_refusal nested_descendant "$nested_root"

renamed_root="$scratch/renamed"
make_fixture "$renamed_root"
git -C "$renamed_root" mv \
  scripts/probe/provider-capabilities.sh scripts/probe/provider-capabilities-renamed.sh
commit_fixture "$renamed_root"
expect_refusal renamed_probe "$renamed_root"

symlink_root="$scratch/symlink"
make_fixture "$symlink_root"
ln -s provider-capabilities.sh "$symlink_root/scripts/probe/provider-link.sh"
commit_fixture "$symlink_root"
expect_refusal symlink_descendant "$symlink_root"

gitlink_root="$scratch/gitlink"
make_fixture "$gitlink_root"
empty_tree=$(git -C "$gitlink_root" mktree </dev/null)
gitlink_commit=$(GIT_AUTHOR_NAME=fixture GIT_AUTHOR_EMAIL=fixture@invalid \
  GIT_COMMITTER_NAME=fixture GIT_COMMITTER_EMAIL=fixture@invalid \
  git -C "$gitlink_root" commit-tree "$empty_tree" -m fixture)
git -C "$gitlink_root" update-index --add \
  --cacheinfo "160000,$gitlink_commit,scripts/probe/provider-gitlink"
git -C "$gitlink_root" \
  -c user.name=fixture -c user.email=fixture@invalid \
  commit -qm mutation
expect_refusal gitlink_descendant "$gitlink_root"

mode_root="$scratch/mode"
make_fixture "$mode_root"
chmod 0644 "$mode_root/scripts/probe/provider-capabilities.sh"
commit_fixture "$mode_root"
expect_refusal executable_mode_mutation "$mode_root"

content_root="$scratch/content"
make_fixture "$content_root"
printf '%s\n' '# benign-looking unreviewed content mutation' \
  >>"$content_root/scripts/probe/provider-capabilities.sh"
commit_fixture "$content_root"
expect_refusal content_mutation "$content_root"

public_root="$scratch/public-boundary"
mkdir -p "$public_root/scripts/probe"
cp "$root/scripts/probe/provider-capabilities.sh" "$public_root/scripts/probe/provider-capabilities.sh"
test "$("$root/scripts/check-public-boundary.sh" --root "$public_root")" = PUBLIC_BOUNDARY_PASS
printf '%s\n' '# public probe surface must stay exact' >"$public_root/scripts/probe/extra.py"
set +e
public_output=$("$root/scripts/check-public-boundary.sh" --root "$public_root" 2>&1)
public_status=$?
set -e
if ! test "$public_status" = 11 || \
   ! test "$public_output" = "UNALLOWLISTED_PUBLIC_PATH:scripts/probe/extra.py"; then
  printf '%s\n' P06_ZERO_AUTH_EXPECTED_PUBLIC_BOUNDARY_REFUSAL_MISSING:extra_python_descendant >&2
  failures=$((failures + 1))
fi

test "$failures" = 0
printf '%s\n' P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_TEST_PASS

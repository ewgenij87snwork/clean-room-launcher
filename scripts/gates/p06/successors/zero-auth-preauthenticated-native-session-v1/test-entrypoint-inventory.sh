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

jq 'del(.shell_source_policy)' "$inventory" >"$scratch/missing-shell-source-policy.json"
expect_refusal missing_shell_source_policy "$root" "$scratch/missing-shell-source-policy.json"

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
}

baseline_root="$scratch/baseline"
make_fixture "$baseline_root"
test "$(ruby "$validator" "$baseline_root" "$inventory")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

untracked_root="$scratch/untracked"
make_fixture "$untracked_root"
mkdir -p "$untracked_root/scripts/probe/runtime"
printf '%s\n' '#!/bin/sh' 'codex --version' \
  >"$untracked_root/scripts/probe/runtime/untracked-provider.sh"
test "$(ruby "$validator" "$untracked_root" "$inventory")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

benign_root="$scratch/benign-tracked-shell"
make_fixture "$benign_root"
mkdir -p "$benign_root/scripts/probe/runtime"
printf '%s\n' '#!/bin/sh' ':' >"$benign_root/scripts/probe/runtime/benign.sh"
chmod 0644 "$benign_root/scripts/probe/runtime/benign.sh"
git -C "$benign_root" add scripts/probe/runtime/benign.sh
expect_refusal tracked_shell_source_not_in_inventory "$benign_root" "$inventory"

for shell_extension in sh bash zsh; do
  nested_root="$scratch/nested-$shell_extension"
  make_fixture "$nested_root"
  mkdir -p "$nested_root/scripts/probe/runtime"
  printf '%s\n' '#!/bin/sh' 'codex --version' \
    >"$nested_root/scripts/probe/runtime/unguarded-provider.$shell_extension"
  chmod 0644 "$nested_root/scripts/probe/runtime/unguarded-provider.$shell_extension"
  git -C "$nested_root" add "scripts/probe/runtime/unguarded-provider.$shell_extension"
  expect_refusal "nested_nonexec_$shell_extension" "$nested_root" "$inventory"
done

extensionless_root="$scratch/extensionless"
make_fixture "$extensionless_root"
mkdir -p "$extensionless_root/scripts/probe/runtime"
printf '%s\n' '#!/bin/sh' 'codex --version' \
  >"$extensionless_root/scripts/probe/runtime/unguarded-provider"
git -C "$extensionless_root" add scripts/probe/runtime/unguarded-provider
expect_refusal extensionless_shell_source "$extensionless_root" "$inventory"

extensionless_benign_root="$scratch/extensionless-benign"
make_fixture "$extensionless_benign_root"
mkdir -p "$extensionless_benign_root/scripts/probe/runtime"
printf '%s\n' ':' >"$extensionless_benign_root/scripts/probe/runtime/sourceable"
git -C "$extensionless_benign_root" add scripts/probe/runtime/sourceable
expect_refusal extensionless_sourceable_path "$extensionless_benign_root" "$inventory"

symlink_root="$scratch/symlink"
make_fixture "$symlink_root"
mkdir -p "$symlink_root/scripts/probe/runtime"
ln -s ../provider-capabilities.sh "$symlink_root/scripts/probe/runtime/provider-link.sh"
git -C "$symlink_root" add scripts/probe/runtime/provider-link.sh
expect_refusal tracked_shell_symlink "$symlink_root" "$inventory"
set +e
symlink_public_output=$("$root/scripts/check-public-boundary.sh" --root "$symlink_root" 2>&1)
symlink_public_status=$?
set -e
test "$symlink_public_status" = 10
test "$symlink_public_output" = SYMLINK_ESCAPE

extensionless_symlink_root="$scratch/extensionless-symlink"
make_fixture "$extensionless_symlink_root"
mkdir -p "$extensionless_symlink_root/scripts/probe/runtime"
ln -s ../provider-capabilities.sh "$extensionless_symlink_root/scripts/probe/runtime/provider-link"
git -C "$extensionless_symlink_root" add scripts/probe/runtime/provider-link
expect_refusal tracked_extensionless_symlink "$extensionless_symlink_root" "$inventory"

mode_root="$scratch/mode"
make_fixture "$mode_root"
empty_tree=$(git -C "$mode_root" mktree </dev/null)
gitlink_commit=$(GIT_AUTHOR_NAME=fixture GIT_AUTHOR_EMAIL=fixture@invalid \
  GIT_COMMITTER_NAME=fixture GIT_COMMITTER_EMAIL=fixture@invalid \
  git -C "$mode_root" commit-tree "$empty_tree" -m fixture)
git -C "$mode_root" update-index --add \
  --cacheinfo "160000,$gitlink_commit,scripts/probe/provider-gitlink.sh"
expect_refusal tracked_shell_mode_violation "$mode_root" "$inventory"

extensionless_mode_root="$scratch/extensionless-mode"
make_fixture "$extensionless_mode_root"
extensionless_empty_tree=$(git -C "$extensionless_mode_root" mktree </dev/null)
extensionless_gitlink_commit=$(GIT_AUTHOR_NAME=fixture GIT_AUTHOR_EMAIL=fixture@invalid \
  GIT_COMMITTER_NAME=fixture GIT_COMMITTER_EMAIL=fixture@invalid \
  git -C "$extensionless_mode_root" commit-tree "$extensionless_empty_tree" -m fixture)
git -C "$extensionless_mode_root" update-index --add \
  --cacheinfo "160000,$extensionless_gitlink_commit,scripts/probe/runtime/provider-gitlink"
expect_refusal tracked_extensionless_mode_violation "$extensionless_mode_root" "$inventory"

missing_guard_root="$scratch/missing-guard"
make_fixture "$missing_guard_root"
sed '/^require_preauthenticated_session "\$preauthenticated_session"$/d' \
  "$root/scripts/probe/provider-capabilities.sh" >"$missing_guard_root/scripts/probe/provider-capabilities.sh"
git -C "$missing_guard_root" add scripts/probe/provider-capabilities.sh
expect_refusal missing_probe_guard "$missing_guard_root" "$inventory"

comment_guard_root="$scratch/comment-guard"
make_fixture "$comment_guard_root"
sed 's/^require_preauthenticated_session "\$preauthenticated_session"$/# require_preauthenticated_session "$preauthenticated_session"/' \
  "$root/scripts/probe/provider-capabilities.sh" >"$comment_guard_root/scripts/probe/provider-capabilities.sh"
git -C "$comment_guard_root" add scripts/probe/provider-capabilities.sh
expect_refusal commented_probe_guard "$comment_guard_root" "$inventory"

moved_guard_root="$scratch/moved-guard"
make_fixture "$moved_guard_root"
awk '
  $0 != "require_preauthenticated_session \"$preauthenticated_session\"" { print }
  END { print "require_preauthenticated_session \"$preauthenticated_session\"" }
' "$root/scripts/probe/provider-capabilities.sh" >"$moved_guard_root/scripts/probe/provider-capabilities.sh"
git -C "$moved_guard_root" add scripts/probe/provider-capabilities.sh
expect_refusal guard_after_provider_birth "$moved_guard_root" "$inventory"

duplicate_root="$scratch/duplicate-birth"
make_fixture "$duplicate_root"
printf '%s\n' 'version=$(codex --version)' >>"$duplicate_root/scripts/probe/provider-capabilities.sh"
git -C "$duplicate_root" add scripts/probe/provider-capabilities.sh
jq '.provider_births += [.provider_births[0]]' "$inventory" >"$scratch/duplicate-birth.json"
test "$(ruby "$validator" "$duplicate_root" "$scratch/duplicate-birth.json")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

for direct_form in bare exec prompt env path; do
  direct_root="$scratch/direct-$direct_form"
  make_fixture "$direct_root"
  case "$direct_form" in
    bare) printf '%s\n' 'codex' >>"$direct_root/scripts/probe/provider-capabilities.sh" ;;
    exec) printf '%s\n' 'exec claude' >>"$direct_root/scripts/probe/provider-capabilities.sh" ;;
    prompt) printf '%s\n' "codex -p 'local probe'" >>"$direct_root/scripts/probe/provider-capabilities.sh" ;;
    env) printf '%s\n' "env TASKSEAL_LOCAL=1 claude -p 'local probe'" >>"$direct_root/scripts/probe/provider-capabilities.sh" ;;
    path) printf '%s\n' "'/opt/providers/codex' -p 'local probe'" >>"$direct_root/scripts/probe/provider-capabilities.sh" ;;
  esac
  git -C "$direct_root" add scripts/probe/provider-capabilities.sh
  expect_refusal "unlisted_static_direct_$direct_form" "$direct_root" "$inventory"
done

multiline_root="$scratch/direct-multiline"
make_fixture "$multiline_root"
printf '%s\n' 'exec \' '  codex \' "  -p 'local probe'" \
  >>"$multiline_root/scripts/probe/provider-capabilities.sh"
git -C "$multiline_root" add scripts/probe/provider-capabilities.sh
expect_refusal unlisted_static_direct_multiline "$multiline_root" "$inventory"

quoted_argument_root="$scratch/quoted-provider-argument"
make_fixture "$quoted_argument_root"
printf '%s\n' "printf '%s\\n' 'codex -p is documentation text'" \
  >>"$quoted_argument_root/scripts/probe/provider-capabilities.sh"
git -C "$quoted_argument_root" add scripts/probe/provider-capabilities.sh
test "$(ruby "$validator" "$quoted_argument_root" "$inventory")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS

for ambiguous_form in eval shell_c xargs backtick heredoc; do
  ambiguous_root="$scratch/ambiguous-$ambiguous_form"
  make_fixture "$ambiguous_root"
  case "$ambiguous_form" in
    eval) printf '%s\n' 'eval "$provider_command"' >>"$ambiguous_root/scripts/probe/provider-capabilities.sh" ;;
    shell_c) printf '%s\n' 'sh -c "$provider_command"' >>"$ambiguous_root/scripts/probe/provider-capabilities.sh" ;;
    xargs) printf '%s\n' 'xargs "$provider_command"' >>"$ambiguous_root/scripts/probe/provider-capabilities.sh" ;;
    backtick) printf '%s\n' 'result=`$provider_command`' >>"$ambiguous_root/scripts/probe/provider-capabilities.sh" ;;
    heredoc) printf '%s\n' 'sh <<EOF' '$provider_command' 'EOF' >>"$ambiguous_root/scripts/probe/provider-capabilities.sh" ;;
  esac
  git -C "$ambiguous_root" add scripts/probe/provider-capabilities.sh
  expect_refusal "ambiguous_provider_dispatch_$ambiguous_form" "$ambiguous_root" "$inventory"
done

rust_root="$scratch/unguarded-rust"
make_fixture "$rust_root"
printf '%s\n' 'fn rogue() { let _ = Command::new("other-provider"); }' \
  >"$rust_root/src/adapters/unguarded.rs"
expect_refusal future_unguarded_provider_birth "$rust_root" "$inventory"

runtime_root="$scratch/runtime-public-boundary"
make_fixture "$runtime_root"
mkdir -p "$runtime_root/scripts/runtime"
printf '%s\n' '#!/bin/sh' 'codex --version' \
  >"$runtime_root/scripts/runtime/unguarded-provider.sh"
git -C "$runtime_root" add scripts/runtime/unguarded-provider.sh
test "$(ruby "$validator" "$runtime_root" "$inventory")" = P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS
set +e
runtime_output=$("$root/scripts/check-public-boundary.sh" --root "$runtime_root" 2>&1)
runtime_status=$?
set -e
test "$runtime_status" = 11
test "$runtime_output" = "UNALLOWLISTED_PUBLIC_PATH:scripts/runtime/unguarded-provider.sh"

printf '%s\n' P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_TEST_PASS

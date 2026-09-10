#!/bin/sh
set -eu

REPO="ewgenij87snwork/clean-room-launcher"
RELEASE_BASE="https://github.com/$REPO/releases/latest/download"

fail() {
  printf 'CLROOM_INSTALL_FAILED:%s\n' "$1" >&2
  exit 1
}

platform_supported() {
  [ "$1" = "Darwin" ] && [ "$2" = "arm64" ]
}

install_target_safe() {
  target=$1
  [ ! -L "$target" ] || return 1
  [ ! -e "$target" ] || [ -f "$target" ]
}

install_dir_safe() {
  target=$1
  [ "$target" = "$HOME/.local/bin" ] || return 1
  [ ! -L "$HOME/.local" ] || return 1
  [ ! -e "$HOME/.local" ] || [ -d "$HOME/.local" ] || return 1
  [ ! -L "$target" ] || return 1
  [ ! -e "$target" ] || [ -d "$target" ]
}

preflight_install_targets() {
  install_dir_safe "$install_dir" || return 1
  for name in clroom clroom-codex clroom-claude; do
    install_target_safe "$install_dir/$name" || return 1
  done
}

select_subject() {
  /usr/bin/awk '
    NF == 2 &&
    length($1) == 64 &&
    $1 !~ /[^0-9A-Fa-f]/ &&
    $2 ~ /^clean-room-launcher-v[0-9]+\.[0-9]+\.[0-9]+-aarch64-apple-darwin\.tar\.gz$/ {
      print tolower($1), $2
    }
  ' "$1"
}

load_subject() {
  entries=$(select_subject "$1") || return 1
  count=$(printf '%s\n' "$entries" | /usr/bin/awk 'NF { n += 1 } END { print n + 0 }')
  [ "$count" -eq 1 ] || return 1
  EXPECTED_SHA=$(printf '%s\n' "$entries" | /usr/bin/awk 'NF { print $1 }')
  ASSET=$(printf '%s\n' "$entries" | /usr/bin/awk 'NF { print $2 }')
  [ -n "$EXPECTED_SHA" ] && [ -n "$ASSET" ]
}

verify_archive() {
  actual=$(/usr/bin/shasum -a 256 "$1" | /usr/bin/awk '{ print tolower($1) }') || return 1
  [ "$actual" = "$EXPECTED_SHA" ]
}

extract_binary() {
  archive=$1
  output=$2
  root=${ASSET%.tar.gz}
  /bin/mkdir -p "$output" || return 1
  for name in clroom clroom-codex clroom-claude; do
    member="$root/bin/$name"
    count=$(/usr/bin/tar -tzf "$archive" | /usr/bin/awk -v expected="$member" '$0 == expected { n += 1 } END { print n + 0 }') || return 1
    [ "$count" -eq 1 ] || return 1
    regular=$(/usr/bin/tar -tvzf "$archive" "$member" | /usr/bin/awk -v expected="$member" '$1 ~ /^-/ && $NF == expected { n += 1 } END { print n + 0 }') || return 1
    [ "$regular" -eq 1 ] || return 1
    /usr/bin/tar -xOzf "$archive" "$member" > "$output/$name" || return 1
    [ -s "$output/$name" ] || return 1
    /bin/chmod 0755 "$output/$name" || return 1
  done
}

self_test() {
  test_root=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/clroom-installer-test.XXXXXX") || fail "SELF_TEST_TMPDIR"
  root="clean-room-launcher-v0.2.0-aarch64-apple-darwin"
  asset="$root.tar.gz"
  /bin/mkdir -p "$test_root/$root/bin"
  for name in clroom clroom-codex clroom-claude; do
    printf '#!/bin/sh\nprintf "%s fixture\\n"\n' "$name" > "$test_root/$root/bin/$name"
    /bin/chmod 0755 "$test_root/$root/bin/$name"
  done
  /usr/bin/tar -czf "$test_root/$asset" -C "$test_root" "$root"
  digest=$(/usr/bin/shasum -a 256 "$test_root/$asset" | /usr/bin/awk '{ print $1 }')
  zeros=$(printf '0%.0s' 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64)
  printf '%s  %s\n%s  %s\n' "$digest" "$asset" "$zeros" "sbom.cdx.json" > "$test_root/SHA256SUMS"

  platform_supported Darwin arm64 || fail "SELF_TEST_PLATFORM_POSITIVE"
  if platform_supported Darwin x86_64 || platform_supported Linux arm64; then
    fail "SELF_TEST_PLATFORM_NEGATIVE"
  fi

  missing_target="$test_root/missing-target"
  install_target_safe "$missing_target" || fail "SELF_TEST_TARGET_MISSING"
  printf 'existing\n' > "$test_root/existing-target"
  install_target_safe "$test_root/existing-target" || fail "SELF_TEST_TARGET_REGULAR"
  /bin/mkdir "$test_root/target-dir"
  if install_target_safe "$test_root/target-dir"; then
    fail "SELF_TEST_TARGET_DIRECTORY_ACCEPTED"
  fi
  /bin/ln -s "$test_root/existing-target" "$test_root/target-link"
  if install_target_safe "$test_root/target-link"; then
    fail "SELF_TEST_TARGET_SYMLINK_ACCEPTED"
  fi

  saved_home=$HOME
  HOME="$test_root/home"
  /bin/mkdir -p "$HOME"
  /bin/ln -s "$test_root" "$HOME/.local"
  if install_dir_safe "$HOME/.local/bin"; then
    fail "SELF_TEST_INSTALL_DIR_SYMLINK_ACCEPTED"
  fi
  /bin/rm "$HOME/.local"
  printf '%s\n' directory > "$HOME/.local"
  if install_dir_safe "$HOME/.local/bin"; then
    fail "SELF_TEST_INSTALL_DIR_FILE_ACCEPTED"
  fi
  HOME=$saved_home
  export HOME

  load_subject "$test_root/SHA256SUMS" || fail "SELF_TEST_SUBJECT"
  verify_archive "$test_root/$asset" || fail "SELF_TEST_DIGEST"
  extract_binary "$test_root/$asset" "$test_root/bin-out" || fail "SELF_TEST_EXTRACT"
  for name in clroom clroom-codex clroom-claude; do
    [ "$("$test_root/bin-out/$name")" = "$name fixture" ] || fail "SELF_TEST_BINARY"
  done

  /bin/cp "$test_root/SHA256SUMS" "$test_root/SHA256SUMS.duplicate"
  printf '%s  %s\n' "$digest" "$asset" >> "$test_root/SHA256SUMS.duplicate"
  if load_subject "$test_root/SHA256SUMS.duplicate"; then
    fail "SELF_TEST_DUPLICATE_ACCEPTED"
  fi

  load_subject "$test_root/SHA256SUMS" || fail "SELF_TEST_SUBJECT_RELOAD"
  /bin/cp "$test_root/$asset" "$test_root/tampered.tar.gz"
  printf 'tamper' >> "$test_root/tampered.tar.gz"
  if verify_archive "$test_root/tampered.tar.gz"; then
    fail "SELF_TEST_TAMPER_ACCEPTED"
  fi

  /bin/rm -rf -- "$test_root/${root:?}"
  /bin/mkdir -p "$test_root/wrong-root/bin"
  for name in clroom clroom-codex clroom-claude; do printf '#!/bin/sh\nexit 0\n' > "$test_root/wrong-root/bin/$name"; done
  /usr/bin/tar -czf "$test_root/$asset" -C "$test_root" wrong-root
  digest=$(/usr/bin/shasum -a 256 "$test_root/$asset" | /usr/bin/awk '{ print $1 }')
  printf '%s  %s\n' "$digest" "$asset" > "$test_root/SHA256SUMS.layout"
  load_subject "$test_root/SHA256SUMS.layout" || fail "SELF_TEST_LAYOUT_SUBJECT"
  verify_archive "$test_root/$asset" || fail "SELF_TEST_LAYOUT_DIGEST"
  if extract_binary "$test_root/$asset" "$test_root/layout-clroom"; then
    fail "SELF_TEST_LAYOUT_ACCEPTED"
  fi

  /bin/rm -rf "$test_root"
  printf 'CLROOM_INSTALLER_SELF_TEST_PASS\n'
}

if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit 0
fi
[ "$#" -eq 0 ] || fail "UNEXPECTED_ARGUMENT"

platform_supported "$(/usr/bin/uname -s)" "$(/usr/bin/uname -m)" || fail "MACOS_APPLE_SILICON_REQUIRED"
[ -n "${HOME:-}" ] || fail "HOME_REQUIRED"

install_dir="$HOME/.local/bin"
tmp=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/clroom-install.XXXXXX") || fail "TMPDIR"
target_tmp=""
trap '[ -z "${target_tmp:-}" ] || /bin/rm -f "$target_tmp"; /bin/rm -rf "$tmp"' EXIT HUP INT TERM

manifest="$tmp/SHA256SUMS"
/usr/bin/curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error --retry 3 \
  --output "$manifest" "$RELEASE_BASE/SHA256SUMS" || fail "CHECKSUM_DOWNLOAD"
load_subject "$manifest" || fail "RELEASE_MANIFEST"

archive="$tmp/$ASSET"
/usr/bin/curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error --retry 3 \
  --output "$archive" "$RELEASE_BASE/$ASSET" || fail "ARCHIVE_DOWNLOAD"
verify_archive "$archive" || fail "ARCHIVE_CHECKSUM"
extract_binary "$archive" "$tmp/bin" || fail "ARCHIVE_LAYOUT"
for name in clroom clroom-codex clroom-claude; do "$tmp/bin/$name" --help >/dev/null 2>&1 || fail "STAGED_BINARY_CHECK"; done

preflight_install_targets || fail "INSTALL_TARGET_PREFLIGHT"
/bin/mkdir -p "$install_dir" || fail "INSTALL_DIR"
for name in clroom clroom-codex clroom-claude; do
  target="$install_dir/$name"
  install_target_safe "$target" || fail "INSTALL_TARGET_UNSAFE"
  target_tmp=$(/usr/bin/mktemp "$install_dir/.$name-install.XXXXXX") || fail "INSTALL_TEMP"
  /usr/bin/install -m 0755 "$tmp/bin/$name" "$target_tmp" || fail "INSTALL_STAGE"
  "$target_tmp" --help >/dev/null 2>&1 || fail "INSTALL_STAGE_CHECK"
  /bin/mv -f "$target_tmp" "$target" || fail "INSTALL_BINARY"
  target_tmp=""
done

printf 'Installed CLROOM to %s\n' "$target"
case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *) printf 'Add %s to PATH to run clroom from any directory.\n' "$install_dir" ;;
esac

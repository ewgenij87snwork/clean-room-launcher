#!/bin/sh
set -eu

REPO="ewgenij87snwork/clean-room-launcher"
RELEASE_BASE="https://github.com/$REPO/releases/latest/download"

fail() {
  printf 'CLROOM_INSTALL_FAILED:%s\n' "$1" >&2
  exit 1
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
  member="$root/bin/clroom"
  count=$(/usr/bin/tar -tzf "$archive" | /usr/bin/awk -v expected="$member" '$0 == expected { n += 1 } END { print n + 0 }') || return 1
  [ "$count" -eq 1 ] || return 1
  /usr/bin/tar -xOzf "$archive" "$member" > "$output" || return 1
  [ -s "$output" ]
}

self_test() {
  test_root=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/clroom-installer-test.XXXXXX") || fail "SELF_TEST_TMPDIR"
  root="clean-room-launcher-v0.2.0-aarch64-apple-darwin"
  asset="$root.tar.gz"
  /bin/mkdir -p "$test_root/$root/bin"
  printf '#!/bin/sh\nprintf "clroom fixture\\n"\n' > "$test_root/$root/bin/clroom"
  /bin/chmod 0755 "$test_root/$root/bin/clroom"
  /usr/bin/tar -czf "$test_root/$asset" -C "$test_root" "$root"
  digest=$(/usr/bin/shasum -a 256 "$test_root/$asset" | /usr/bin/awk '{ print $1 }')
  printf '%s  %s\n%s  %s\n' "$digest" "$asset" "$(printf '0%.0s' 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64)" "sbom.cdx.json" > "$test_root/SHA256SUMS"

  load_subject "$test_root/SHA256SUMS" || fail "SELF_TEST_SUBJECT"
  verify_archive "$test_root/$asset" || fail "SELF_TEST_DIGEST"
  extract_binary "$test_root/$asset" "$test_root/clroom" || fail "SELF_TEST_EXTRACT"
  [ "$("$test_root/clroom")" = "clroom fixture" ] || fail "SELF_TEST_BINARY"

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

  /bin/rm -rf "$test_root"
  printf 'CLROOM_INSTALLER_SELF_TEST_PASS\n'
}

if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit 0
fi
[ "$#" -eq 0 ] || fail "UNEXPECTED_ARGUMENT"

[ "$(/usr/bin/uname -s)" = "Darwin" ] || fail "MACOS_REQUIRED"
[ "$(/usr/bin/uname -m)" = "arm64" ] || fail "APPLE_SILICON_REQUIRED"
[ -n "${HOME:-}" ] || fail "HOME_REQUIRED"

install_dir="$HOME/.local/bin"
tmp=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/clroom-install.XXXXXX") || fail "TMPDIR"
trap '/bin/rm -rf "$tmp"' EXIT HUP INT TERM

manifest="$tmp/SHA256SUMS"
/usr/bin/curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error --retry 3 \
  --output "$manifest" "$RELEASE_BASE/SHA256SUMS" || fail "CHECKSUM_DOWNLOAD"
load_subject "$manifest" || fail "RELEASE_MANIFEST"

archive="$tmp/$ASSET"
/usr/bin/curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error --retry 3 \
  --output "$archive" "$RELEASE_BASE/$ASSET" || fail "ARCHIVE_DOWNLOAD"
verify_archive "$archive" || fail "ARCHIVE_CHECKSUM"
extract_binary "$archive" "$tmp/clroom" || fail "ARCHIVE_LAYOUT"

/bin/mkdir -p "$install_dir" || fail "INSTALL_DIR"
/usr/bin/install -m 0755 "$tmp/clroom" "$install_dir/clroom" || fail "INSTALL_BINARY"
"$install_dir/clroom" --help >/dev/null 2>&1 || fail "INSTALLED_BINARY_CHECK"

printf 'Installed CLROOM to %s/clroom\n' "$install_dir"
case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *) printf 'Add %s to PATH to run clroom from any directory.\n' "$install_dir" ;;
esac

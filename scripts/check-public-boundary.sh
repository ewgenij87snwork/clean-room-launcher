#!/bin/sh
set -eu

[ "$#" -eq 2 ] && [ "$1" = "--root" ] || {
  echo "usage: check-public-boundary.sh --root PATH" >&2
  exit 64
}

root=$(cd "$2" && pwd -P)

if find "$root" \( -name .git -o -name target -o -name .taskseal-dev \) -prune -o -type l -print | grep -q .; then
  echo "SYMLINK_ESCAPE" >&2
  exit 10
fi

inventory=$(mktemp "${TMPDIR:-/tmp}/taskseal-public-inventory.XXXXXX")
cleanup() {
  case "$inventory" in
    "${TMPDIR:-/tmp}"/taskseal-public-inventory.*) rm -f -- "$inventory" ;;
    *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

find "$root" \( -name .git -o -name target -o -name .taskseal-dev \) -prune -o -type f -print |
  LC_ALL=C sort > "$inventory"

while IFS= read -r file; do
  relative=${file#"$root"/}
  case "$relative" in
    AGENTS.md|README.md|Cargo.toml|Cargo.lock|rust-toolchain.toml|LICENSE|SECURITY.md|GOVERNANCE.md|CHANGELOG.md|deny.toml|.gitignore|schemas/canonical-json-profile.md) ;;
    src/*|schemas/contracts/*|fixtures/contracts/*|fixtures/core/*|fixtures/catalog/*|fixtures/cli/*|adapters/declarations/*|tests/contracts/*|tests/core/*|tests/catalog/*|tests/cli.rs|tests/cli/*|tests/fixtures/*|controls/*|scripts/check-public-boundary.sh|scripts/check-control-coverage.rb|scripts/probe/*|scripts/gates/p02/*|scripts/gates/p03/*|scripts/gates/p04/*|scripts/gates/p05/*|reports/contracts/*|reports/gates/p02/*|reports/gates/p03/*|reports/gates/p04/*|reports/gates/p05/*) ;;
    *) echo "UNALLOWLISTED_PUBLIC_PATH:$relative" >&2; exit 11 ;;
  esac

  case "$relative" in
    tests/fixtures/public-boundary/*) continue ;;
  esac

  if LC_ALL=C grep -E -q 'role:[[:space:]]*(conductor|teacher|student|verifier)' "$file"; then
    echo "PRIVATE_PRAXIS_ROLE" >&2
    exit 12
  fi
  if LC_ALL=C grep -E -q '(/Users/[A-Za-z0-9._-]+/|/home/[A-Za-z0-9._-]+/)' "$file"; then
    echo "ABSOLUTE_HOME_PATH" >&2
    exit 13
  fi
  if LC_ALL=C grep -E -q '(ghp_[A-Za-z0-9]{30,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16})' "$file"; then
    echo "CREDENTIAL_TOKEN" >&2
    exit 14
  fi
  if LC_ALL=C grep -E -q '"role"[[:space:]]*:[[:space:]]*"(user|assistant)"[[:space:]]*,[[:space:]]*"content"' "$file"; then
    echo "TRANSCRIPT_FRAGMENT" >&2
    exit 15
  fi
done < "$inventory"

cleanup
trap - EXIT HUP INT TERM
echo "PUBLIC_BOUNDARY_PASS"

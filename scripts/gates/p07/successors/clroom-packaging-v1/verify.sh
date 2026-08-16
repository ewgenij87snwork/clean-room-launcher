#!/bin/sh
# Sole local completion gate for P07-CLROOM-PACKAGING-V1.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
work=$(mktemp -d "${TMPDIR:-/tmp}/clroom-p07-gate.XXXXXX")

artifact_root="$work/artifact"
fixture_root="$work/fixtures"
lifecycle_root="$work/lifecycle"
result="$work/lifecycle.json"

CARGO_TARGET_DIR="$work/target" CARGO_NET_OFFLINE=true \
  "$root/packaging/build-artifacts.sh" "$artifact_root" >/dev/null
archive=$(find "$artifact_root" -type f -name 'clean-room-launcher-*.tar.gz' -print -quit)
[ -n "$archive" ] || { echo "P07_CLROOM_PACKAGING_REFUSED:ARCHIVE_MISSING" >&2; exit 1; }
python3 "$root/packaging/verify-artifact.py" "$archive" >/dev/null

python3 "$root/tests/packaging/fixtures/homebrew/make_fixture_archives.py" \
  --output "$fixture_root" >/dev/null
mkdir -p "$lifecycle_root"
python3 "$root/packaging/homebrew/lifecycle.py" \
  --fake --fake-brew "$root/tests/packaging/fixtures/homebrew/fake_brew.py" \
  --workspace "$lifecycle_root" --output "$result" >/dev/null

ruby -rjson -e '
  v = JSON.parse(File.read(ARGV.fetch(0)))
  abort "P07_CLROOM_PACKAGING_REFUSED:LIFECYCLE_SCHEMA" unless v["schema_version"] == "taskseal.p07.homebrew-lifecycle.v1"
  abort "P07_CLROOM_PACKAGING_REFUSED:LIFECYCLE_QUALIFICATION" unless v["qualification"] == "NOT_QUALIFIED"
  abort "P07_CLROOM_PACKAGING_REFUSED:LIFECYCLE_FAILURE" unless v["failure_class"].nil? && v["cleanup_complete"]
  abort "P07_CLROOM_PACKAGING_REFUSED:LIFECYCLE_CHECKS" unless v.fetch("checks").values.all?
  names = v.fetch("steps").map { |s| s.fetch("name") }
  required = %w[install_n upgrade_n_plus_1 rollback_n uninstall_current untap]
  abort "P07_CLROOM_PACKAGING_REFUSED:LIFECYCLE_STEPS" unless required.all? { |name| names.include?(name) }
' "$result"

sha=$(shasum -a 256 "$archive" | awk '{print $1}')
printf 'P07_CLROOM_PACKAGING_V1_PASS artifact_sha256=%s lifecycle=local-fixture-no-network\n' "$sha"

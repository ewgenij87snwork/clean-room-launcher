#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
candidate=${1:-"$root/.github/workflows/release-candidate.yml"}
release=${2:-"$root/.github/workflows/release.yml"}

fail() {
  printf "RELEASE_GOVERNANCE_CONTRACT_BLOCKED:%s\n" "$1" >&2
  exit 1
}

check_file() {
  local file=$1
  [[ -f "$file" ]] || fail "MISSING_WORKFLOW"
}
check_file "$candidate"
check_file "$release"

for file in "$candidate" "$release"; do
  grep -Fq 'repos/$GITHUB_REPOSITORY/releases/latest' "$file" || fail "PUBLISHED_BASELINE"
  grep -Fq 'scripts/release/audit-release-delta.py' "$file" || fail "FULL_DELTA_AUDIT"
  grep -Fq 'release-governance-evidence' "$file" || fail "GOVERNANCE_EVIDENCE"
done

grep -Fq 'CLROOM_RELEASE_DELTA_AUDIT=$audit_mode' "$candidate" || fail "PR_AUDIT_MODE"
grep -Fq 'fetch-depth: 0' "$candidate" || fail "PR_FULL_HISTORY"

grep -Fq 'release tag must point to the exact accepted default-branch tip' "$release" || fail "EXACT_MAIN_BINDING"
grep -Fq 'release_date="$(date -u +%F)"' "$release" || fail "UTC_RELEASE_DATE"
grep -Fq 'title="$GITHUB_REF_NAME — Clean Room Launcher"' "$release" || fail "VERSION_FIRST_TITLE"
grep -Fq 'args=(release create "$tag" --draft --verify-tag' "$release" || fail "DRAFT_ONLY_CREATION"
if grep -Eq 'gh release (edit|create).*--(draft=false|latest)' "$release"; then
  fail "PUBLICATION_IN_TAG_WORKFLOW"
fi

for path in \
  "$root/packaging/release-contract.json" \
  "$root/scripts/release/audit-release-delta.py" \
  "$root/scripts/release/local-release-review.sh" \
  "$root/scripts/release/verify-draft-plugin-activation.sh" \
  "$root/scripts/release/verify-published-release.sh"; do
  [[ -f "$path" ]] || fail "MISSING_GOVERNANCE_COMPONENT"
done

grep -Fq 'DRAFT_PLUGIN_E2E_BLOCKED:RELEASE_NOT_DRAFT' "$root/scripts/release/verify-draft-plugin-activation.sh" || fail "DRAFT_RELEASE_STATE_BINDING"
grep -Fq 'DRAFT_PLUGIN_E2E_BLOCKED:SOURCE_COMMIT_MISMATCH' "$root/scripts/release/verify-draft-plugin-activation.sh" || fail "DRAFT_ARTIFACT_SOURCE_BINDING"
grep -Fq 'releases/latest/download/install.sh' "$root/scripts/release/verify-published-release.sh" || fail "PUBLIC_LATEST_INSTALL_PATH"
grep -Fq 'PUBLIC_INSTALL_SMOKE_' "$root/scripts/release/verify-published-release.sh" || fail "PUBLIC_INSTALL_SMOKE"

echo "RELEASE_GOVERNANCE_CONTRACT_PASS"

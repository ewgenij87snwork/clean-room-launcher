#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
provisioner="$root/scripts/release/provision-provider-canaries.sh"
release_candidate="$root/.github/workflows/release-candidate.yml"
release="$root/.github/workflows/release.yml"

fail() {
  printf 'PROVIDER_CANARY_CONTRACT_BLOCKED:%s\n' "$1" >&2
  exit 1
}

for file in "$provisioner" "$release_candidate" "$release"; do
  [[ -f "$file" ]] || fail "FILE_MISSING"
done

for needle in \
  '@openai/codex@0.154.0' \
  '@openai/codex@0.154.0-darwin-arm64' \
  '@anthropic-ai/claude-code@2.1.272' \
  'FV/x1OHXYv/ifjf3mXj9ThTTAWcUZN6cGIRQRhRxkKNOPuImu1WW0c8ev1vUkE9XGH90dEnYG1tBjIkxRikg0w==' \
  'HP/vJCH/t2hB9Kg6hotN9UglClJ6/z584fal5lEP14C9gNAgAQS4/kTQC7l5V+BA3TqwDPwINSjul28cX8AYXg==' \
  'sOwHBM69H8Zka3/D3rc2VNNemPYNlgfYTdhsoqPoXZdK5KcKQlzoue4asJ2RVc+tGb/Pz1qxjVV9nVJQ87W7Ng==' \
  'aarch64-apple-darwin/bin/codex'; do
  grep -Fq "$needle" "$provisioner" || fail "PIN_OR_LAYOUT_MISSING"
done

for workflow in "$release_candidate" "$release"; do
  grep -Fq './scripts/release/provision-provider-canaries.sh "$RUNNER_TEMP/clroom-providers" "$GITHUB_ENV"' "$workflow" \
    || fail "WORKFLOW_PROVISIONER_MISSING"
  if grep -Eq 'npm[[:space:]]+install[[:space:]]' "$workflow"; then
    fail "WORKFLOW_NPM_INSTALL_FORBIDDEN"
  fi
done

if grep -Eq 'npm[[:space:]]+install[[:space:]]' "$provisioner"; then
  fail "PROVISIONER_NPM_INSTALL_FORBIDDEN"
fi

bash -n "$provisioner" || fail "PROVISIONER_SYNTAX"
printf 'PROVIDER_CANARY_CONTRACT_PASS\n'

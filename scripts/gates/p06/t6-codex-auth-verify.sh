#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/task-6.json
jq -e '
  keys == ["auth_state","network","output","provider_launch","qualification","result","schema_version","sources","subject","task"] and
  .schema_version == "taskseal.codex-auth-receipt.v1" and .task == 6 and .result == "accepted" and
  .auth_state == "UNAVAILABLE" and .network == "NOT_ATTEMPTED" and .provider_launch == false and .qualification == "NOT_QUALIFIED" and
  ([.sources[].path] == ["src/adapters/codex/mod.rs","src/adapters/codex/environment.rs","tests/adapters.rs","tests/adapters/codex/environment.rs"]) and
  (.output.path == "outputs/task-6-offline-auth.txt") and (.output.sha256|test("^[0-9a-f]{64}$"))
' "$receipt" >/dev/null
subject=$(jq -r .subject "$receipt")
git rev-parse --verify "$subject^{commit}" >/dev/null && git merge-base --is-ancestor "$subject" HEAD
git diff --quiet "$subject" -- src/adapters/codex/mod.rs src/adapters/codex/environment.rs tests/adapters.rs tests/adapters/codex/environment.rs
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do test "$(git show "$subject:$path" | shasum -a 256 | awk '{print $1}')" = "$expected"; done
test "$(shasum -a 256 reports/gates/p06/outputs/task-6-offline-auth.txt | awk '{print $1}')" = "$(jq -r .output.sha256 "$receipt")"
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/' "$receipt" reports/gates/p06/outputs/task-6-offline-auth.txt; then
  echo P06_T6_PRIVACY_REFUSED >&2
  exit 2
fi
timeout 120 cargo test --test adapters codex_environment --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
echo P06_T6_AUTH_PASS

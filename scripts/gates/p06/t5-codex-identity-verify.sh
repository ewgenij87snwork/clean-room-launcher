#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
cd "$root"
receipt=reports/gates/p06/task-5.json
jq -e '
  keys == ["output","provider_launch","qualification","result","schema_version","sources","subject","task","tuple"] and
  .schema_version == "taskseal.codex-tuple-receipt.v1" and .task == 5 and .result == "accepted" and
  .qualification == "NOT_QUALIFIED" and .provider_launch == false and
  .tuple == {"provider_id":"codex","artifact_digest":"19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37","version":[0,147,0],"os":"macos","arch":"aarch64"} and
  (.subject|test("^[0-9a-f]{40}$")) and
  ([.sources[].path] == ["src/adapters/mod.rs","src/adapters/codex/mod.rs","src/adapters/codex/identity.rs","tests/adapters.rs","tests/adapters/codex/identity.rs"]) and
  all(.sources[]; keys == ["path","sha256"] and (.sha256|test("^[0-9a-f]{64}$"))) and
  (.output.path == "outputs/task-5-local-preflight.txt") and (.output.sha256|test("^[0-9a-f]{64}$"))
' "$receipt" >/dev/null || { echo P06_T5_RECEIPT_INVALID >&2; exit 2; }
subject=$(jq -r .subject "$receipt")
git rev-parse --verify "$subject^{commit}" >/dev/null 2>&1 && git merge-base --is-ancestor "$subject" HEAD || { echo P06_T5_SUBJECT_INVALID >&2; exit 2; }
git diff --quiet "$subject" -- src/adapters/mod.rs src/adapters/codex/mod.rs src/adapters/codex/identity.rs tests/adapters.rs tests/adapters/codex/identity.rs || { echo P06_T5_SOURCE_CHANGED_AFTER_SUBJECT >&2; exit 2; }
jq -r '.sources[] | [.path,.sha256] | @tsv' "$receipt" | while IFS='	' read -r path expected; do
  actual=$(git show "$subject:$path" | shasum -a 256 | awk '{print $1}')
  test "$actual" = "$expected" || { echo P06_T5_SOURCE_MISMATCH >&2; exit 2; }
done
expected=$(jq -r .output.sha256 "$receipt")
actual=$(shasum -a 256 reports/gates/p06/outputs/task-5-local-preflight.txt | awk '{print $1}')
test "$actual" = "$expected" || { echo P06_T5_OUTPUT_MISMATCH >&2; exit 2; }
command=$(command -v codex) || { echo P06_T5_CODEX_UNAVAILABLE >&2; exit 2; }
test "$(shasum -a 256 "$command" | awk '{print $1}')" = "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37" || { echo P06_T5_ARTIFACT_MISMATCH >&2; exit 2; }
test "$(env -i PATH=/usr/bin:/bin "$command" --version)" = "codex-cli 0.147.0" || { echo P06_T5_VERSION_MISMATCH >&2; exit 2; }
test "$(uname -s)" = Darwin && test "$(uname -m)" = arm64 || { echo P06_T5_PLATFORM_MISMATCH >&2; exit 2; }
! rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/' "$receipt" reports/gates/p06/outputs/task-5-local-preflight.txt
timeout 120 cargo test --test adapters codex_identity --locked --offline
timeout 120 cargo clippy --all-targets --all-features --locked --offline -- -D warnings
rg -n 'P06_REQUIRED: provider tuple is not qualified' src/cli/dispatch.rs >/dev/null
echo P06_T5_IDENTITY_PASS

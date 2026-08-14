#!/bin/sh
set -eu
root=${P06_GATE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)}
cd "$(CDPATH= cd -- "$root" && pwd -P)"
jq -e '
  .schema_version == "taskseal.p06-qualification-gate.v1" and
  .gate == "scripts/gates/p06/verify.sh" and .result == "QUALIFICATION_GATE_PASS_NOT_QUALIFIED" and
  .tasks.foundation == "4/4" and .tasks.tuple == "8/8" and .tasks.qualified == "0/1" and
  .required_tuple.qualification == "NOT_QUALIFIED" and .native_processes == 0 and
  .raw_output_retained == false and .skip_as_pass == false and
  (.unsupported_tuples == "NOT_QUALIFIED") and (.reason|length > 20) and
  (.subject|test("^[0-9a-f]{40}$"))
' reports/gates/p06/qualification-gate.json >/dev/null
git rev-parse --verify "$(jq -r .subject reports/gates/p06/qualification-gate.json)^{commit}" >/dev/null
git merge-base --is-ancestor "$(jq -r .subject reports/gates/p06/qualification-gate.json)" HEAD
jq -e '.schema_version == "taskseal.foundation-v6-report.v1" and .result == "FOUNDATION_4_OF_4_CANDIDATE" and .qualification == "NOT_QUALIFIED" and .provider_launch == false and [.receipts[].path] == ["task-3.json","task-4.json"]' reports/gates/p06/foundation-v6.json >/dev/null
scripts/gates/p06/codex/verify.sh
jq -e '.schema_version == "taskseal.p06-runtime-campaign.v1" and (.cases|length == 1) and .native_processes >= 1 and .native_processes <= 15 and .raw_output_retained == false and .qualification == "NOT_QUALIFIED" and (.cases[0].digest|test("^[0-9a-f]{64}$"))' reports/gates/p06/runtime-campaign.json >/dev/null
if rg -n '"qualification":"QUALIFIED"' reports/gates/p06/qualification-gate.json qualification/codex/macos-aarch64-0.147.0.json; then
  echo P06_GATE_UNSUPPORTED_CLAIM_OR_SKIP >&2
  exit 2
fi
if rg -n -i 'secret|token|password|api[_-]?key|credential|authorization|bearer|/Users/|/home/|prompt=|transcript' reports/gates/p06/qualification-gate.json qualification/codex/macos-aarch64-0.147.0.json; then
  echo P06_GATE_PRIVACY_REFUSED >&2
  exit 2
fi
echo P06_13_OF_13_CHECKS_SEALED_NOT_QUALIFIED

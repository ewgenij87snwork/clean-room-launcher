#!/bin/sh
# Artifact-bound preparation only. This harness deliberately cannot launch Codex.
set -eu

artifact= artifact_sha= p06= p04= capture= output= fixture=false
if [ "${1:-}" = "--verify-output" ]; then
  output=${2:?}
  ruby -rjson -rdigest -e 'x=JSON.parse(File.read(ARGV[0])); h=x.delete("output_sha256"); abort "OUTPUT_EDITED" unless h == Digest::SHA256.hexdigest(JSON.generate(x));' "$output" || { echo "P08_CODEX_ACCEPTANCE_REFUSED:OUTPUT_EDITED" >&2; exit 72; }
  echo "P08_CODEX_ACCEPTANCE_OUTPUT_VERIFIED"; exit 0
fi
while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact) artifact=${2:?}; shift 2 ;;
    --artifact-sha256) artifact_sha=${2:?}; shift 2 ;;
    --p06) p06=${2:?}; shift 2 ;;
    --p04) p04=${2:?}; shift 2 ;;
    --capture) capture=${2:?}; shift 2 ;;
    --output) output=${2:?}; shift 2 ;;
    --fixture) fixture=true; shift ;;
    *) echo "P08_CODEX_ACCEPTANCE_REFUSED:USAGE" >&2; exit 64 ;;
  esac
done
[ -n "$artifact" ] && [ -n "$artifact_sha" ] && [ -n "$p06" ] && [ -n "$p04" ] && [ -n "$capture" ] && [ -n "$output" ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:USAGE" >&2; exit 64; }
[ -f "$artifact" ] && [ -f "$p06" ] && [ -f "$p04" ] && [ -f "$capture" ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:EVIDENCE_MISSING" >&2; exit 65; }
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd -P)
if [ "$fixture" = false ]; then
  [ "$p06" = "$root/reports/gates/p06/task-11.json" ] && [ "$p04" = "$root/reports/gates/p04/acceptance-evidence.json" ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:P06_PIN_MISMATCH" >&2; exit 65; }
  [ "$(shasum -a 256 "$p06" | awk '{print $1}')" = 89337194023b589e7a45f97c5122181c19c4b8e87f828516eb65743dd0ca19be ] && [ "$(shasum -a 256 "$p04" | awk '{print $1}')" = d9abc02b9cde8bf223f246477695b42582a0038745977207319d6f04d7a44265 ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:P06_PIN_MISMATCH" >&2; exit 65; }
  [ "$artifact_sha" = 19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37 ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:ARTIFACT_BYTES_UNAVAILABLE" >&2; exit 65; }
fi
printf '%s' "$artifact_sha" | grep -Eq '^[0-9a-f]{64}$' || { echo "P08_CODEX_ACCEPTANCE_REFUSED:INVALID_ARTIFACT_SHA256" >&2; exit 66; }
actual=$(shasum -a 256 "$artifact" | awk '{print $1}')
[ "$actual" = "$artifact_sha" ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:ARTIFACT_CHECKSUM_MISMATCH" >&2; exit 67; }
export TASKSEAL_ARTIFACT_SHA="$artifact_sha"

json_check() {
  ruby -rjson -e "$1" "$2" >/dev/null 2>&1
}
json_check 'x=JSON.parse(File.read(ARGV[0])); abort unless x["schema_version"] == "taskseal.codex-state-preservation-receipt.v1" && x["task"] == 11 && x["result"] == "accepted" && x.dig("tuple", "provider_id") == "codex" && x.dig("tuple", "artifact_digest") == ENV.fetch("TASKSEAL_ARTIFACT_SHA") && x["provider_launch"] == false && x["protected_state_unchanged"] == true' "$p06" || { echo "P08_CODEX_ACCEPTANCE_REFUSED:P06_ARTIFACT_OR_RECEIPT_MISMATCH" >&2; exit 68; }
json_check 'x=JSON.parse(File.read(ARGV[0])); abort unless x["schema_version"] == "taskseal.p04.acceptance-evidence.v1" && x.dig("census", "admitted").is_a?(Integer) && x.dig("census", "loaded_now") == 0 && x.dig("context_bytes", "full_bodies_at_startup") == 0' "$p04" || { echo "P08_CODEX_ACCEPTANCE_REFUSED:P04_CANARY_EVIDENCE_INVALID" >&2; exit 69; }

capture_values=$(ruby -rjson -e '
  x=JSON.parse(File.read(ARGV[0])); fail unless x["capture_mode"] == "DETERMINISTIC_FAKE";
  fail "NON_TERMINAL_EXECUTION" unless x["terminal"] == true;
  fail "UNSAFE_ARGS" unless x["argv"] == ["tseal", "codex", "--safe"];
  fail "UNEXPECTED_BODY_VISIBILITY" unless x.dig("catalog", "needed_name_visible") == true && x.dig("catalog", "unused_body_present") == false && x.dig("catalog", "invoked_body_available") == true;
  before=x["protected_before_sha256"]; after=x["protected_after_sha256"]; fail "PROTECTED_MUTATION" unless before.is_a?(String) && before.match?(/\A[0-9a-f]{64}\z/) && before == after;
  fail "CLEANUP_ABSENT" unless x.dig("cleanup", "exit") == "NOT_RUN" && x.dig("cleanup", "relaunch") == "NOT_RUN" && x.dig("cleanup", "uninstall") == "NOT_RUN";
  startup=x["startup_context_sha256"]; fail "STARTUP_CONTEXT_INVALID" unless startup.is_a?(String) && startup.match?(/\A[0-9a-f]{64}\z/);
  puts [startup, before].join(" ")
' "$capture" 2>&1) || {
  case "$capture_values" in
    *NON_TERMINAL_EXECUTION*|*UNSAFE_ARGS*|*UNEXPECTED_BODY_VISIBILITY*|*PROTECTED_MUTATION*|*CLEANUP_ABSENT*|*STARTUP_CONTEXT_INVALID*) echo "P08_CODEX_ACCEPTANCE_REFUSED:$capture_values" >&2 ;;
    *) echo "P08_CODEX_ACCEPTANCE_REFUSED:CAPTURE_INVALID" >&2 ;;
  esac
  exit 70
}
set -- $capture_values
startup_sha=$1 protected_sha=$2

output_dir=$(dirname "$output")
[ -d "$output_dir" ] || { echo "P08_CODEX_ACCEPTANCE_REFUSED:OUTPUT_PARENT_MISSING" >&2; exit 71; }
output_tmp=$(mktemp "$output_dir/.codex-alpha.XXXXXX")
trap 'rm -f "$output_tmp"' EXIT HUP INT TERM
TASKSEAL_ARTIFACT_SHA="$artifact_sha" TASKSEAL_STARTUP_SHA="$startup_sha" TASKSEAL_PROTECTED_SHA="$protected_sha" ruby -rjson -rdigest -e '
  payload={"schema_version"=>"taskseal.codex-clean-launch-acceptance.v1", "result"=>"PREPARED_NOT_QUALIFIED", "live_observation"=>"NOT_RUN", "reason"=>"OWNER_GATE_REQUIRED_PROVIDER_PROCESS", "capture_mode"=>"DETERMINISTIC_FAKE", "artifact_sha256"=>ENV.fetch("TASKSEAL_ARTIFACT_SHA"), "startup_context_sha256"=>ENV.fetch("TASKSEAL_STARTUP_SHA"), "catalog_census"=>{"needed_name_visible"=>true, "unused_body_present"=>false, "invoked_body_available"=>true}, "command"=>["tseal", "codex", "--safe"], "protected_state_sha256"=>ENV.fetch("TASKSEAL_PROTECTED_SHA"), "protected_mutation"=>false, "cleanup"=>{"exit"=>"NOT_RUN", "relaunch"=>"NOT_RUN", "uninstall"=>"NOT_RUN"}};
  payload["output_sha256"]=Digest::SHA256.hexdigest(JSON.generate(payload)); STDOUT.write(JSON.generate(payload), "\n")
' > "$output_tmp"
mv "$output_tmp" "$output"
trap - EXIT HUP INT TERM
echo "P08_CODEX_ACCEPTANCE_PREPARED_NOT_QUALIFIED"

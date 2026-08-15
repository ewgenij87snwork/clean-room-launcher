#!/bin/sh
# Artifact-bound preparation only. This harness deliberately cannot launch Codex.
set -eu

refuse() {
  echo "P08_CODEX_ACCEPTANCE_REFUSED:$1" >&2
  exit "${2:-70}"
}

if [ "${1:-}" = "--verify-output" ]; then
  output=${2:-}
  [ -n "$output" ] && [ "$#" -eq 2 ] && [ -f "$output" ] || refuse OUTPUT_EDITED 72
  ruby -rjson -rdigest -e '
    raw=File.binread(ARGV[0]); x=JSON.parse(raw); h=x.delete("output_sha256")
    abort unless h.is_a?(String) && h == Digest::SHA256.hexdigest(JSON.generate(x))
    x["output_sha256"]=h
    abort unless raw == JSON.generate(x) + "\n"
  ' "$output" >/dev/null 2>&1 || refuse OUTPUT_EDITED 72
  echo "P08_CODEX_ACCEPTANCE_OUTPUT_VERIFIED"
  exit 0
fi

artifact= artifact_sha= installed_tseal= p06= p04= capture= capture_sha= output=
test_only_fixture=false
while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact) artifact=${2:?}; shift 2 ;;
    --artifact-sha256) artifact_sha=${2:?}; shift 2 ;;
    --installed-tseal) installed_tseal=${2:?}; shift 2 ;;
    --p06) p06=${2:?}; shift 2 ;;
    --p04) p04=${2:?}; shift 2 ;;
    --capture) capture=${2:?}; shift 2 ;;
    --capture-sha256) capture_sha=${2:?}; shift 2 ;;
    --output) output=${2:?}; shift 2 ;;
    --test-only-fixture) test_only_fixture=true; shift ;;
    *) refuse USAGE 64 ;;
  esac
done
[ -n "$artifact" ] && [ -n "$artifact_sha" ] && [ -n "$installed_tseal" ] && [ -n "$p06" ] && [ -n "$p04" ] && [ -n "$capture" ] && [ -n "$capture_sha" ] && [ -n "$output" ] || refuse USAGE 64
[ -f "$artifact" ] && [ -f "$p06" ] && [ -f "$p04" ] && [ -f "$capture" ] || refuse EVIDENCE_MISSING 65

root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd -P)
schema=$root/schemas/release/codex-clean-launch-capture.schema.json
[ -f "$schema" ] || refuse CAPTURE_SCHEMA_INVALID 65

if [ "$test_only_fixture" = true ]; then
  [ "${TASKSEAL_TEST_ONLY_FIXTURE:-}" = 1 ] || refuse TEST_ONLY_FIXTURE_MARKER_REQUIRED 65
else
  [ "$p06" = "$root/reports/gates/p06/task-11.json" ] &&
    [ "$p04" = "$root/reports/gates/p04/acceptance-evidence.json" ] &&
    [ "$artifact_sha" = 19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37 ] || refuse PRODUCTION_ARTIFACT_PIN_REQUIRED 65
  [ "$(shasum -a 256 "$p06" | awk '{print $1}')" = 89337194023b589e7a45f97c5122181c19c4b8e87f828516eb65743dd0ca19be ] || refuse P06_PIN_MISMATCH 65
  [ "$(shasum -a 256 "$p04" | awk '{print $1}')" = d9abc02b9cde8bf223f246477695b42582a0038745977207319d6f04d7a44265 ] || refuse P04_PIN_MISMATCH 65
fi

printf '%s' "$artifact_sha" | grep -Eq '^[0-9a-f]{64}$' || refuse INVALID_ARTIFACT_SHA256 66
actual_artifact_sha=$(shasum -a 256 "$artifact" | awk '{print $1}')
[ "$actual_artifact_sha" = "$artifact_sha" ] || refuse ARTIFACT_CHECKSUM_MISMATCH 67
printf '%s' "$capture_sha" | grep -Eq '^[0-9a-f]{64}$' || refuse INVALID_CAPTURE_SHA256 67
[ "$(shasum -a 256 "$capture" | awk '{print $1}')" = "$capture_sha" ] || refuse CAPTURE_CHECKSUM_MISMATCH 67

[ -f "$installed_tseal" ] && [ -x "$installed_tseal" ] || refuse INSTALLED_TSEAL_MISSING 67
installed_real=$(ruby -e 'puts File.realpath(ARGV[0])' "$installed_tseal" 2>/dev/null) || refuse INSTALLED_TSEAL_MISSING 67
case "$installed_real" in
  "$root"|"$root"/*) refuse DEVELOPER_CHECKOUT_REFUSED 67 ;;
esac
if git -C "$(dirname "$installed_real")" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  refuse DEVELOPER_CHECKOUT_REFUSED 67
fi

python3 "$root/packaging/verify-artifact.py" "$artifact" >/dev/null 2>&1 || refuse ARTIFACT_PROVENANCE_INVALID 67
archive_tseal_sha=$(python3 - "$artifact" <<'PY'
import hashlib, sys, tarfile
with tarfile.open(sys.argv[1], "r:gz") as archive:
    members = [m for m in archive.getmembers() if m.name.endswith("/bin/tseal")]
    if len(members) != 1 or not members[0].isfile(): raise SystemExit(1)
    print(hashlib.sha256(archive.extractfile(members[0]).read()).hexdigest())
PY
) || refuse ARTIFACT_PROVENANCE_INVALID 67
installed_sha=$(shasum -a 256 "$installed_real" | awk '{print $1}')
[ "$archive_tseal_sha" = "$installed_sha" ] || refuse INSTALLED_ARTIFACT_MISMATCH 67

export TASKSEAL_ARTIFACT_SHA=$artifact_sha TASKSEAL_INSTALLED_SHA=$installed_sha
ruby -rjson -e '
  x=JSON.parse(File.read(ARGV[0])); p06=JSON.parse(File.read(ARGV[1])); p04=JSON.parse(File.read(ARGV[2]))
  abort unless p06["schema_version"] == "taskseal.codex-state-preservation-receipt.v1" && p06["task"] == 11 && p06["result"] == "accepted" && p06.dig("tuple", "provider_id") == "codex" && p06.dig("tuple", "artifact_digest") == ENV.fetch("TASKSEAL_ARTIFACT_SHA") && p06["provider_launch"] == false && p06["protected_state_unchanged"] == true
' "$capture" "$p06" "$p04" >/dev/null 2>&1 || refuse P06_ARTIFACT_OR_RECEIPT_MISMATCH 68
ruby -rjson -e '
  p04=JSON.parse(File.read(ARGV[0])); abort unless p04["schema_version"] == "taskseal.p04.acceptance-evidence.v1" && p04.dig("census", "admitted").is_a?(Integer) && p04.dig("census", "loaded_now") == 0 && p04.dig("context_bytes", "full_bodies_at_startup") == 0 && p04.dig("inventory_body_digests", "a") == "5f07933adc98d602398359e57783d348658eaa60fa7907da263b53291f8184b1" && p04.dig("inventory_body_digests", "b") == "32eea1ee666999e53a16feb944d355ef49bcff8ec379a3e02ee690172752f55e"
' "$p04" >/dev/null 2>&1 || refuse P04_CANARY_EVIDENCE_INVALID 69

capture_error=$(ruby -rjson -e '
  def exact(x, keys, reason="CAPTURE_SCHEMA_INVALID")
    raise reason unless x.is_a?(Hash) && x.keys.sort == keys.sort
  end
  def sha?(x)
    x.is_a?(String) && x.match?(/\A[0-9a-f]{64}\z/)
  end
  x=JSON.parse(File.read(ARGV[0])); p04=JSON.parse(File.read(ARGV[1]))
  exact(x, %w[schema_version capture_mode promotion_eligible live_observation terminal argv artifact_provenance canaries startup catalog protected_state cleanup])
  raise "CAPTURE_SCHEMA_INVALID" unless x["schema_version"] == "taskseal.codex-clean-launch-capture.v1"
  raise "LIVE_CAPTURE_REQUIRES_OWNER_GATE" unless x["capture_mode"] == "TEST_ONLY_FIXTURE" && x["promotion_eligible"] == false && x["live_observation"] == "NOT_RUN"
  raise "NON_TERMINAL_EXECUTION" unless x["terminal"] == true
  raise "UNSAFE_ARGS" unless x["argv"] == ["tseal", "codex", "--"]
  provenance=x["artifact_provenance"]
  exact(provenance, %w[install_source archive_sha256 installed_tseal_sha256 installed_path_class developer_checkout])
  raise "ARTIFACT_PROVENANCE_INVALID" unless provenance == {"install_source"=>"RELEASE_ARCHIVE", "archive_sha256"=>ENV.fetch("TASKSEAL_ARTIFACT_SHA"), "installed_tseal_sha256"=>ENV.fetch("TASKSEAL_INSTALLED_SHA"), "installed_path_class"=>"EXTERNAL_INSTALL_PREFIX", "developer_checkout"=>false}
  canaries=x["canaries"]; exact(canaries, %w[needed unused protected], "CANARY_EVIDENCE_INVALID")
  expected={"needed"=>["a", p04.dig("inventory_body_digests", "a")], "unused"=>["b", p04.dig("inventory_body_digests", "b")], "protected"=>["b", p04.dig("inventory_body_digests", "b")]}
  expected.each do |name, pair|
    value=canaries[name]; exact(value, %w[id body_sha256], "CANARY_EVIDENCE_INVALID")
    raise "CANARY_EVIDENCE_INVALID" unless value["id"] == pair[0] && value["body_sha256"] == pair[1] && sha?(value["body_sha256"])
  end
  startup=x["startup"]; exact(startup, %w[context_sha256 context_bytes compact_screen clean_startup full_bodies_visible])
  raise "STARTUP_CONTEXT_INVALID" unless sha?(startup["context_sha256"]) && startup["context_bytes"] == p04.dig("context_bytes", "startup_level_a") && startup["compact_screen"] == true && startup["clean_startup"] == true && startup["full_bodies_visible"] == 0
  catalog=x["catalog"]; exact(catalog, %w[admitted loaded_now needed_name_visible unused_body_present invoked_body_available])
  raise "UNEXPECTED_BODY_VISIBILITY" unless catalog == {"admitted"=>p04.dig("census", "admitted"), "loaded_now"=>0, "needed_name_visible"=>true, "unused_body_present"=>false, "invoked_body_available"=>true}
  protected=x["protected_state"]; exact(protected, %w[before_sha256 after_sha256 unchanged])
  raise "PROTECTED_MUTATION" unless sha?(protected["before_sha256"]) && protected["before_sha256"] == protected["after_sha256"] && protected["unchanged"] == true
  cleanup=x["cleanup"]; exact(cleanup, %w[exit relaunch uninstall])
  raise "CLEANUP_ABSENT" unless cleanup == {"exit"=>"NOT_RUN", "relaunch"=>"NOT_RUN", "uninstall"=>"NOT_RUN"}
' "$capture" "$p04" 2>&1) || true
[ -z "$capture_error" ] || refuse "$capture_error" 70

output_dir=$(dirname "$output")
[ -d "$output_dir" ] || refuse OUTPUT_PARENT_MISSING 71
output_tmp=$(mktemp "$output_dir/.codex-alpha.XXXXXX")
trap 'rm -f "$output_tmp"' EXIT HUP INT TERM
schema_sha=$(shasum -a 256 "$schema" | awk '{print $1}')
export TASKSEAL_CAPTURE_SHA=$capture_sha TASKSEAL_SCHEMA_SHA=$schema_sha
ruby -rjson -rdigest -e '
  capture=JSON.parse(File.read(ARGV[0]))
  payload={
    "schema_version"=>"taskseal.codex-clean-launch-acceptance.v1",
    "result"=>"PREPARED_NOT_QUALIFIED",
    "live_observation"=>"NOT_RUN",
    "reason"=>"OWNER_GATE_REQUIRED_PROVIDER_PROCESS",
    "capture_mode"=>"TEST_ONLY_FIXTURE",
    "fixture_only"=>true,
    "promotion_eligible"=>false,
    "artifact_sha256"=>ENV.fetch("TASKSEAL_ARTIFACT_SHA"),
    "installed_artifact_verified"=>true,
    "installed_tseal_sha256"=>ENV.fetch("TASKSEAL_INSTALLED_SHA"),
    "capture_sha256"=>ENV.fetch("TASKSEAL_CAPTURE_SHA"),
    "capture_schema_sha256"=>ENV.fetch("TASKSEAL_SCHEMA_SHA"),
    "command"=>capture["argv"],
    "canaries"=>capture["canaries"],
    "startup"=>capture["startup"],
    "catalog"=>capture["catalog"],
    "protected_state_sha256"=>capture.dig("protected_state", "before_sha256"),
    "protected_mutation"=>false,
    "cleanup"=>capture["cleanup"]
  }
  payload["output_sha256"]=Digest::SHA256.hexdigest(JSON.generate(payload))
  STDOUT.write(JSON.generate(payload), "\n")
' "$capture" >"$output_tmp"
mv "$output_tmp" "$output"
trap - EXIT HUP INT TERM
echo "P08_CODEX_ACCEPTANCE_PREPARED_NOT_QUALIFIED"

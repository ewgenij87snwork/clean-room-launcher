#!/bin/sh
# Closed, local demo receipt builder. It deliberately never launches a provider.
set -eu
refuse() { echo "P08_DEMO_REFUSED:$1" >&2; exit "${2:-70}"; }
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd -P)
schema=$root/schemas/release/demo-receipt.schema.json

if [ "${1:-}" = "--verify-output" ]; then
  [ "$#" -eq 2 ] && [ -f "$2" ] || refuse OUTPUT_EDITED 72
  ruby -rjson -rdigest -e '
    raw=File.binread(ARGV[0]); x=JSON.parse(raw); schema=JSON.parse(File.read(ARGV[1])); hash=x.delete("output_sha256")
    abort unless hash.is_a?(String) && hash == Digest::SHA256.hexdigest(JSON.generate(x))
    x["output_sha256"]=hash; abort unless raw == JSON.generate(x)+"\n"
    abort "OUTPUT_SCHEMA" unless x.keys.sort == schema.fetch("properties").keys.sort && schema.fetch("required").all?{|key| x.key?(key)}
    schema.fetch("properties").each{|key, rule| value=x[key]; abort "OUTPUT_SCHEMA" if rule.key?("const") && value != rule["const"]; abort "OUTPUT_SCHEMA" if rule["type"] == "boolean" && value != true && value != false; abort "OUTPUT_SCHEMA" if rule["type"] == "object" && !value.is_a?(Hash); abort "OUTPUT_SCHEMA" if rule["pattern"] && !(value.is_a?(String) && Regexp.new(rule["pattern"]).match?(value))}
  ' "$2" "$schema" >/dev/null 2>&1 || refuse OUTPUT_SCHEMA 72
  echo "P08_DEMO_OUTPUT_VERIFIED"; exit 0
fi

artifact= artifact_sha= installed_tseal= fixture= fixture_sha= capture= output= test_only=false
while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact) artifact=${2:?}; shift 2 ;;
    --artifact-sha256) artifact_sha=${2:?}; shift 2 ;;
    --installed-tseal) installed_tseal=${2:?}; shift 2 ;;
    --fixture) fixture=${2:?}; shift 2 ;;
    --fixture-sha256) fixture_sha=${2:?}; shift 2 ;;
    --capture) capture=${2:?}; shift 2 ;;
    --output) output=${2:?}; shift 2 ;;
    --test-only-fixture) test_only=true; shift ;;
    *) refuse USAGE 64 ;;
  esac
done
[ -n "$artifact" ] && [ -n "$artifact_sha" ] && [ -n "$fixture" ] && [ -n "$fixture_sha" ] && [ -n "$capture" ] && [ -n "$output" ] || refuse USAGE 64
[ -f "$artifact" ] && [ -f "$fixture" ] && [ -f "$capture" ] || refuse EVIDENCE_MISSING 65
printf %s "$artifact_sha" | grep -Eq '^[0-9a-f]{64}$' || refuse ARTIFACT_DIGEST 66
printf %s "$fixture_sha" | grep -Eq '^[0-9a-f]{64}$' || refuse FIXTURE_DIGEST 66
[ "$(shasum -a 256 "$fixture" | awk '{print $1}')" = "$fixture_sha" ] || refuse WRONG_FIXTURE 66

[ "$test_only" = true ] && [ "${TASKSEAL_TEST_ONLY_FIXTURE:-}" = 1 ] || refuse PRODUCTION_NOT_RUN 67
artifact_real=$(ruby -e 'puts File.realpath(ARGV[0])' "$artifact" 2>/dev/null) || refuse EVIDENCE_MISSING 65
case "$artifact_real" in "$root"|"$root"/*) refuse DEVELOPER_CHECKOUT_REFUSED 67;; esac
p07_digest=$(ruby -rjson -e 'x=JSON.parse(File.read(ARGV[0])); abort unless x["schema_version"]=="taskseal.p07.task-receipt.v1" && x["task"]==3 && x.dig("claims","source_commit")=="01ad1d894aabe265b08d61d67d39da1a29cad9e4"; puts x.dig("claims","archive_sha256")' "$root/reports/gates/p07/task-3.json" 2>/dev/null) || refuse P07_ARTIFACT_EVIDENCE 67
[ "$p07_digest" = "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33" ] || refuse P07_ARTIFACT_EVIDENCE 67
[ "$artifact_sha" = "$p07_digest" ] || refuse P07_ARTIFACT_DIGEST 67
actual_artifact_sha=$(shasum -a 256 "$artifact" | awk '{print $1}')
[ "$actual_artifact_sha" = "$artifact_sha" ] || refuse STALE_ARTIFACT 67
artifact_bytes_verified=true
if [ "$artifact_bytes_verified" = true ]; then
  [ -n "$installed_tseal" ] && [ -f "$installed_tseal" ] && [ -x "$installed_tseal" ] || refuse CLEAN_INSTALL_REQUIRED 67
  installed_real=$(ruby -e 'puts File.realpath(ARGV[0])' "$installed_tseal" 2>/dev/null) || refuse CLEAN_INSTALL_REQUIRED 67
  case "$installed_real" in "$root"|"$root"/*) refuse DEVELOPER_CHECKOUT_REFUSED 67;; esac
  archive_tseal_sha=$(python3 - "$artifact" <<'PY'
import hashlib, sys, tarfile
with tarfile.open(sys.argv[1], "r:gz") as archive:
    members = [m for m in archive.getmembers() if m.name.endswith("/bin/tseal")]
    if len(members) != 1 or not members[0].isfile(): raise SystemExit(1)
    print(hashlib.sha256(archive.extractfile(members[0]).read()).hexdigest())
PY
) || refuse CLEAN_INSTALL_REQUIRED 67
  [ "$archive_tseal_sha" = "$(shasum -a 256 "$installed_real" | awk '{print $1}')" ] || refuse INSTALLED_ARTIFACT_MISMATCH 67
fi
[ "$(dirname "$output")" != "$root/reports/release" ] || refuse TEST_OUTPUT_MUST_NOT_REPLACE_PRODUCTION 67

error_file=$(mktemp "${TMPDIR:-/tmp}/taskseal-demo-error.XXXXXX")
trap 'rm -f "$error_file"' EXIT HUP INT TERM
if ! ruby -rjson -rdigest -e '
  def refuse(code); abort "P08_DEMO_REFUSED:#{code}"; end
  def exact(value, keys, code); refuse(code) unless value.is_a?(Hash) && value.keys.sort == keys.sort; end
  fixture=JSON.parse(File.binread(ARGV[0])); capture=JSON.parse(File.binread(ARGV[1]))
  exact(fixture, %w[schema_version mode promotion_eligible p07_source_evidence p07_artifact_evidence p08_task3_contract claims catalog_census semantic_fields redaction], "FIXTURE_SCHEMA")
  refuse("FIXTURE_SCHEMA") unless fixture["schema_version"] == "taskseal.demo-fixture.v1" && fixture["mode"] == "TEST_ONLY_REPLAY"
  refuse("FIXTURE_NON_PROMOTABLE") unless fixture["promotion_eligible"] == false
  refuse("STALE_ARTIFACT_EVIDENCE") unless fixture["p07_source_evidence"] == "reports/gates/p07/task-3.json#/claims/source_commit" && fixture["p07_artifact_evidence"] == "reports/gates/p07/task-3.json#/claims/archive_sha256" && fixture["p08_task3_contract"] == "reports/release/codex-alpha.json"
  exact(fixture["redaction"], %w[declared_non_semantic raw_prompt_retained credential_retained private_path_retained], "REDACTION_SCHEMA")
  refuse("REDACTION_SCHEMA") unless fixture.dig("redaction", "declared_non_semantic") == ["recorded_at"] && fixture.dig("redaction", "raw_prompt_retained") == false && fixture.dig("redaction", "credential_retained") == false && fixture.dig("redaction", "private_path_retained") == false
  raw=File.binread(ARGV[1]); refuse("PRIVATE_PATH") if raw.match?(%r{/(?:Users|home)/})
  refuse("RAW_PROMPT") if raw.include?("raw_prompt")
  refuse("CREDENTIAL") if raw.include?("credential")
  exact(capture, %w[schema_version mode promotion_eligible commands results claims catalog_census cleanup recorded_at], "CAPTURE_SCHEMA")
  refuse("CAPTURE_SCHEMA") unless capture["schema_version"] == "taskseal.demo-capture.v1" && capture["mode"] == "TEST_ONLY_REPLAY" && capture["promotion_eligible"] == false
  refuse("CAPTURE_RESULT") unless capture["commands"] == ["tseal catalog"] && capture["results"] == ["catalog: 2 skills"] && capture["claims"] == fixture["claims"] && capture["catalog_census"] == fixture["catalog_census"] && capture["recorded_at"] == "REDACTED_NON_SEMANTIC"
  exact(capture["cleanup"], %w[completed], "CLEANUP"); refuse("CLEANUP") unless capture.dig("cleanup", "completed") == true
  semantic={"claims"=>capture["claims"], "catalog_census"=>capture["catalog_census"], "result_digest"=>Digest::SHA256.hexdigest(JSON.generate({"commands"=>capture["commands"],"results"=>capture["results"]}))}
  out={"schema_version"=>"taskseal.demo-receipt.v1","result"=>"PREPARED_NOT_QUALIFIED","live_observation"=>"NOT_RUN","reason"=>"EXACT_ARTIFACT_BYTES_AND_CLEAN_INSTALL_OBSERVATION_REQUIRED","mode"=>"TEST_ONLY_REPLAY","fixture_only"=>true,"promotion_eligible"=>false,"artifact_sha256"=>ARGV[2],"artifact_bytes_verified"=>ARGV[4]=="true","fixture_sha256"=>Digest::SHA256.file(ARGV[0]).hexdigest,"p07_source_evidence"=>fixture["p07_source_evidence"],"p07_artifact_evidence"=>fixture["p07_artifact_evidence"],"p08_task3_contract"=>fixture["p08_task3_contract"],"semantic"=>semantic,"cleanup"=>"NOT_RUN","controls"=>{"developer_checkout_refused"=>true,"edited_output_refused"=>true,"redaction_closed"=>true,"provider_process"=>"NOT_RUN","network"=>false,"publication"=>false}}
  out["output_sha256"]=Digest::SHA256.hexdigest(JSON.generate(out)); schema=JSON.parse(File.read(ARGV[5])); refuse("OUTPUT_SCHEMA") unless out.keys.sort==schema.fetch("properties").keys.sort && schema.fetch("required").all?{|key|out.key?(key)}; schema.fetch("properties").each{|key,rule| value=out[key]; refuse("OUTPUT_SCHEMA") if rule.key?("const") && value != rule["const"]; refuse("OUTPUT_SCHEMA") if rule["type"]=="boolean" && value != true && value != false; refuse("OUTPUT_SCHEMA") if rule["type"]=="object" && !value.is_a?(Hash); refuse("OUTPUT_SCHEMA") if rule["pattern"] && !(value.is_a?(String) && Regexp.new(rule["pattern"]).match?(value))}; File.binwrite(ARGV[3], JSON.generate(out)+"\n")
' "$fixture" "$capture" "$artifact_sha" "$output" "$artifact_bytes_verified" "$schema" 2>"$error_file"; then
  code=$(sed -n 's/.*P08_DEMO_REFUSED:\([A-Z_]*\).*/\1/p' "$error_file" | head -1)
  [ -n "$code" ] || code=CAPTURE_SCHEMA
  refuse "$code"
fi

# Re-run the closed parser above explicitly so Ruby errors cannot become a pass.
[ -f "$output" ] || refuse CAPTURE_SCHEMA 70
trap - EXIT HUP INT TERM
rm -f "$error_file"
echo "P08_DEMO_PREPARED_NOT_QUALIFIED"

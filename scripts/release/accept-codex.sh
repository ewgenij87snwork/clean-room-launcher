#!/bin/sh
# Prepare a clroom/Codex acceptance candidate. It never starts Codex itself.
set -eu
refuse() { echo "P08_CLROOM_CODEX_REFUSED:$1" >&2; exit 1; }

artifact= installed= p06= output=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact) artifact=${2:?}; shift 2 ;;
    --installed-clroom) installed=${2:?}; shift 2 ;;
    --p06-result) p06=${2:?}; shift 2 ;;
    --output) output=${2:?}; shift 2 ;;
    *) refuse USAGE ;;
  esac
done
[ -f "$artifact" ] && [ -f "$installed" ] && [ -x "$installed" ] && [ -f "$p06" ] && [ -n "$output" ] || refuse EVIDENCE_MISSING
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd -P)
case "$(ruby -e 'puts File.realpath(ARGV[0])' "$installed")" in "$root"|"$root"/*) refuse DEVELOPER_CHECKOUT ;; esac
python3 "$root/packaging/verify-artifact.py" "$artifact" >/dev/null || refuse ARTIFACT_INVALID
archive_sha=$(shasum -a 256 "$artifact" | awk '{print $1}')
archive_clroom_sha=$(python3 - "$artifact" <<'PY'
import hashlib, sys, tarfile
with tarfile.open(sys.argv[1], "r:gz") as a:
    found=[m for m in a.getmembers() if m.name.endswith("/bin/clroom") and m.isfile()]
    if len(found)!=1: raise SystemExit(1)
    print(hashlib.sha256(a.extractfile(found[0]).read()).hexdigest())
PY
) || refuse ARTIFACT_LAYOUT
[ "$archive_clroom_sha" = "$(shasum -a 256 "$installed" | awk '{print $1}')" ] || refuse INSTALLED_ARTIFACT_MISMATCH
ruby -rjson -e '
  x=JSON.parse(File.read(ARGV[0])); abort unless x["schema_version"]=="clroom.macos-isolated-launch.result.v1" && x["process_count"]==1 && x["real_help_exit_class"]=="HELP_EXIT_0" && x["raw_retained"]==false
' "$p06" >/dev/null 2>&1 || refuse P06_BOUNDARY_UNPROVEN
ruby -rjson -rdigest -e '
  v={"schema_version"=>"clroom.codex-acceptance.v1","result"=>"PREPARED_NOT_QUALIFIED","reason"=>"OWNER_GATE_REQUIRED_PROVIDER_PROCESS","artifact_sha256"=>ARGV[0],"installed_clroom_verified"=>true,"p06_result_sha256"=>Digest::SHA256.file(ARGV[1]).hexdigest,"provider_process"=>"NOT_RUN"}; v["output_sha256"]=Digest::SHA256.hexdigest(JSON.generate(v)); File.write(ARGV[2],JSON.generate(v)+"\n")
' "$archive_sha" "$p06" "$output"
echo P08_CLROOM_CODEX_PREPARED_NOT_QUALIFIED

#!/bin/sh
# Local release-preparation gate. It never launches a provider or publishes.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd -P)
cd "$root"

ruby -rjson -rdigest -e '
  candidate=JSON.parse(File.read("reports/release/candidate.json"))
  abort unless candidate["schema_version"] == "clroom.release-dossier.v1"
  abort unless candidate["candidate_commit"] == "e0b043b6cd1336eba8d92d5ece4056f7251b0812"
  abort unless candidate["qualification"] == "NOT_QUALIFIED"
  abort unless candidate.dig("artifact", "binary") == "clroom"
  abort unless candidate.dig("artifact", "package") == "clean-room-launcher"
  abort unless candidate.fetch("known_blockers") == %w[PUBLICATION_NOT_AUTHORIZED NAMESPACE_OWNERSHIP_UNPROVEN EXTERNAL_INSTALL_NOT_RUN]

  prepared=JSON.parse(File.read("reports/release/clroom-codex-prepared.json"))
  hash=prepared.delete("output_sha256")
  abort unless hash == Digest::SHA256.hexdigest(JSON.generate(prepared))
  abort unless prepared["result"] == "PREPARED_NOT_QUALIFIED"
  abort unless prepared["provider_process"] == "NOT_RUN"

  expected={
    "reports/gates/p06/successors/clroom-macos-isolated-launch-v1/result.json"=>"15ba7ca41420c855475aab6ff4193914bdaad6dc945003cc32ea474a251755bf",
    "reports/gates/p07/successors/clroom-packaging-v1/task-2.json"=>"62df830ef0c52e373fa112334e3f2f50bcc8434cdecc74a84c3acdd5004a4568",
    "reports/release/clroom-codex-live.json"=>"f96de7724aa5b187b57d689565cda3575c234cf9f58f562270c88a2bfc55ff8b"
  }
  expected.each { |path, digest| abort unless Digest::SHA256.file(path).hexdigest == digest }
' || { echo P08_CLROOM_LOCAL_REFUSED:EVIDENCE >&2; exit 1; }

public_files="README.md SECURITY.md docs/demo.md docs/install.md docs/providers.md docs/limitations.md docs/upgrade-rollback.md"
if grep -Eiw '\b(tseal|croom)\b' $public_files >/dev/null; then
  echo P08_CLROOM_LOCAL_REFUSED:STALE_PUBLIC_COMMAND >&2
  exit 1
fi
if grep -E 'production-ready|supports all providers|supports all operating systems' $public_files >/dev/null; then
  echo P08_CLROOM_LOCAL_REFUSED:UNSUPPORTED_CLAIM >&2
  exit 1
fi

echo P08_CLROOM_LOCAL_RELEASE_PREPARED_NOT_QUALIFIED

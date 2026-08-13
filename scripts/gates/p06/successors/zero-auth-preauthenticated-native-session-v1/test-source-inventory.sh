#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
scanner="$root/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory.rb"

if test ! -f "$scanner"; then
  printf '%s\n' P06_ZERO_AUTH_RED_SOURCE_INVENTORY_MISSING
  exit 1
fi

expected_refusal='#!/bin/sh
printf '\''%s\n'\'' HISTORICAL_ONLY_REFUSED >&2
exit 78
'
historical_paths='scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh
scripts/gates/p06/successors/observation-capability-v1/probe-local.sh
scripts/gates/p06/successors/observation-capability-v1/verify.sh
scripts/gates/p06/t8-native-observe-once.sh'

printf '%s\n' "$historical_paths" | while IFS= read -r path; do
  test "$(git -C "$root" ls-files -s -- "$path" | awk '{print $1}')" = 100644
  test "$(cat "$root/$path")" = "$(printf '%s' "$expected_refusal")"

  set +e
  actual=$(env -i PATH=/usr/bin:/bin /bin/sh "$root/$path" ignored 2>&1)
  status=$?
  set -e
  test "$status" = 78
  test "$actual" = HISTORICAL_ONLY_REFUSED
done
test "$(git -C "$root" rev-parse 2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6:scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh)" = 8284ce0505357d869ab86ffef08a2d8bdd4d6b11
test "$(git -C "$root" show 2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6:scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh | shasum -a 256 | awk '{print $1}')" = d340b6a04510b89f279f354aad3fd94ef221f1b4a2847262868c2dc8592baadc
test "$(git -C "$root" rev-parse 3e40a4b688bb489b5cbc9a8ec8e32a2be032c26e:scripts/gates/p06/successors/observation-capability-v1/probe-local.sh)" = 987a2f30d810020283a4631d920e29c08d5b4b50
test "$(git -C "$root" show 3e40a4b688bb489b5cbc9a8ec8e32a2be032c26e:scripts/gates/p06/successors/observation-capability-v1/probe-local.sh | shasum -a 256 | awk '{print $1}')" = d741ad3860c572e2c086a824efdc727a16a56feff840677211064aaba556d15f
test "$(git -C "$root" rev-parse 3e40a4b688bb489b5cbc9a8ec8e32a2be032c26e:scripts/gates/p06/successors/observation-capability-v1/verify.sh)" = aaf4481390b106f9114e3713197c18f1c3891cc5
test "$(git -C "$root" show 3e40a4b688bb489b5cbc9a8ec8e32a2be032c26e:scripts/gates/p06/successors/observation-capability-v1/verify.sh | shasum -a 256 | awk '{print $1}')" = aecb73d396a9f745b1fd98d9fbdbed214bed2b5e85744df261882d6d46af5216
test "$(git -C "$root" rev-parse 2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6:scripts/gates/p06/t8-native-observe-once.sh)" = d84262dfecf5ed3e00f2f6bf925bfaa65ec9b119
test "$(git -C "$root" show 2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6:scripts/gates/p06/t8-native-observe-once.sh | shasum -a 256 | awk '{print $1}')" = 21f565399615c0d9c5dd7bfc6da7e521cda77b3f554a0be7f705003de48fb89f

test "$(ruby "$scanner" "$root")" = P06_ZERO_AUTH_SOURCE_INVENTORY_PASS

truth="$root/reports/contracts/provider-capability-truth.json"
if ! jq -e '
  .schema_version == "taskseal.provider-capability-truth.v2" and
  .authority == {
    classification:"HISTORICAL_ONLY_NON_AUTHORITATIVE",
    superseded_by:["OD-10","AUTH-01"],
    current_auth_fingerprint_claim:"ABSENT",
    historical_auth_fingerprint_claim:{
      authority:"NON_AUTHORITATIVE",
      source_commit:"2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6",
      source_blob_oid:"d35eecaf1cc14ff0007b0ce584109f6cc8a4c237",
      source_sha256:"fc79f7813cea234ca46fd20dcd0f56c777f1aeffaa465168dbd8643be118f202",
      json_pointers:[
        "/persistent_subjects/codex_auth_sha256_before",
        "/persistent_subjects/codex_auth_sha256_after"
      ]
    },
    rule:"The predecessor Git object preserves provenance only. TaskSeal does not read, retain, compare, or authorize provider access from an authentication fingerprint."
  } and
  (.persistent_subjects | has("codex_auth_sha256_before") | not) and
  (.persistent_subjects | has("codex_auth_sha256_after") | not)
' "$truth" >/dev/null; then
  printf '%s\n' P06_ZERO_AUTH_PROVIDER_TRUTH_SUPERSESSION_REQUIRED
  exit 1
fi
test "$(git -C "$root" rev-parse 2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6:reports/contracts/provider-capability-truth.json)" = d35eecaf1cc14ff0007b0ce584109f6cc8a4c237
test "$(git -C "$root" show 2d29ecfef073c5ad1a04d3acb96a6ccb48261ce6:reports/contracts/provider-capability-truth.json | shasum -a 256 | awk '{print $1}')" = fc79f7813cea234ca46fd20dcd0f56c777f1aeffaa465168dbd8643be118f202

scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-source.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
fixture="$scratch/repository"
mkdir -p "$fixture/scripts"
git -C "$fixture" init -q

write_current() {
  body=$1
  {
    printf '%s\n' '#!/bin/sh' 'set -eu'
    printf '%s\n' "$body"
  } >"$fixture/scripts/current.sh"
  chmod 755 "$fixture/scripts/current.sh"
  git -C "$fixture" add --chmod=+x scripts/current.sh
}

expect_refusal() {
  label=$1
  marker=$2
  body=$3
  write_current "$body"
  set +e
  actual=$(ruby "$scanner" "$fixture" 2>&1)
  status=$?
  set -e
  test "$status" = 1
  test "$actual" = "P06_ZERO_AUTH_SOURCE_REFUSAL:$marker:scripts/current.sh" || {
    printf '%s\n' "P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:$label:$status:$actual"
    exit 1
  }
}

write_current 'printf '\''%s\n'\'' LOCAL_ONLY'
test "$(ruby "$scanner" "$fixture")" = P06_ZERO_AUTH_SOURCE_INVENTORY_PASS

expect_refusal auth_file AUTH_FILE 'cat "$HOME/.codex/auth.json"'
expect_refusal credential_extraction CREDENTIAL_EXTRACTION '/usr/bin/plutil -extract "$FIELD" raw "$SOURCE"'
expect_refusal credential_copy CREDENTIAL_COPY 'cp "$CREDENTIAL_SOURCE" "$TMPDIR/copied"'
expect_refusal provider_login PROVIDER_LOGIN 'codex login'
expect_refusal browser_auth BROWSER_AUTH 'open "https://provider.invalid/oauth/device"'
expect_refusal api_key_input TOKEN_INPUT 'read -r API_KEY'
expect_refusal token_flag_input TOKEN_INPUT 'codex --with-access-token "$ACCESS_TOKEN"'
write_current 'printf '\''%s\n'\'' LOCAL_ONLY'

write_release_source() {
  body=$1
  mkdir -p "$fixture/src/cli"
  printf '%s\n' "$body" >"$fixture/src/cli/renamed.rs"
  chmod 644 "$fixture/src/cli/renamed.rs"
  git -C "$fixture" add src/cli/renamed.rs
}

expect_release_refusal() {
  label=$1
  marker=$2
  body=$3
  write_release_source "$body"
  set +e
  actual=$(ruby "$scanner" "$fixture" 2>&1)
  status=$?
  set -e
  if test "$status" != 1 || test "$actual" != "P06_ZERO_AUTH_SOURCE_REFUSAL:$marker:src/cli/renamed.rs"; then
    printf '%s\n' "P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:$label:$status:$actual"
    exit 1
  fi
}

expect_release_refusal rust_auth_file AUTH_FILE \
  'fn read(home: &std::path::Path) { let _ = std::fs::read(home.join("auth.json")); }'
expect_release_refusal rust_multiline_extraction CREDENTIAL_EXTRACTION \
  'fn extract(source: &str) { let _ = std::process::Command::new("plutil")
    .arg("-extract").arg("access_token").arg(source).output(); }'
expect_release_refusal rust_multiline_copy CREDENTIAL_COPY \
  'fn copy(credential_source: &str, destination: &str) { let _ = std::fs::copy(
    credential_source,
    destination); }'
expect_release_refusal rust_dynamic_provider_login PROVIDER_LOGIN \
  'fn run(binary: &str) { let subcommand = "login";
    let _ = std::process::Command::new(binary).arg(subcommand).status(); }'
expect_release_refusal rust_unknown_provider_login PROVIDER_LOGIN \
  'fn run() { let _ = std::process::Command::new("other-provider")
    .arg("login").status(); }'
expect_release_refusal rust_browser_device_flow BROWSER_AUTH \
  'fn open() { let _ = std::process::Command::new("xdg-open")
    .arg("https://provider.invalid/oauth/device-flow").spawn(); }'
expect_release_refusal rust_provider_prefixed_key TOKEN_INPUT \
  'fn key() { let _ = std::env::var("OPENAI_API_KEY"); }'
git -C "$fixture" rm -q -f src/cli/renamed.rs

# Break caught: a tracked executable under scripts/gates is still runnable source;
# its location must never exempt a provider-login process birth.
mkdir -p "$fixture/scripts/gates/p06/successors/renamed-capability"
printf '%s\n' '#!/bin/sh' 'set -eu' 'clean_env "$TOOL" login --help' >"$fixture/scripts/gates/p06/successors/renamed-capability/probe-local.sh"
chmod 755 "$fixture/scripts/gates/p06/successors/renamed-capability/probe-local.sh"
git -C "$fixture" add --chmod=+x scripts/gates/p06/successors/renamed-capability/probe-local.sh
set +e
actual=$(ruby "$scanner" "$fixture" 2>&1)
status=$?
set -e
if test "$status" != 1 || test "$actual" != 'P06_ZERO_AUTH_SOURCE_REFUSAL:PROVIDER_LOGIN:scripts/gates/p06/successors/renamed-capability/probe-local.sh'; then
  printf '%s\n' "P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:gate_provider_login:$status:$actual"
  exit 1
fi
git -C "$fixture" rm -q -f scripts/gates/p06/successors/renamed-capability/probe-local.sh

# Break caught: a tracked symlink can rename an executable auth path and must
# fail closed before its target is inspected.
ln -s current.sh "$fixture/scripts/renamed-auth"
git -C "$fixture" add scripts/renamed-auth
set +e
actual=$(ruby "$scanner" "$fixture" 2>&1)
status=$?
set -e
if test "$status" != 1 || test "$actual" != 'P06_ZERO_AUTH_SOURCE_REFUSAL:TRACKED_SYMLINK:scripts/renamed-auth'; then
  printf '%s\n' "P06_ZERO_AUTH_EXPECTED_REFUSAL_MISSING:tracked_symlink:$status:$actual"
  exit 1
fi
git -C "$fixture" rm -q -f scripts/renamed-auth

# Break caught: only exact-digest, per-file-rationale evidence source may carry
# inert forbidden literals. Any byte change invalidates the closed allowlist.
evidence_path=scripts/gates/p06/test-inert-login-fixture.sh
mkdir -p "$fixture/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1"
printf '%s\n' '#!/bin/sh' 'fixture_line='\''clean_env "$TOOL" login --help'\''' 'test -n "$fixture_line"' >"$fixture/$evidence_path"
chmod 755 "$fixture/$evidence_path"
git -C "$fixture" add --chmod=+x "$evidence_path"
evidence_sha=$(shasum -a 256 "$fixture/$evidence_path" | awk '{print $1}')
jq -n --arg path "$evidence_path" --arg sha "$evidence_sha" '{
  schema_version:"taskseal.p06.zero-auth.source-inventory-allowlist.v1",
  entries:[{path:$path,sha256:$sha,matched_classes:["PROVIDER_LOGIN"],rationale:"Inert test fixture assigns a literal string and never executes a provider process."}]
}' >"$fixture/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory-allowlist.json"
git -C "$fixture" add scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory-allowlist.json
test "$(ruby "$scanner" "$fixture")" = P06_ZERO_AUTH_SOURCE_INVENTORY_PASS
printf '%s\n' '# digest mutation' >>"$fixture/$evidence_path"
git -C "$fixture" add "$evidence_path"
set +e
actual=$(ruby "$scanner" "$fixture" 2>&1)
status=$?
set -e
test "$status" = 1
test "$actual" = "P06_ZERO_AUTH_SOURCE_REFUSAL:ALLOWLIST_DIGEST:$evidence_path"
git -C "$fixture" rm -q -f "$evidence_path" scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory-allowlist.json

mkdir -p "$fixture/scripts/gates/p06/successors/observation-capability-v1-phase2" "$fixture/scripts/gates/p06"
printf '%s' "$expected_refusal" >"$fixture/scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh"
printf '%s' "$expected_refusal" >"$fixture/scripts/gates/p06/t8-native-observe-once.sh"
chmod 644 "$fixture/scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh" "$fixture/scripts/gates/p06/t8-native-observe-once.sh"
git -C "$fixture" add scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh scripts/gates/p06/t8-native-observe-once.sh
write_current 'printf '\''%s\n'\'' LOCAL_ONLY'
test "$(ruby "$scanner" "$fixture")" = P06_ZERO_AUTH_SOURCE_INVENTORY_PASS

chmod 755 "$fixture/scripts/gates/p06/t8-native-observe-once.sh"
git -C "$fixture" add --chmod=+x scripts/gates/p06/t8-native-observe-once.sh
set +e
actual=$(ruby "$scanner" "$fixture" 2>&1)
status=$?
set -e
test "$status" = 1
test "$actual" = 'P06_ZERO_AUTH_SOURCE_REFUSAL:HISTORICAL_EXECUTABLE:scripts/gates/p06/t8-native-observe-once.sh'

printf '%s\n' '#!/bin/sh' 'cat "$HOME/.codex/auth.json"' 'printf '\''%s\n'\'' HISTORICAL_ONLY_REFUSED >&2' 'exit 78' >"$fixture/scripts/gates/p06/t8-native-observe-once.sh"
chmod 644 "$fixture/scripts/gates/p06/t8-native-observe-once.sh"
git -C "$fixture" add --chmod=-x scripts/gates/p06/t8-native-observe-once.sh
set +e
actual=$(ruby "$scanner" "$fixture" 2>&1)
status=$?
set -e
test "$status" = 1
test "$actual" = 'P06_ZERO_AUTH_SOURCE_REFUSAL:HISTORICAL_STUB:scripts/gates/p06/t8-native-observe-once.sh'

printf '%s\n' P06_ZERO_AUTH_SOURCE_INVENTORY_TEST_PASS

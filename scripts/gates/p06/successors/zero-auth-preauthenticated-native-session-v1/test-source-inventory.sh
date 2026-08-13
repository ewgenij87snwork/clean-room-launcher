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

#!/bin/sh
set -eu

provider=""
fixture=""
root=""
require_clean_claim=false
preauthenticated_session=""

usage() {
  echo "usage: provider-capabilities.sh --root PATH --provider codex|claude --fixture NAME --preauthenticated-session available|unavailable|ambiguous [--require-clean-claim]" >&2
  exit 64
}

require_preauthenticated_session() {
  case "$1" in
    available) return 0 ;;
    unavailable)
      echo "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_UNAVAILABLE" >&2
      exit 78
      ;;
    ambiguous)
      echo "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_AMBIGUOUS" >&2
      exit 78
      ;;
    "")
      echo "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_REQUIRED" >&2
      exit 78
      ;;
    *)
      echo "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_STATE_REFUSED" >&2
      exit 78
      ;;
  esac
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --root) [ "$#" -ge 2 ] || usage; root=$2; shift 2 ;;
    --provider) [ "$#" -ge 2 ] || usage; provider=$2; shift 2 ;;
    --fixture) [ "$#" -ge 2 ] || usage; fixture=$2; shift 2 ;;
    --preauthenticated-session) [ "$#" -ge 2 ] || usage; preauthenticated_session=$2; shift 2 ;;
    --require-clean-claim) require_clean_claim=true; shift ;;
    *) usage ;;
  esac
done

[ -n "$root" ] && [ -n "$provider" ] && [ -n "$fixture" ] || usage
require_preauthenticated_session "$preauthenticated_session"
root=$(cd "$root" && pwd -P)
fixture_root="$root/fixtures/contracts/provider-capabilities/$provider/$fixture"
[ -d "$fixture_root" ] || { echo "UNKNOWN_FIXTURE" >&2; exit 66; }

digest_tree() {
  directory=$1
  find "$directory" -type f -print | LC_ALL=C sort | while IFS= read -r file; do
    relative=${file#"$directory"/}
    shasum -a 256 "$file" | awk -v relative="$relative" '{print $1 "  " relative}'
  done | shasum -a 256 | awk '{print $1}'
}

json_report() {
  jq -cn \
    --arg provider "$provider" \
    --arg executable_digest "$executable_digest" \
    --arg version "$version" \
    --arg os "$os" \
    --arg arch "$arch" \
    --arg discovery_root "$discovery_root" \
    --arg metadata_at_start "$metadata_at_start" \
    --arg body_on_invocation "$body_on_invocation" \
    --arg runtime_filter "$runtime_filter" \
    --arg auth_dependencies "$auth_dependencies" \
    --arg ambient_sources "$ambient_sources" \
    --argjson projection_candidate "$projection_candidate" \
    --arg state "$state" \
    --arg source_before "$source_before" \
    --arg source_after "$source_after" \
    --argjson model_invoked "$model_invoked" \
    --argjson persistent_state_unchanged "$persistent_state_unchanged" \
    '{provider:$provider,executable_digest:$executable_digest,version:$version,os:$os,arch:$arch,discovery_roots:[$discovery_root],native_metadata_lifecycle:{metadata_at_start:$metadata_at_start,body_on_invocation:$body_on_invocation},runtime_filter:$runtime_filter,auth_dependencies:$auth_dependencies,ambient_sources:[$ambient_sources],projection_candidate:$projection_candidate,state:$state,model_invoked:$model_invoked,persistent_state_unchanged:$persistent_state_unchanged,source_fixture_digest_before:$source_before,source_fixture_digest_after:$source_after}'
}

source_before=$(digest_tree "$fixture_root")
os=$(uname -s)
arch=$(uname -m)
model_invoked=false

case "$provider" in
  codex)
    executable=$(command -v codex || true)
    [ -n "$executable" ] || { echo "CODEX_NOT_FOUND" >&2; exit 69; }
    executable=$(realpath "$executable")
    executable_digest=$(shasum -a 256 "$executable" | awk '{print $1}')
    version=$(codex --version)
    discovery_root='$CODEX_HOME/skills'
    metadata_at_start=unsupported
    body_on_invocation=unsupported
    runtime_filter=unsupported
    auth_dependencies='not required for debug prompt-input'
    ambient_sources='provider-managed system skills may be materialized in ephemeral CODEX_HOME'
    projection_candidate=false
    state=unsupported

    if [ "$fixture" = "no-native-isolation" ]; then
      if [ "$require_clean_claim" = true ]; then
        echo "UNSUPPORTED_CLEAN_CLAIM: no provider-native isolation fixture" >&2
        exit 78
      fi
    elif [ "$fixture" = "wrong-version" ]; then
      expected_version=$(cat "$fixture_root/version.txt")
      [ "$version" = "$expected_version" ] || state=unsupported
    else
      ephemeral_home=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-codex-home.XXXXXX")
      cleanup_ephemeral_home() {
        case "$ephemeral_home" in
          "${TMPDIR:-/tmp}"/taskseal-codex-home.*) rm -rf -- "$ephemeral_home" ;;
          *) echo "REFUSED_UNSAFE_TEMP_CLEANUP" >&2; exit 70 ;;
        esac
      }
      trap cleanup_ephemeral_home EXIT HUP INT TERM
      cp -R "$fixture_root/." "$ephemeral_home/"
      start_json=$(CODEX_HOME="$ephemeral_home" codex debug prompt-input 'TASKSEAL_START_PROBE')
      invoked_json=$(CODEX_HOME="$ephemeral_home" codex debug prompt-input '$taskseal-canary TASKSEAL_CANARY_TRIGGER')

      if printf '%s' "$start_json" | grep -q 'TASKSEAL_CANARY_TRIGGER'; then
        metadata_at_start=qualified
      fi
      if printf '%s' "$start_json" | grep -q 'TASKSEAL_CANARY_BODY_7E5B1E21'; then
        metadata_at_start=unsupported
      fi
      if printf '%s' "$invoked_json" | grep -q 'TASKSEAL_CANARY_BODY_7E5B1E21'; then
        body_on_invocation=qualified
      fi

      if [ "$metadata_at_start" = qualified ] && [ "$body_on_invocation" = qualified ]; then
        runtime_filter=qualified
        projection_candidate=true
        state=qualified
      elif [ "$metadata_at_start" = qualified ]; then
        runtime_filter=narrowed
        state=narrowed
      fi
      cleanup_ephemeral_home
      trap - EXIT HUP INT TERM
    fi
    ;;
  claude)
    executable=$(command -v claude || true)
    [ -n "$executable" ] || { echo "CLAUDE_NOT_FOUND" >&2; exit 69; }
    executable=$(realpath "$executable")
    executable_digest=$(shasum -a 256 "$executable" | awk '{print $1}')
    version=$(claude --version)
    discovery_root='no runtime root admitted by P02 no-spend fixture'
    metadata_at_start=unsupported
    body_on_invocation=unsupported
    runtime_filter=unsupported
    auth_dependencies='not inspected; model invocation prohibited'
    ambient_sources='CLI help only'
    projection_candidate=false
    state=unsupported
    ;;
  *) usage ;;
esac

source_after=$(digest_tree "$fixture_root")
if [ "$source_before" = "$source_after" ]; then
  persistent_state_unchanged=true
else
  persistent_state_unchanged=false
  state=unsupported
  projection_candidate=false
fi

json_report

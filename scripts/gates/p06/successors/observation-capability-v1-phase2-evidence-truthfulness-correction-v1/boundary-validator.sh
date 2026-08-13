#!/bin/sh

p06_boundary_refuse() {
  printf '%s\n' "$1" >&2
  return 1
}

p06_boundary_validate_tuple_platform() {
  if test "$#" != 3; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  if test "$1" != 19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37 ||
    test "$2" != Darwin || test "$3" != arm64; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_WRONG_TUPLE
    return 1
  fi
}

p06_boundary_validate_tuple_version() {
  if test "$#" != 1; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  if test "$1" != 'codex-cli 0.147.0'; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_WRONG_TUPLE
    return 1
  fi
}

p06_boundary_validate_base() {
  if test "$#" != 5; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  if test "$1" != "$2" || test "$3" != "$2" ||
    test "$4" != feat/p06-codex-observation-capability-v1-phase2 ||
    test "$5" != true; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_WRONG_BASE
    return 1
  fi
}

p06_boundary_validate_authority() {
  if test "$#" != 3; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  authority_json=$1
  authority_root=$2
  authority_head=$3
  if ! printf '%s\n' "$authority_json" | jq -e \
    --arg root "$authority_root" --arg head "$authority_head" '
      .schema_version == "taskseal.execution-authority.v2" and
      .plan_id == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2" and
      .repository_realpath == $root and .worktree_realpath == $root and
      .branch == "feat/p06-codex-observation-capability-v1-phase2" and
      .head == $head and
      .observation_authority == {
        id:"P06-CODEX-OBS-CAP-V1-PH2-D3C7534-ONE",
        credential_source:"/Users/ysorokin/.codex/auth.json",
        credential_field:".tokens.access_token",
        login_invocations:1,
        model_processes:1,
        model_process_timeout_seconds:120,
        intrinsic_provider_requests_and_retries:"included",
        retries:0
      }
    ' >/dev/null 2>&1; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_WRONG_AUTHORITY
    return 1
  fi
}

p06_boundary_validate_counter_state() {
  if test "$#" != 2; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  if test "$1" != UNUSED || test "$2" != UNUSED; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_REUSED_COUNTER
    return 1
  fi
}

p06_boundary_validate_source_field() {
  if test "$#" != 3; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  if test "$1" != /Users/ysorokin/.codex/auth.json; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_SOURCE
    return 1
  fi
  if test "$2" != .tokens.access_token; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_FIELD
    return 1
  fi
  if test "$3" != tokens.access_token || test ".$3" != "$2"; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_ALTERNATE_CREDENTIAL_FIELD
    return 1
  fi
}

p06_boundary_validate_policy() {
  if test "$#" != 7; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  offline_policy=$1
  extract_policy=$2
  online_policy=$3
  keychain_policy=$4
  policy_temporary_root=$5
  policy_auth_source=$6
  policy_command=$7
  expected_profile_base="(version 1)
(deny default)
(import \"system.sb\")
(allow file-read-metadata (subpath \"/private\"))
(allow file-read* (subpath \"$policy_temporary_root\") (literal \"$policy_command\") (subpath \"/System\") (subpath \"/usr\") (subpath \"/private/etc\") (subpath \"/private/var/db/timezone\") (subpath \"/dev\"))
(allow process*)
(allow sysctl-read)
(allow file-write* (subpath \"$policy_temporary_root\"))"
  keychain_denial='(deny file-read* (subpath "/Users/ysorokin/Library/Keychains") (subpath "/Library/Keychains") (subpath "/System/Library/Keychains"))'
  security_denial='(deny mach-lookup (global-name-regex #"^com\.apple\.(security|SecurityAgent)"))'
  expected_offline="$expected_profile_base
(deny network*)
(deny mach-lookup)"
  expected_extract="$expected_profile_base
(allow file-read* (literal \"$policy_auth_source\"))
(deny network*)
(deny mach-lookup)"
  expected_online="$expected_profile_base
(allow network-outbound)
$keychain_denial
$security_denial"
  expected_keychain="$expected_profile_base
(deny network*)
$keychain_denial
$security_denial"

  if test "$offline_policy" != "$expected_offline" ||
    test "$extract_policy" != "$expected_extract" ||
    test "$online_policy" != "$expected_online" ||
    test "$keychain_policy" != "$expected_keychain"; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_POLICY_DRIFT
    return 1
  fi
}

p06_boundary_validate_output() {
  if test "$#" != 12; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  output_extract_result=$1
  output_login_status=$2
  output_login_result=$3
  output_model_started=$4
  output_model_status=$5
  output_native_observation=$6
  output_root_discovery=$7
  output_forbidden_ambient=$8
  output_result_sha=$9
  shift 9
  output_binary_unchanged=$1
  output_protected_unchanged=$2
  output_worktree_unchanged=$3

  if test "$output_extract_result" != EXTRACT_PREVALIDATED; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT
    return 1
  fi
  case "$output_login_status" in
    0)
      if test "$output_login_result" != LOGIN_ACCEPTED; then
        p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT
        return 1
      fi
      ;;
    *[!0-9]*|'')
      p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT
      return 1
      ;;
    *)
      if test "$output_login_result" != LOGIN_REFUSED; then
        p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT
        return 1
      fi
      ;;
  esac
  case "$output_model_started" in
    true|false) ;;
    *) p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT; return 1 ;;
  esac
  case "$output_model_status" in NOT_STARTED|*[!0-9]*|'')
    if test "$output_model_status" != NOT_STARTED; then
      p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT
      return 1
    fi
    ;;
  esac
  case "$output_native_observation" in
    NOT_STARTED|BOUNDARY_REFUSED|MODEL_UNAVAILABLE|NATIVE_REFUSED|OBSERVED) ;;
    *) p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT; return 1 ;;
  esac
  case "$output_root_discovery" in NOT_RUN|L0_L2_L3) ;; *) p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT; return 1 ;; esac
  case "$output_forbidden_ambient" in UNKNOWN|false) ;; *) p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT; return 1 ;; esac
  if test "$output_result_sha" != ABSENT; then
    if test "${#output_result_sha}" != 64; then
      p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT
      return 1
    fi
    case "$output_result_sha" in *[!0-9a-f]*) p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT; return 1 ;; esac
  fi
  for output_boolean in "$output_binary_unchanged" "$output_protected_unchanged" "$output_worktree_unchanged"; do
    case "$output_boolean" in true|false) ;; *) p06_boundary_refuse P06_BOUNDARY_REFUSAL_OUTPUT_DRIFT; return 1 ;; esac
  done
}

p06_boundary_validate_cleanup() {
  if test "$#" != 2; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  case "$1" in
    /private/tmp/taskseal-p06-phase2-runtime.*) ;;
    *) p06_boundary_refuse P06_BOUNDARY_REFUSAL_CLEANUP_FAILED; return 1 ;;
  esac
  if test "$2" != removed; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_CLEANUP_FAILED
    return 1
  fi
}

p06_boundary_validate_write_set() {
  if test "$#" != 1; then
    p06_boundary_refuse P06_BOUNDARY_REFUSAL_INVALID_ARGUMENTS
    return 1
  fi
  test -z "$1" && return 0
  while IFS= read -r changed_path; do
    case "$changed_path" in
      scripts/gates/p06/successors/observation-capability-v1-phase2/*|reports/gates/p06/successors/observation-capability-v1-phase2/*) ;;
      *) p06_boundary_refuse "P06_PHASE2_WRITE_SET_REFUSED:$changed_path"; return 1 ;;
    esac
  done <<EOF
$1
EOF
}

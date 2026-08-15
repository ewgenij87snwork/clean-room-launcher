#!/bin/sh
# Artifact-only clean-machine harness.  It intentionally never installs from source.
set -eu

artifact= checksum= receipt=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --artifact) artifact=${2:?}; shift 2 ;;
    --sha256) checksum=${2:?}; shift 2 ;;
    --receipt) receipt=${2:?}; shift 2 ;;
    *) echo "P08_LIVE_OS_REFUSED:USAGE" >&2; exit 64 ;;
  esac
done
[ -n "$artifact" ] && [ -n "$checksum" ] && [ -n "$receipt" ] || { echo "P08_LIVE_OS_REFUSED:USAGE" >&2; exit 64; }
[ -f "$artifact" ] || { echo "P08_LIVE_OS_REFUSED:ARTIFACT_MISSING" >&2; exit 65; }
actual=$(shasum -a 256 "$artifact" | awk '{print $1}')
[ "$actual" = "$checksum" ] || { echo "P08_LIVE_OS_REFUSED:ARTIFACT_CHECKSUM_MISMATCH" >&2; exit 66; }

digest_or_unavailable() {
  value=$1 label=$2
  [ -z "$value" ] && { printf '%s' UNAVAILABLE; return; }
  printf '%s' "$value" | grep -Eq '^[0-9a-f]{64}$' || { echo "P08_LIVE_OS_REFUSED:INVALID_${label}" >&2; exit 67; }
  printf '%s' "$value"
}
config=$(digest_or_unavailable "${TASKSEAL_CONFIG_SHA256:-}" CONFIG_SHA256)
provider=$(digest_or_unavailable "${TASKSEAL_PROVIDER_SHA256:-}" PROVIDER_SHA256)
git_state=$(digest_or_unavailable "${TASKSEAL_GIT_SHA256:-}" GIT_SHA256)
user_files=$(digest_or_unavailable "${TASKSEAL_USER_FILES_SHA256:-}" USER_FILES_SHA256)
[ -z "${TASKSEAL_CONFIG_SHA256_AFTER:-}" ] || [ "$config" = "${TASKSEAL_CONFIG_SHA256_AFTER}" ] || { echo "P08_LIVE_OS_REFUSED:PROTECTED_STATE_MISMATCH" >&2; exit 68; }
[ -z "${TASKSEAL_PROVIDER_SHA256_AFTER:-}" ] || [ "$provider" = "${TASKSEAL_PROVIDER_SHA256_AFTER}" ] || { echo "P08_LIVE_OS_REFUSED:PROTECTED_STATE_MISMATCH" >&2; exit 68; }
[ -z "${TASKSEAL_GIT_SHA256_AFTER:-}" ] || [ "$git_state" = "${TASKSEAL_GIT_SHA256_AFTER}" ] || { echo "P08_LIVE_OS_REFUSED:PROTECTED_STATE_MISMATCH" >&2; exit 68; }
[ -z "${TASKSEAL_USER_FILES_SHA256_AFTER:-}" ] || [ "$user_files" = "${TASKSEAL_USER_FILES_SHA256_AFTER}" ] || { echo "P08_LIVE_OS_REFUSED:PROTECTED_STATE_MISMATCH" >&2; exit 68; }

image_id=${TASKSEAL_CLEAN_IMAGE_ID:-UNAVAILABLE}
prerequisites=$(digest_or_unavailable "${TASKSEAL_PREREQUISITES_SHA256:-}" PREREQUISITES_SHA256)
host_os=$(uname -s)
reason=clean_image_or_prerequisites_not_verified
[ "$host_os" = Darwin ] || reason=unsupported_host_os
payload=$(printf '{"schema_version":"taskseal.live-os-receipt.v1","lane":"macos","qualification":"NOT_QUALIFIED","artifact_sha256":"%s","clean_image":{"id":"%s","verified":false},"prerequisites":{"sha256":"%s","verified":false},"lifecycle":{"install":"NOT_RUN","run":"NOT_RUN","upgrade":"NOT_RUN","rollback":"NOT_RUN","uninstall":"NOT_RUN"},"protected_state_before":{"config_sha256":"%s","provider_sha256":"%s","git_sha256":"%s","user_files_sha256":"%s"},"protected_state_after":{"config_sha256":"%s","provider_sha256":"%s","git_sha256":"%s","user_files_sha256":"%s"},"reason":"%s"}' "$actual" "$image_id" "$prerequisites" "$config" "$provider" "$git_state" "$user_files" "$config" "$provider" "$git_state" "$user_files" "$reason")
receipt_sha=$(printf '%s' "$payload" | shasum -a 256 | awk '{print $1}')
printf '%s}\n' "${payload%\}}\"receipt_sha256\":\"$receipt_sha\"" > "$receipt"
echo "P08_LIVE_OS_NOT_QUALIFIED lane=macos"

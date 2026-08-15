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
actual=$(sha256sum "$artifact" | awk '{print $1}')
[ "$actual" = "$checksum" ] || { echo "P08_LIVE_OS_REFUSED:ARTIFACT_CHECKSUM_MISMATCH" >&2; exit 66; }

required_digest() {
  value=$1 label=$2
  [ -n "$value" ] || { echo "P08_LIVE_OS_REFUSED:MISSING_PROTECTED_STATE" >&2; exit 67; }
  printf '%s' "$value" | grep -Eq '^[0-9a-f]{64}$' || { echo "P08_LIVE_OS_REFUSED:INVALID_${label}" >&2; exit 67; }
  printf '%s' "$value"
}
config_before=$(required_digest "${TASKSEAL_CONFIG_SHA256:-}" CONFIG_SHA256); provider_before=$(required_digest "${TASKSEAL_PROVIDER_SHA256:-}" PROVIDER_SHA256); git_before=$(required_digest "${TASKSEAL_GIT_SHA256:-}" GIT_SHA256); user_files_before=$(required_digest "${TASKSEAL_USER_FILES_SHA256:-}" USER_FILES_SHA256)
config_after=$(required_digest "${TASKSEAL_CONFIG_SHA256_AFTER:-}" CONFIG_SHA256_AFTER); provider_after=$(required_digest "${TASKSEAL_PROVIDER_SHA256_AFTER:-}" PROVIDER_SHA256_AFTER); git_after=$(required_digest "${TASKSEAL_GIT_SHA256_AFTER:-}" GIT_SHA256_AFTER); user_files_after=$(required_digest "${TASKSEAL_USER_FILES_SHA256_AFTER:-}" USER_FILES_SHA256_AFTER)
[ "$config_before" = "$config_after" ] && [ "$provider_before" = "$provider_after" ] && [ "$git_before" = "$git_after" ] && [ "$user_files_before" = "$user_files_after" ] || { echo "P08_LIVE_OS_REFUSED:PROTECTED_STATE_MISMATCH" >&2; exit 68; }

image_id=${TASKSEAL_CLEAN_IMAGE_ID:-UNAVAILABLE}; printf '%s' "$image_id" | grep -Eq '^[A-Za-z0-9._:-]+$' || { echo "P08_LIVE_OS_REFUSED:INVALID_CLEAN_IMAGE_ID" >&2; exit 69; }
prerequisites=${TASKSEAL_PREREQUISITES_SHA256:-UNAVAILABLE}
host_os=$(uname -s)
reason=clean_image_or_prerequisites_not_verified
[ "$host_os" = Linux ] || reason=unsupported_host_os
payload=$(printf '{"schema_version":"taskseal.live-os-receipt.v1","lane":"ubuntu","qualification":"NOT_QUALIFIED","artifact_sha256":"%s","clean_image":{"id":"%s","verified":false},"prerequisites":{"sha256":"%s","verified":false},"lifecycle":{"install":"NOT_RUN","run":"NOT_RUN","upgrade":"NOT_RUN","rollback":"NOT_RUN","uninstall":"NOT_RUN"},"protected_state_before":{"config_sha256":"%s","provider_sha256":"%s","git_sha256":"%s","user_files_sha256":"%s"},"protected_state_after":{"config_sha256":"%s","provider_sha256":"%s","git_sha256":"%s","user_files_sha256":"%s"},"reason":"%s"}' "$actual" "$image_id" "$prerequisites" "$config_before" "$provider_before" "$git_before" "$user_files_before" "$config_after" "$provider_after" "$git_after" "$user_files_after" "$reason")
receipt_sha=$(printf '%s' "$payload" | sha256sum | awk '{print $1}')
printf '%s,"receipt_sha256":"%s"}\n' "${payload%?}" "$receipt_sha" > "$receipt"
echo "P08_LIVE_OS_NOT_QUALIFIED lane=ubuntu"

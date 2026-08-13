#!/bin/sh
set -eu
temporary_root=$(mktemp -d /tmp/taskseal-p06-t8-sandbox.XXXXXX)
temporary_root=$(realpath "$temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-t8-sandbox.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM
profile="$temporary_root/native.sb"
escaped_root=$(printf '%s' "$temporary_root" | sed 's/[\\"]/\\&/g')
printf '(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read*)\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n(allow network-outbound)\n' "$escaped_root" >"$profile"
/usr/bin/sandbox-exec -f "$profile" /usr/bin/touch "$temporary_root/allowed"
test -f "$temporary_root/allowed"
set +e
/usr/bin/sandbox-exec -f "$profile" /usr/bin/touch /tmp/taskseal-p06-t8-sandbox-outside-denied 2>/dev/null
status=$?
set -e
test "$status" -ne 0
test ! -e /tmp/taskseal-p06-t8-sandbox-outside-denied
echo P06_T8_NATIVE_SANDBOX_PROFILE_PASS

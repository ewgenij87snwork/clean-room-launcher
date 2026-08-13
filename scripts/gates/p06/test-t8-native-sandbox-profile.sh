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
offline_profile="$temporary_root/offline.sb"
escaped_root=$(printf '%s' "$temporary_root" | sed 's/[\\"]/\\&/g')
printf '(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read-metadata (subpath "/private"))\n(allow file-read* (subpath "%s") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n(allow network-outbound)\n' "$escaped_root" "$escaped_root" >"$profile"
printf '(version 1)\n(deny default)\n(import "system.sb")\n(allow file-read-metadata (subpath "/private"))\n(allow file-read* (subpath "%s") (subpath "/System") (subpath "/usr") (subpath "/private/etc") (subpath "/private/var/db/timezone") (subpath "/dev"))\n(allow process*)\n(allow sysctl-read)\n(allow file-write* (subpath "%s"))\n(deny network*)\n' "$escaped_root" "$escaped_root" >"$offline_profile"
printf 'allowed\n' >"$temporary_root/readable"
/usr/bin/sandbox-exec -f "$profile" /usr/bin/touch "$temporary_root/allowed"
test -f "$temporary_root/allowed"
/usr/bin/sandbox-exec -f "$profile" /bin/cat "$temporary_root/readable" >/dev/null
set +e
/usr/bin/sandbox-exec -f "$profile" /usr/bin/touch /tmp/taskseal-p06-t8-sandbox-outside-denied 2>/dev/null
status=$?
set -e
test "$status" -ne 0
test ! -e /tmp/taskseal-p06-t8-sandbox-outside-denied
set +e
/usr/bin/sandbox-exec -f "$profile" /bin/cat /Users/ysorokin/taskseal/Cargo.toml >/dev/null 2>&1
read_status=$?
/usr/bin/sandbox-exec -f "$offline_profile" /usr/bin/ruby -rsocket -e 'TCPServer.new("127.0.0.1", 0)' >"$temporary_root/network.stdout" 2>"$temporary_root/network.stderr"
network_status=$?
set -e
test "$read_status" -ne 0
test "$network_status" -ne 0
rg -n 'Operation not permitted|Errno::EPERM' "$temporary_root/network.stderr" >/dev/null
echo P06_T8_NATIVE_SANDBOX_PROFILE_PASS

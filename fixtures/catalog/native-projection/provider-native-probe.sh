#!/bin/sh
set -eu
for binding in "$@"; do
  id=${binding%%=*}
  path=${binding#*=}
  test "$id" != "$binding"
  test "$(basename "$path")" = SKILL.md
  digest=$(shasum -a 256 "$path" | awk '{print $1}')
  printf '%s=%s\n' "$id" "$digest"
done

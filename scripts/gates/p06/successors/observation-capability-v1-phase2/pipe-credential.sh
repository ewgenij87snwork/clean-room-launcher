#!/bin/sh
set -eu
source_file=${1:?credential source is required}
shift
test "$#" -gt 0
test -f "$source_file" && test ! -L "$source_file"
test "$(stat -f %Lp "$source_file")" = 600
jq -e '
  keys == ["tokens"] and
  (.tokens | type == "object") and
  (.tokens | keys == ["access_token"]) and
  (.tokens.access_token | type == "string" and length > 0)
' "$source_file" >/dev/null
jq -e -r '.tokens.access_token' "$source_file" | "$@"

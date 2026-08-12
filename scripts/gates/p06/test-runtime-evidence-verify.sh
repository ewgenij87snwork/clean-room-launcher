#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
cp -R "$root/reports/gates/p06" "$tmp/receipts"

sed -i.bak 's/"result":"accepted"/"result":"tampered"/' "$tmp/receipts/task-1-v5.json"
rm "$tmp/receipts/task-1-v5.json.bak"
if "$root/scripts/gates/p06/runtime-evidence-verify.sh" --receipt-dir "$tmp/receipts" >/dev/null 2>&1; then
  echo "tampered receipt accepted" >&2
  exit 1
fi

cp -R "$root/reports/gates/p06" "$tmp/clean"
sed -i.bak 's/"output_sha256":"[0-9a-f]\{64\}"/"output_sha256":"0000000000000000000000000000000000000000000000000000000000000000"/' "$tmp/clean/task-2-v5.json"
rm "$tmp/clean/task-2-v5.json.bak"
if "$root/scripts/gates/p06/runtime-evidence-verify.sh" --receipt-dir "$tmp/clean" >/dev/null 2>&1; then
  echo "tampered artifact reference accepted" >&2
  exit 1
fi

echo P06_RUNTIME_EVIDENCE_NEGATIVES_PASS

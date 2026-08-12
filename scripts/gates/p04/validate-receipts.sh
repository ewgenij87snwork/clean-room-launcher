#!/bin/sh
set -eu
root=${1:-.}
previous=""
for n in 1 2 3 4 5 6 7 8; do
  file="$root/reports/gates/p04/task-$n.json"
  test -f "$file" || { echo "MISSING_RECEIPT:$n" >&2; exit 20; }
  jq -e --argjson n "$n" '.schema_version=="taskseal.task-receipt.v1" and .plan=="P04" and .task==$n and .acceptance_id==("ACC-P04-T"+($n|tostring)) and .evidence_id==("EVD-P04-T"+($n|tostring)) and .result=="accepted" and .skips_counted_as_pass==0 and (.input_head|test("^[0-9a-f]{40}$")) and (.result_head|test("^[0-9a-f]{40}$"))' "$file" >/dev/null || { echo "INVALID_RECEIPT:$n" >&2; exit 21; }
  input=$(jq -r .input_head "$file"); result=$(jq -r .result_head "$file")
  git -C "$root" cat-file -e "$input^{commit}" && git -C "$root" cat-file -e "$result^{commit}" || { echo "UNBOUND_HEAD:$n" >&2; exit 22; }
  git -C "$root" merge-base --is-ancestor "$input" "$result" || { echo "INVALID_HEAD_CHAIN:$n" >&2; exit 24; }
  jq -e '(.commands|type)=="array" and (.commands|length)>0 and all(.commands[]; .exit==0) and (.subjects|type)=="object" and (.subjects|length)>0 and (.accepted_at|type)=="string"' "$file" >/dev/null || { echo "INVALID_COMMAND_EVIDENCE:$n" >&2; exit 25; }
  accepted=$(jq -r .accepted_at "$file")
  committed=$(git -C "$root" show -s --format=%cI "$result")
  first=$(printf '%s\n%s\n' "$committed" "$accepted" | LC_ALL=C sort | head -n 1)
  [ "$first" = "$committed" ] || { echo "INVALID_RECEIPT_CHRONOLOGY:$n" >&2; exit 27; }
  if jq -e 'has("subjects")' "$file" >/dev/null; then
    jq -r '.subjects|to_entries[]|[.key,.value]|@tsv' "$file" | while IFS="$(printf '\t')" read -r path expected; do
      actual=$(git -C "$root" show "$result:$path" | shasum -a 256 | awk '{print $1}')
      [ "$actual" = "$expected" ] || { echo "SUBJECT_DIGEST_MISMATCH:$n:$path" >&2; exit 26; }
    done
  fi
  previous=$result
done
jq -e --arg head "$previous" '.result=="PASS" and .accepted_tasks==8 and .result_head==$head and .subject_head==$head and .skips_counted_as_pass==0' "$root/reports/gates/p04/catalog-gate.json" >/dev/null || { echo "INVALID_GATE_BINDING" >&2; exit 23; }
current=$(git -C "$root" rev-parse HEAD)
git -C "$root" merge-base --is-ancestor "$previous" "$current" || { echo "STALE_GATE_SUBJECT" >&2; exit 28; }
git -C "$root" diff --quiet "$previous" "$current" -- . ':(exclude)reports/gates/p04/**' || { echo "SUBJECT_CHANGED_AFTER_GATE" >&2; exit 29; }
echo P04_RECEIPTS_VALID

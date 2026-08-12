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
  previous=$result
done
jq -e --arg head "$previous" '.result=="PASS" and .accepted_tasks==8 and .result_head==$head and .skips_counted_as_pass==0' "$root/reports/gates/p04/catalog-gate.json" >/dev/null || { echo "INVALID_GATE_BINDING" >&2; exit 23; }
echo P04_RECEIPTS_VALID

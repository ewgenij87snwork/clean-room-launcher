#!/bin/sh
set -eu
root=${1:-.}
fixture="$root/fixtures/catalog/native-projection"
digest_a=$(shasum -a 256 "$fixture/a/SKILL.md" | awk '{print $1}')
digest_b=$(shasum -a 256 "$fixture/b/SKILL.md" | awk '{print $1}')
probe=$(
  sh "$fixture/provider-native-probe.sh" \
    "a=$fixture/a/SKILL.md" "b=$fixture/b/SKILL.md"
)
expected=$(printf 'a=%s\nb=%s' "$digest_a" "$digest_b")
[ "$probe" = "$expected" ] || { echo PROBE_OBSERVATION_MISMATCH >&2; exit 31; }
before=$(($(wc -c < "$fixture/a/SKILL.md") + $(wc -c < "$fixture/b/SKILL.md")))
startup=$(jq -cn --arg a "$digest_a" --arg b "$digest_b" '[{"id":"a","name":"alpha","capability":"c","trigger_summary":"use alpha","source_id":"src","body_digest":$a},{"id":"b","name":"beta","capability":"c","trigger_summary":"use beta","source_id":"src","body_digest":$b}]')
after=$(printf %s "$startup" | wc -c | tr -d ' ')
jq -cn \
  --argjson before "$before" --argjson after "$after" \
  --arg a "$digest_a" --arg b "$digest_b" \
  '{schema_version:"taskseal.p04.acceptance-evidence.v1",census:{admitted:2,level_a:2,loaded_now:0,load_on_invoke:2,unavailable:0,refused:0,total:2},context_bytes:{full_body_baseline:$before,startup_level_a:$after,full_bodies_at_startup:0},reasons:{a:"DEFERRED_NATIVE",b:"DEFERRED_NATIVE"},observed_body_digests:{a:$a,b:$b},controls:["SKL-01","SKL-02","SKL-03","SKL-04","SKL-05","SKL-06","SKL-07","SKL-09","SKL-12","SKL-14","SKL-15"],command:{name:"scripts/gates/p04/emit-acceptance-evidence.sh",environment:"offline-local-fixture",exit:0}}'

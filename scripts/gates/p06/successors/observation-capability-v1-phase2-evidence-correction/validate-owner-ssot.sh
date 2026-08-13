#!/bin/sh
set -eu

test "$#" = 3
status_path=$1
dashboard_path=$2
worklog_path=$3

test -f "$status_path"
test -f "$dashboard_path"
test -f "$worklog_path"

require_literal() {
  literal=$1
  path=$2
  rg -Fq -- "$literal" "$path"
}

require_literal 'Product implementation: 42% — 3 of 7 public v0.1 release plans implemented.' "$status_path"
require_literal 'Current stage: 53% of P06 — 7/13' "$status_path"
require_literal 'qualified tuples 0/1' "$status_path"
require_literal 'NOT_QUALIFIED' "$status_path"
require_literal 'STOPPED_AT_T8' "$status_path"
require_literal 'REV: BLOCKER' "$status_path"
require_literal "Task 3 replacement receipt is sealed on the \`373c9091fac7d34419d77484639d50c1331ee050\` lineage" "$status_path"
require_literal "\`P06_PHASE2_EVIDENCE_CORRECTION_PASS\`" "$status_path"
require_literal "Independent correction review remains \`PENDING\`; it is the only remaining gate." "$status_path"
require_literal "Historical active time before the captured Task 3 interval is \`not tracked\`." "$status_path"

require_literal 'Общий скоуп TaskSeal: 42% — реализовано 3 из 7 release-планов v0.1' "$dashboard_path"
require_literal 'Текущий этап P06: 53% — 7/13; Phase 2 evidence correction, qualified tuples 0/1' "$dashboard_path"
require_literal 'REV BLOCKER · EVIDENCE CORRECTION' "$dashboard_path"
require_literal 'NOT_QUALIFIED' "$dashboard_path"
require_literal 'STOPPED_AT_T8' "$dashboard_path"
require_literal 'id="task3-seal-state" data-receipt="sealed" data-gate="P06_PHASE2_EVIDENCE_CORRECTION_PASS" data-review="PENDING"' "$dashboard_path"
require_literal "Task 3 replacement receipt sealed on \`373c909\` lineage; sole gate PASS; independent terminal review pending." "$dashboard_path"
require_literal 'id="current-interval-state">STOPPED · captured Task 3 interval' "$dashboard_path"
require_literal 'id="current-active" data-start="2026-08-13T11:12:41+02:00" data-stop="2026-08-13T11:14:18+02:00" data-stale="2026-08-13T11:32:41+02:00">1 мин 37 с' "$dashboard_path"
require_literal 'id="current-remaining" data-low="10" data-high="20" data-mode="reasoned">10–20 мин · reasoned' "$dashboard_path"
require_literal 'id="current-variance">No retrospective duration was invented; 97 seconds cover only the captured initial Task 3 interval, and later review-fix time is not tracked.' "$dashboard_path"

if rg -n 'Current stage: 54%|Текущий этап P06: 54%|Phase 2 LOGIN_REFUSED disposition gate \+ independent REV PASS|Sole gate and independent rev PASS|Seal the Task 3 correction receipt|Task 3 and its independent review remain pending' \
  "$status_path" "$dashboard_path"; then
  exit 2
fi

test "$(wc -l <"$worklog_path" | tr -d ' ')" = 114
test "$(head -n 110 "$worklog_path" | shasum -a 256 | awk '{print $1}')" = fa0fef231880c923d8e0434526d111028f195abc9c23da8be4ef8507c85d13e3

jq -s -e '
  length == 114 and
  ([.[].event_id] | length) == ([.[].event_id] | unique | length) and
  (.[110] | keys) == ["category","correction_reason","event","event_id","forecast_minutes","historical_active_time","plan","remaining_minutes","replacement_event","schema","section","stale_after_at","started_at","status","supersedes_event_id","task","version","work_id","worker_id"] and
  (.[111] | keys) == ["active_seconds","category","event","event_id","forecast_minutes","plan","remaining_minutes","result","schema","section","started_at","status","stopped_at","task","variance_reason","version","work_id","worker_id"] and
  (.[112] | keys) == ["category","event","event_id","forecast_minutes","plan","remaining_minutes","schema","section","stale_after_at","started_at","status","task","version","work_id","worker_id"] and
  (.[113] | keys) == ["active_seconds","category","correction_reason","event","event_id","forecast_minutes","plan","remaining_minutes","replacement_event","result","schema","section","started_at","status","stopped_at","supersedes_event_id","task","variance_reason","version","work_id","worker_id"] and
  .[110].schema == "taskseal.worklog.event.v1" and
  .[110].event_id == "P06-CODEX-OBS-CAP-V1-PH2-002-claim-correction-start" and
  .[110].work_id == .[108].work_id and
  .[110].event == "correction" and
  .[110].supersedes_event_id == .[108].event_id and
  .[110].replacement_event == "start" and
  .[110].correction_reason == "Supersede the false independent REV PASS claim without reconstructing unrecorded Phase 2 active time." and
  .[110].started_at == "2026-08-13T11:14:18+02:00" and
  .[110].stale_after_at == "2026-08-13T11:14:18+02:00" and
  .[110].worker_id == "codex-task3" and
  .[110].category == "verification_review" and
  .[110].version == "v0.1" and
  .[110].plan == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
  .[110].section == "owner-evidence-correction" and
  .[110].task == "3" and
  .[110].forecast_minutes == null and
  .[110].remaining_minutes == {low:0,high:0} and
  .[110].status == "active" and
  .[110].historical_active_time == "not tracked" and
  .[111].schema == "taskseal.worklog.event.v1" and
  .[111].event_id == "P06-CODEX-OBS-CAP-V1-PH2-003-claim-correction-stop" and
  .[111].work_id == .[110].work_id and
  .[111].event == "stop" and
  .[111].started_at == .[110].started_at and
  .[111].stopped_at == .[110].started_at and
  .[111].active_seconds == 0 and
  .[111].worker_id == .[110].worker_id and
  .[111].category == .[110].category and
  .[111].version == .[110].version and
  .[111].plan == .[110].plan and
  .[111].section == .[110].section and
  .[111].task == .[110].task and
  .[111].forecast_minutes == null and
  .[111].remaining_minutes == {low:0,high:0} and
  .[111].result == "The Phase 2 negative receipt remains NOT_QUALIFIED and STOPPED_AT_T8; its terminal review is REV BLOCKER, not PASS. Historical Phase 2 active time is not tracked." and
  .[111].variance_reason == "Zero seconds records only the append-only claim correction; no missing Phase 2 start or duration was reconstructed." and
  .[111].status == "stopped" and
  .[112].schema == "taskseal.worklog.event.v1" and
  .[112].event_id == "P06-CODEX-OBS-CAP-V1-PH2-CORRECTION-T3-002-start" and
  .[112].work_id == .[109].work_id and
  .[112].event == "start" and
  .[112].started_at == "2026-08-13T11:12:41+02:00" and
  .[112].stale_after_at == "2026-08-13T11:32:41+02:00" and
  .[112].worker_id == "codex-task3" and
  .[112].category == "implementation" and
  .[112].version == "v0.1" and
  .[112].plan == "P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION" and
  .[112].section == "owner-evidence" and
  .[112].task == "3" and
  .[112].forecast_minutes == null and
  .[112].remaining_minutes == {low:10,high:20} and
  .[112].status == "active" and
  .[113].schema == "taskseal.worklog.event.v1" and
  .[113].event_id == "P06-CODEX-OBS-CAP-V1-PH2-CORRECTION-T3-003-stop-correction" and
  .[113].work_id == .[109].work_id and
  .[113].event == "correction" and
  .[113].supersedes_event_id == .[109].event_id and
  .[113].replacement_event == "stop" and
  .[113].correction_reason == "Replace the noncanonical Task 3 correction object with the canonical stop form for the captured interval and an allowed category." and
  .[113].started_at == .[112].started_at and
  .[113].stopped_at == "2026-08-13T11:14:18+02:00" and
  .[113].active_seconds == 97 and
  .[113].worker_id == .[112].worker_id and
  .[113].category == .[112].category and
  .[113].version == .[112].version and
  .[113].plan == .[112].plan and
  .[113].section == .[112].section and
  .[113].task == .[112].task and
  .[113].forecast_minutes == null and
  .[113].remaining_minutes == {low:10,high:20} and
  .[113].result == "The initial Task 3 receipt and sole gate were sealed on the 373c909 lineage; review requires a replacement seal and remains PENDING." and
  .[113].variance_reason == "No retrospective duration was invented: 97 seconds cover only the captured initial Task 3 interval; later review-fix time remains untracked." and
  .[113].status == "REV_BLOCKER_CORRECTION_REVIEW_PENDING" and
  ([.[110],.[113]] | all(.category == "research_design" or .category == "implementation" or .category == "verification_review" or .category == "release_operations")) and
  ([.[] | select(.supersedes_event_id? == "P06-CODEX-OBS-CAP-V1-PH2-001-negative-accepted")] | length) == 1 and
  ([.[] | select(.supersedes_event_id? == "P06-CODEX-OBS-CAP-V1-PH2-CORRECTION-T3-001")] | length) == 1
' "$worklog_path" >/dev/null

printf '%s\n' P06_PHASE2_CORRECTION_OWNER_SSOT_PASS

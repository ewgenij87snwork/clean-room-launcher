#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
verify="$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh"
correction="$root/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/correction.json"
task_1="$root/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-1.json"
task_2="$root/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-2.json"
phase_2="$root/reports/gates/p06/successors/observation-capability-v1-phase2/phase-2.json"
synthetic="$root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/synthetic-boundary.sh"
owner_worklog=$(jq -r .worklog_path "$root/.taskseal-dev/execution-authority.json")
selected_case=${P06_PHASE2_CORRECTION_TEST_CASE:-all}

temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-correction-verify.XXXXXX)
temporary_root=$(realpath "$temporary_root")
phase_2_temporary_root=$(mktemp -d /tmp/taskseal-p06-phase2-verify.XXXXXX)
phase_2_temporary_root=$(realpath "$phase_2_temporary_root")
cleanup() {
  case "$temporary_root" in
    /private/tmp/taskseal-p06-phase2-correction-verify.*) rm -rf -- "$temporary_root" ;;
    *) return 99 ;;
  esac
  case "$phase_2_temporary_root" in
    /private/tmp/taskseal-p06-phase2-verify.*) rm -rf -- "$phase_2_temporary_root" ;;
    *) return 99 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

run_bootstrap_gate() {
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap "$verify"
}

run_bootstrap_gate >/dev/null

correction_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$correction" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
    P06_PHASE2_CORRECTION_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}

phase_2_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$phase_2_temporary_root/$name.json"
  jq "$filter" "$phase_2" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
    P06_PHASE2_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}

task_1_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$task_1" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
    P06_PHASE2_CORRECTION_TASK_1_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  test "$mutation_status" -ne 0
}

task_2_mutation_must_fail() {
  name=$1
  filter=$2
  mutated="$temporary_root/$name.json"
  jq "$filter" "$task_2" >"$mutated"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2 \
    P06_PHASE2_CORRECTION_TASK_2_RECEIPT="$mutated" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf 'P06_EXPECTED_REFUSAL_MISSING:%s\n' "$name" >&2
    return 1
  fi
}

run_selected() {
  test "$selected_case" = all || test "$selected_case" = "$1"
}

prepare_fixture_repository() {
  fixture_name=$1
  fixture_repository="$temporary_root/$fixture_name-repository"
  fixture_predecessor="$temporary_root/$fixture_name-predecessor"
  git clone -q --no-hardlinks "$root" "$fixture_repository"
  git -C "$fixture_repository" config user.name taskseal-fixture
  git -C "$fixture_repository" config user.email taskseal-fixture@example.invalid
  mkdir -p "$fixture_repository/.taskseal-dev"
  jq --arg fixture_root "$fixture_repository" '
    .repository_realpath=$fixture_root | .worktree_realpath=$fixture_root
  ' "$root/.taskseal-dev/execution-authority.json" >"$fixture_repository/.taskseal-dev/execution-authority.json"

  git -C "$fixture_repository" worktree add -q \
    -b feat/p06-codex-observation-capability-v1-phase2 \
    "$fixture_predecessor" \
    refs/remotes/origin/feat/p06-codex-observation-capability-v1-phase2
  predecessor_source=$(git -C "$root" worktree list --porcelain | awk '
    $1 == "worktree" { candidate=$2 }
    $0 == "branch refs/heads/feat/p06-codex-observation-capability-v1-phase2" { print candidate }
  ')
  mkdir -p "$fixture_predecessor/.taskseal-dev"
  jq --arg fixture_root "$fixture_predecessor" '
    .repository_realpath=$fixture_root | .worktree_realpath=$fixture_root
  ' "$predecessor_source/.taskseal-dev/execution-authority.json" >"$fixture_predecessor/.taskseal-dev/execution-authority.json"
  : >"$fixture_predecessor/.taskseal-dev/phase2-login-used"
}

create_task_3_receipt() {
  fixture_repository=$1
  binding_head=$2
  fixture_authority="$fixture_repository/.taskseal-dev/execution-authority.json"
  dashboard_path=$(jq -r .dashboard_path "$fixture_authority")
  status_path=$(jq -r .status_path "$fixture_authority")
  worklog_path=$(jq -r .worklog_path "$fixture_authority")
  dashboard_sha=$(shasum -a 256 "$dashboard_path" | awk '{print $1}')
  status_sha=$(shasum -a 256 "$status_path" | awk '{print $1}')
  worklog_sha=$(shasum -a 256 "$worklog_path" | awk '{print $1}')
  {
    printf '%s\t%s\n' "$dashboard_path" "$dashboard_sha"
    printf '%s\t%s\n' "$status_path" "$status_sha"
    printf '%s\t%s\n' "$worklog_path" "$worklog_sha"
  } >"$temporary_root/task-3-subject-records.txt"
  task_3_subject_sha=$(LC_ALL=C sort "$temporary_root/task-3-subject-records.txt" | shasum -a 256 | awk '{print $1}')
  task_1_sha=$(shasum -a 256 "$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-1.json" | awk '{print $1}')
  task_2_sha=$(shasum -a 256 "$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-2.json" | awk '{print $1}')
  correction_sha=$(shasum -a 256 "$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/correction.json" | awk '{print $1}')
  binding_tree=$(git -C "$fixture_repository" rev-parse "$binding_head^{tree}")
  pass_sha=$(printf '%s\n' P06_PHASE2_EVIDENCE_CORRECTION_PASS | shasum -a 256 | awk '{print $1}')
  task_3_path="$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-3.json"
  jq -n \
    --arg binding_head "$binding_head" --arg binding_tree "$binding_tree" \
    --arg dashboard_path "$dashboard_path" --arg dashboard_sha "$dashboard_sha" \
    --arg status_path "$status_path" --arg status_sha "$status_sha" \
    --arg worklog_path "$worklog_path" --arg worklog_sha "$worklog_sha" \
    --arg subject_sha "$task_3_subject_sha" --arg task_1_sha "$task_1_sha" \
    --arg task_2_sha "$task_2_sha" --arg correction_sha "$correction_sha" \
    --arg pass_sha "$pass_sha" '
    {
      schema_version:"taskseal.p06.phase2-evidence-correction.task-receipt.v1",
      plan_id:"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION",
      task:3,
      result:"accepted",
      acceptance:{
        id:"P06-PHASE2-CORRECTION-T3-OWNER-EVIDENCE-V1",
        operator_result:"Owner evidence reports P06 7/13 as 53%, removes premature correction REV PASS, and preserves the sealed-negative NOT_QUALIFIED disposition.",
        current_progress:{completed:7,total:13,percent:53},
        independent_review:"PENDING"
      },
      binding:{
        scheme:"parent-bound-receipt.v1",
        input_head:$binding_head,
        implementation_result_head:$binding_head,
        implementation_tree:$binding_tree,
        receipt_commit_parent:$binding_head,
        resolution:"The receipt-only child resolves the non-circular containing commit."
      },
      inputs:{
        plan_checkpoint_sha256:"1cc0164fe6aec201f69b09da212a7fbd79d19f5cef7d96eadde1dc68548fd07b",
        task_1_receipt_sha256:$task_1_sha,
        task_2_receipt_sha256:$task_2_sha,
        correction_receipt_sha256:$correction_sha
      },
      subject:{
        algorithm:"sha256 of sorted path, tab, sha256, newline source records",
        sha256:$subject_sha,
        sources:[
          {path:$dashboard_path,sha256:$dashboard_sha},
          {path:$status_path,sha256:$status_sha},
          {path:$worklog_path,sha256:$worklog_sha}
        ]
      },
      evidence:[{
        id:"P06-PHASE2-CORRECTION-T3-PRECOMMIT-GATE-PASS-V1",
        command:"P06_PHASE2_CORRECTION_STAGE=task-3-precommit sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh",
        exit:0,
        output:"P06_PHASE2_EVIDENCE_CORRECTION_PASS",
        output_sha256:$pass_sha,
        meaning:"The sole correction gate accepted the Task 3 precommit fixture."
      }],
      controls:{
        owner_ssot_only:true,
        worklog_append_only:true,
        network_access:"not invoked",
        provider_or_codex_process:"not invoked",
        credential_or_keychain_read:"not invoked",
        historical_phase_1_t8_phase_2_mutation:false
      }
    }
  ' >"$task_3_path"
}

task_3_interstitial_must_fail() {
  prepare_fixture_repository task-3-interstitial
  interstitial_path="$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/interstitial.json"
  jq -n '{unexpected_interstitial_commit:true}' >"$interstitial_path"
  git -C "$fixture_repository" add -- "$interstitial_path"
  git -C "$fixture_repository" commit -q -m 'fixture: interstitial successor commit'
  interstitial_head=$(git -C "$fixture_repository" rev-parse HEAD)
  create_task_3_receipt "$fixture_repository" "$interstitial_head"
  task_3_path="$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-3.json"
  git -C "$fixture_repository" add -- "$task_3_path"
  git -C "$fixture_repository" commit -q -m 'fixture: task3 receipt after interstitial commit'
  set +e
  (CDPATH='' cd -- "$fixture_repository" && sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh) >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf '%s\n' P06_EXPECTED_REFUSAL_MISSING:task_3_interstitial >&2
    return 1
  fi
}

historical_change_then_revert_must_fail() {
  prepare_fixture_repository historical-revert
  historical_path=reports/gates/p06/successors/observation-capability-v1/phase-1.json
  jq '.result="fixture-tamper"' "$fixture_repository/$historical_path" >"$temporary_root/tampered-phase-1.json"
  mv -- "$temporary_root/tampered-phase-1.json" "$fixture_repository/$historical_path"
  git -C "$fixture_repository" add -- "$historical_path"
  git -C "$fixture_repository" commit -q -m 'fixture: mutate forbidden historical path'
  git -C "$fixture_repository" restore --source=73d48ffbed1794c6691ba59be006aa096dcfcb22 -- "$historical_path"
  git -C "$fixture_repository" commit -q -m 'fixture: restore forbidden historical path' -- "$historical_path"
  set +e
  (CDPATH='' cd -- "$fixture_repository" && \
    P06_PHASE2_CORRECTION_STAGE=task-2 \
    P06_PHASE2_CORRECTION_HISTORY_ONLY=1 \
    sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh) >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf '%s\n' P06_EXPECTED_REFUSAL_MISSING:historical_change_then_revert >&2
    return 1
  fi
}

task_2_committed_command_mutation_must_fail() {
  prepare_fixture_repository task-2-command
  sealed_receipt="$temporary_root/task-2-command-sealed.json"
  cp "$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-2.json" "$sealed_receipt"
  fixture_implementation_head=$(jq -r .binding.implementation_result_head "$sealed_receipt")
  git -C "$fixture_repository" reset --hard -q "$fixture_implementation_head"
  fixture_task_2="$fixture_repository/reports/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/task-2.json"
  jq '.evidence[0].command="sh altered-evidence-command"' "$sealed_receipt" >"$fixture_task_2"
  git -C "$fixture_repository" add -- "$fixture_task_2"
  git -C "$fixture_repository" commit -q -m 'fixture: receipt-only task2 command mutation'
  set +e
  (CDPATH='' cd -- "$fixture_repository" && \
    P06_PHASE2_CORRECTION_STAGE=task-2 \
    sh scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh) >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf '%s\n' P06_EXPECTED_REFUSAL_MISSING:task_2_command >&2
    return 1
  fi
}

prepare_task_3_ssot_fixture() {
  fixture_status="$temporary_root/task-3-status.md"
  fixture_dashboard="$temporary_root/task-3-dashboard.html"
  fixture_worklog="$temporary_root/task-3-worklog.jsonl"
  baseline_status="$temporary_root/task-3-status.baseline.md"
  baseline_dashboard="$temporary_root/task-3-dashboard.baseline.html"
  baseline_worklog="$temporary_root/task-3-worklog.baseline.jsonl"

  cat >"$baseline_status" <<'EOF'
Product implementation: 42% — 3 of 7 public v0.1 release plans implemented.
Current stage: 53% of P06 — 7/13 tasks remain accepted; qualified tuples 0/1; exact tuple NOT_QUALIFIED and STOPPED_AT_T8; Phase 2 terminal review remains REV: BLOCKER.
Task 3 replacement receipt is sealed on the `373c9091fac7d34419d77484639d50c1331ee050` lineage and the successor sole gate returned `P06_PHASE2_EVIDENCE_CORRECTION_PASS`.
Independent correction review remains `PENDING`; it is the only remaining gate.
Historical active time before the captured Task 3 interval is `not tracked`.
EOF

  cat >"$baseline_dashboard" <<'EOF'
<!doctype html>
<html><body>
<p>Общий скоуп TaskSeal: 42% — реализовано 3 из 7 release-планов v0.1</p>
<p>Текущий этап P06: 53% — 7/13; Phase 2 evidence correction, qualified tuples 0/1; NOT_QUALIFIED; STOPPED_AT_T8</p>
<p>Phase 2 terminal review: REV BLOCKER · EVIDENCE CORRECTION</p>
<div id="task3-seal-state" data-receipt="sealed" data-gate="P06_PHASE2_EVIDENCE_CORRECTION_PASS" data-review="PENDING">Task 3 replacement receipt sealed on `373c909` lineage; sole gate PASS; independent terminal review pending.</div>
<div id="current-interval-state">STOPPED · captured Task 3 interval</div>
<span id="current-active" data-start="2026-08-13T11:12:41+02:00" data-stop="2026-08-13T11:14:18+02:00" data-stale="2026-08-13T11:32:41+02:00">1 мин 37 с</span>
<span id="current-remaining" data-low="10" data-high="20" data-mode="reasoned">10–20 мин · reasoned</span>
<p id="current-variance">No retrospective duration was invented; 97 seconds cover only the captured initial Task 3 interval, and later review-fix time is not tracked.</p>
</body></html>
EOF

  head -n 110 "$owner_worklog" >"$baseline_worklog"
  cat >>"$baseline_worklog" <<'EOF'
{"schema":"taskseal.worklog.event.v1","event_id":"P06-CODEX-OBS-CAP-V1-PH2-002-claim-correction-start","work_id":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2","event":"correction","supersedes_event_id":"P06-CODEX-OBS-CAP-V1-PH2-001-negative-accepted","replacement_event":"start","correction_reason":"Supersede the false independent REV PASS claim without reconstructing unrecorded Phase 2 active time.","started_at":"2026-08-13T11:14:18+02:00","stale_after_at":"2026-08-13T11:14:18+02:00","worker_id":"codex-task3","category":"verification_review","version":"v0.1","plan":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION","section":"owner-evidence-correction","task":"3","forecast_minutes":null,"remaining_minutes":{"low":0,"high":0},"status":"active","historical_active_time":"not tracked"}
{"schema":"taskseal.worklog.event.v1","event_id":"P06-CODEX-OBS-CAP-V1-PH2-003-claim-correction-stop","work_id":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2","event":"stop","started_at":"2026-08-13T11:14:18+02:00","stopped_at":"2026-08-13T11:14:18+02:00","active_seconds":0,"worker_id":"codex-task3","category":"verification_review","version":"v0.1","plan":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION","section":"owner-evidence-correction","task":"3","forecast_minutes":null,"remaining_minutes":{"low":0,"high":0},"result":"The Phase 2 negative receipt remains NOT_QUALIFIED and STOPPED_AT_T8; its terminal review is REV BLOCKER, not PASS. Historical Phase 2 active time is not tracked.","variance_reason":"Zero seconds records only the append-only claim correction; no missing Phase 2 start or duration was reconstructed.","status":"stopped"}
{"schema":"taskseal.worklog.event.v1","event_id":"P06-CODEX-OBS-CAP-V1-PH2-CORRECTION-T3-002-start","work_id":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION-T3","event":"start","started_at":"2026-08-13T11:12:41+02:00","stale_after_at":"2026-08-13T11:32:41+02:00","worker_id":"codex-task3","category":"implementation","version":"v0.1","plan":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION","section":"owner-evidence","task":"3","forecast_minutes":null,"remaining_minutes":{"low":10,"high":20},"status":"active"}
{"schema":"taskseal.worklog.event.v1","event_id":"P06-CODEX-OBS-CAP-V1-PH2-CORRECTION-T3-003-stop-correction","work_id":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION-T3","event":"correction","supersedes_event_id":"P06-CODEX-OBS-CAP-V1-PH2-CORRECTION-T3-001","replacement_event":"stop","correction_reason":"Replace the noncanonical Task 3 correction object with the canonical stop form for the captured interval and an allowed category.","started_at":"2026-08-13T11:12:41+02:00","stopped_at":"2026-08-13T11:14:18+02:00","active_seconds":97,"worker_id":"codex-task3","category":"implementation","version":"v0.1","plan":"P06-CODEX-OBSERVATION-CAPABILITY-V1-PHASE2-EVIDENCE-CORRECTION","section":"owner-evidence","task":"3","forecast_minutes":null,"remaining_minutes":{"low":10,"high":20},"result":"The initial Task 3 receipt and sole gate were sealed on the 373c909 lineage; review requires a replacement seal and remains PENDING.","variance_reason":"No retrospective duration was invented: 97 seconds cover only the captured initial Task 3 interval; later review-fix time remains untracked.","status":"REV_BLOCKER_CORRECTION_REVIEW_PENDING"}
EOF

  cp "$baseline_status" "$fixture_status"
  cp "$baseline_dashboard" "$fixture_dashboard"
  cp "$baseline_worklog" "$fixture_worklog"
}

run_task_3_ssot_semantics() {
  P06_PHASE2_CORRECTION_STAGE=task-3-semantics \
    P06_PHASE2_CORRECTION_STATUS_PATH="$fixture_status" \
    P06_PHASE2_CORRECTION_DASHBOARD_PATH="$fixture_dashboard" \
    P06_PHASE2_CORRECTION_WORKLOG_PATH="$fixture_worklog" \
    "$verify"
}

task_3_ssot_mutation_must_fail() {
  name=$1
  set +e
  run_task_3_ssot_semantics >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf 'P06_EXPECTED_REFUSAL_MISSING:%s\n' "$name" >&2
    return 1
  fi
}

if run_selected task_3_ssot; then
  prepare_task_3_ssot_fixture
  set +e
  baseline_output=$(run_task_3_ssot_semantics 2>&1)
  baseline_exit=$?
  set -e
  if test "$baseline_exit" -ne 0; then
    test -z "$baseline_output" || printf '%s\n' "$baseline_output" >&2
    printf '%s\n' P06_EXPECTED_TASK3_SSOT_BASELINE_PASS >&2
    exit 1
  fi

  jq -c 'if .event_id == "P06-CODEX-OBS-CAP-V1-PH2-002-claim-correction-start" then .supersedes_event_id = "missing-target" else . end' \
    "$baseline_worklog" >"$fixture_worklog"
  task_3_ssot_mutation_must_fail task_3_correction_link

  jq -c 'if .event_id == "P06-CODEX-OBS-CAP-V1-PH2-002-claim-correction-start" then .category = "evidence_correction" else . end' \
    "$baseline_worklog" >"$fixture_worklog"
  task_3_ssot_mutation_must_fail task_3_correction_category

  jq -c 'if input_line_number == 1 then .event_id += "-tampered" else . end' \
    "$baseline_worklog" >"$fixture_worklog"
  task_3_ssot_mutation_must_fail task_3_prior_line

  cp "$baseline_worklog" "$fixture_worklog"
  sed 's/Current stage: 53%/Current stage: 54%/' "$baseline_status" >"$fixture_status"
  task_3_ssot_mutation_must_fail task_3_progress

  cp "$baseline_status" "$fixture_status"
  sed 's/REV BLOCKER/independent REV PASS/' "$baseline_dashboard" >"$fixture_dashboard"
  task_3_ssot_mutation_must_fail task_3_premature_pass

  cp "$baseline_dashboard" "$fixture_dashboard"
  sed "s/Independent correction review remains \`PENDING\`; it is the only remaining gate./Seal the Task 3 receipt and run the sole gate./" \
    "$baseline_status" >"$fixture_status"
  task_3_ssot_mutation_must_fail task_3_stale_lifecycle

  cp "$baseline_status" "$fixture_status"
  sed '/id="current-active"/d' "$baseline_dashboard" >"$fixture_dashboard"
  task_3_ssot_mutation_must_fail task_3_time_projection
fi

phase_2_mutation_must_fail privacy '.controls.credential_retained=true'
phase_2_mutation_must_fail protected_state '.controls.protected_state_unchanged=false'
phase_2_mutation_must_fail classification '.product.qualification="QUALIFIED"'
correction_mutation_must_fail historical_drift '.historical[0].sha256="0000000000000000000000000000000000000000000000000000000000000000"'
correction_mutation_must_fail task_1_digest '.task_1.receipt_sha256="0000000000000000000000000000000000000000000000000000000000000000"'
task_1_mutation_must_fail receipt_binding '.binding.implementation_result_head="0000000000000000000000000000000000000000"'

if run_selected task_2_tree; then
  task_2_mutation_must_fail task_2_tree '.binding.implementation_tree="0000000000000000000000000000000000000000"'
fi
if run_selected task_2_privacy; then
  task_2_mutation_must_fail task_2_privacy '.binding.resolution += " Bearer fixture-credential-material"'
fi
if run_selected current_subject; then
  subject_root="$temporary_root/current-subject"
  jq -r '.subject.sources[].path' "$task_2" | while IFS= read -r source_path; do
    mkdir -p "$subject_root/$(dirname -- "$source_path")"
    cp "$root/$source_path" "$subject_root/$source_path"
  done
  printf '%s\n' '# fixture drift' >>"$subject_root/scripts/gates/p06/successors/observation-capability-v1-phase2-evidence-correction/verify.sh"
  set +e
  P06_PHASE2_CORRECTION_STAGE=task-2 \
    P06_PHASE2_CORRECTION_SUBJECT_ROOT="$subject_root" \
    "$verify" >/dev/null 2>&1
  mutation_status=$?
  set -e
  if test "$mutation_status" = 0; then
    printf '%s\n' P06_EXPECTED_REFUSAL_MISSING:current_subject >&2
    exit 1
  fi
fi
if run_selected task_3_interstitial; then
  task_3_interstitial_must_fail
fi
if run_selected historical_revert; then
  historical_change_then_revert_must_fail
fi
if run_selected task_2_command; then
  task_2_committed_command_mutation_must_fail
fi

printf '%s\n' reports/gates/p06/task-8-rooted-disposition.json >"$temporary_root/changed-paths.txt"
set +e
P06_PHASE2_CORRECTION_STAGE=task-2-bootstrap \
  P06_PHASE2_CORRECTION_CHANGED_PATHS_FILE="$temporary_root/changed-paths.txt" \
  "$verify" >/dev/null 2>&1
write_set_status=$?
set -e
test "$write_set_status" -ne 0

set +e
model_counter_output=$("$synthetic" \
  --tuple synthetic-provider@0.0.0/darwin/arm64 \
  --base 73d48ffbed1794c6691ba59be006aa096dcfcb22 \
  --authority P06-PHASE2-CORRECTION-SYNTHETIC-AUTHORITY-V1 \
  --login-counter 0 --model-counter 1 \
  --credential-source fixture://synthetic/credential \
  --credential-field .synthetic.access_token \
  --output P06_SYNTHETIC_SAFE_OUTPUT --keychain DENIED \
  --read-policy fixture-only --write-policy none --cleanup removed \
  --successor observation-capability-v1-phase2-evidence-correction 2>&1)
model_counter_status=$?
set -e
test "$model_counter_status" = 1
test "$model_counter_output" = P06_SYNTHETIC_REFUSAL_REUSED_COUNTER

printf '%s\n' P06_PHASE2_CORRECTION_MUTATIONS_PASS

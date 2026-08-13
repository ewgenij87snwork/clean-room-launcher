#!/usr/bin/env ruby
# frozen_string_literal: true

require "csv"

PASS = "P06_ZERO_AUTH_CONTROL_PASS"

def refuse(code)
  warn "P06_ZERO_AUTH_CONTROL_REFUSAL:#{code}"
  exit 1
end

def read_utf8(path, code)
  File.read(path, encoding: "UTF-8")
rescue StandardError
  refuse(code)
end

def require_literal(text, literal, code)
  refuse(code) unless text.include?(literal)
end

def refuse_contradictory_permissions(text, code)
  compact = text.gsub(/\s+/, " ")
  prohibited_actions = [
    /\brequest\s+login\b/i,
    /\b(?:open|trigger)\b.{0,80}\bbrowser\b.{0,80}\b(?:OAuth(?:\s+flow)?|device\s+flow)\b/i,
    /\binvoke\s+provider\s+login\b/i,
    /\b(?:request|read|copy|store)\b.{0,80}\b(?:API\s+keys?|keys?|tokens?)\b/i,
    /\bfall\s+back\b.{0,80}\b(?:authentication|auth|billing)\b/i,
    /\bruntime\b.{0,80}\b(?:GitHub|stargazer)\b.{0,80}\bcheck\b/i,
    /\bautomatically\s+enable\b/i,
    /\bcontinue\b.{0,80}\bprovider\s+birth\b/i,
    /\b(?:creat(?:e|es|ed|ing)|establish(?:es|ed|ing)?)\b.{0,80}\b(?:authentication|auth)\s+session\b/i,
    /\bfunctionality\b.{0,80}\b(?:becomes|is)\s+unavailable\b/i
  ]
  negative_governance = /\b(?:(?:must|should|may|shall|will)\s+not|can(?:not|\s+not)|never|(?:does|do|is|are)\s+not|prohibited|forbidden)\b/i

  compact.scan(/\bTaskSeal\b[^.!?;]*/i).each do |clause|
    actions = prohibited_actions.flat_map do |pattern|
      clause.to_enum(:scan, pattern).map do
        match = Regexp.last_match
        [match.begin(0), match.end(0)]
      end
    end.sort_by(&:first)
    next if actions.empty?

    previous_end = 0
    negative_scope = false
    actions.each_with_index do |(action_start, action_end), index|
      bridge = clause[previous_end...action_start]
      if bridge.match?(negative_governance)
        negative_scope = true
      elsif index.zero? || bridge.scan(/[A-Za-z]+/).any? { |word| !%w[and or].include?(word.downcase) }
        negative_scope = false
      end
      refuse(code) unless negative_scope

      previous_end = action_end
    end
  end
end

def section(text, heading, next_heading, code)
  start = text.index(heading)
  refuse(code) unless start

  tail = text[(start + heading.length)..]
  finish = tail.index(next_heading)
  finish ? tail[0...finish] : tail
end

def markdown_control_rows(text)
  text.each_line.each_with_object([]) do |line, rows|
    next unless line.start_with?("|")

    cells = line.strip.sub(/\A\|/, "").sub(/\|\z/, "").split("|").map(&:strip)
    next unless cells.length == 6 && cells[0].match?(/\A[A-Z]+-[0-9]+\z/)

    rows << {
      "control_id" => cells[0],
      "requirement" => cells[1],
      "sources" => cells[2],
      "disposition" => cells[3],
      "target" => cells[4],
      "acceptance" => cells[5]
    }
  end
end

refuse("INVALID_ARGUMENTS") unless ARGV.length == 4

owner_path, master_path, trace_path, map_path = ARGV
owner = read_utf8(owner_path, "OWNER_UNREADABLE")
master = read_utf8(master_path, "MASTER_UNREADABLE")
trace = read_utf8(trace_path, "TRACE_UNREADABLE")

od10 = section(owner, "### OD-10 — zero-auth preauthenticated native session", "\n### ", "OD10_MISSING")
require_literal(od10, "Status: **APPROVED** on 2026-08-13.", "OD10_NOT_APPROVED")
require_literal(od10, "dated public evidence", "OD10_DATED_PUBLIC_EVIDENCE")
require_literal(od10, "canonical TaskSeal\nrepository has at least **50,000 public stargazers** **AND** separately approves\na named superseding product decision", "OD10_TWO_OWNER_GATES")
require_literal(od10, "TaskSeal is zero-auth", "OD10_ZERO_AUTH")
require_literal(od10, "MUST NOT request login", "OD10_NO_LOGIN_REQUEST")
require_literal(od10, "open or trigger a browser OAuth or device flow", "OD10_NO_BROWSER_OAUTH_DEVICE")
require_literal(od10, "invoke provider login", "OD10_NO_PROVIDER_LOGIN")
require_literal(od10, "MUST NOT request, read, copy, or store API keys or tokens", "OD10_ZERO_CREDENTIAL_OWNERSHIP")
require_literal(od10, "MUST NOT fall back to any authentication or billing flow", "OD10_NO_AUTH_BILLING_FALLBACK")
require_literal(od10, "MAY\nuse only a provider-native preauthenticated session that was independently\nestablished outside TaskSeal", "OD10_PREAUTHENTICATED_NATIVE_SESSION_ONLY")
require_literal(od10, "Unavailable or ambiguous session state MUST fail before provider birth", "OD10_FAIL_CLOSED_PROVIDER_BIRTH")
require_literal(od10, "local TaskSeal functionality remains available without qualification", "OD10_LOCAL_CONTINUITY")
require_literal(od10, "The\nthreshold is reconsideration eligibility only; it does not change runtime\nbehavior automatically.", "OD10_THRESHOLD_NON_AUTOMATIC")
require_literal(od10, "MUST NOT perform a runtime GitHub or stargazer\ncount check", "OD10_NO_RUNTIME_GITHUB_CHECK")
require_literal(od10, "MUST NOT automatically enable any login", "OD10_NO_AUTOMATIC_ENABLEMENT")
refuse_contradictory_permissions(od10, "OD10_CONTRADICTORY_PERMISSION")

laws = section(master, "## 4. Permanent product laws", "\n## 5.", "MASTER_LAWS_MISSING")
require_literal(laws, "until dated public evidence records at least 50,000 public stargazers", "MASTER_DATED_PUBLIC_EVIDENCE")
require_literal(laws, "at least 50,000 public stargazers", "MASTER_THRESHOLD")
require_literal(laws, "**AND** the owner separately approves a named\n  superseding product decision", "MASTER_TWO_OWNER_GATES")
require_literal(laws, "TaskSeal is zero-auth", "MASTER_ZERO_AUTH")
require_literal(laws, "MUST NOT request\n  login", "MASTER_NO_LOGIN")
require_literal(laws, "open or trigger browser OAuth/device flow", "MASTER_NO_BROWSER_OAUTH_DEVICE")
require_literal(laws, "invoke provider login", "MASTER_NO_PROVIDER_LOGIN")
require_literal(laws, "request/read/copy/store keys or tokens", "MASTER_ZERO_CREDENTIAL_OWNERSHIP")
require_literal(laws, "fall back to authentication or\n  billing", "MASTER_NO_AUTH_BILLING_FALLBACK")
require_literal(laws, "MAY use only a provider-native preauthenticated session\n  independently established outside TaskSeal", "MASTER_PREAUTHENTICATED_NATIVE_SESSION_ONLY")
require_literal(laws, "unavailable or ambiguous session\n  state refuses before provider birth", "MASTER_FAIL_CLOSED_PROVIDER_BIRTH")
require_literal(laws, "local TaskSeal functionality remains\n  available without qualification", "MASTER_LOCAL_CONTINUITY")
require_literal(laws, "threshold is reconsideration eligibility\n  only, with no runtime GitHub/stargazer check and no automatic enablement", "MASTER_THRESHOLD_NON_AUTOMATIC")
refuse_contradictory_permissions(laws, "MASTER_CONTRADICTORY_PERMISSION")
require_literal(master, "446 canonical controls", "MASTER_CONTROL_COUNT")

require_literal(trace, "**446\ncanonical controls total**", "TRACE_CONTROL_COUNT")
require_literal(trace, "Final candidate canonical control count: **446** = 230 primary + 216", "TRACE_FINAL_CONTROL_COUNT")
require_literal(trace, "446 unique canonical controls", "TRACE_UNIQUE_CONTROL_COUNT")
refuse("TRACE_OLD_ADP05_CONFLICT") if trace.include?("Required auth works without recording secret values")

trace_rows = markdown_control_rows(trace)
refuse("TRACE_CONTROL_COUNT") unless trace_rows.length == 230
refuse("TRACE_DUPLICATE_CONTROL") unless trace_rows.map { |row| row.fetch("control_id") }.uniq.length == trace_rows.length

adp05_rows = trace_rows.select { |row| row.fetch("control_id") == "ADP-05" }
refuse("TRACE_ADP05_CARDINALITY") unless adp05_rows.length == 1
adp05 = adp05_rows.first
refuse("TRACE_ADP05_ZERO_CREDENTIAL_OWNERSHIP") unless adp05 == {
  "control_id" => "ADP-05",
  "requirement" => "TaskSeal transports no authentication material; provider access may use only an independently established provider-native preauthenticated session",
  "sources" => "OD-10, MASTER",
  "disposition" => "ADAPT",
  "target" => "S01/P06",
  "acceptance" => "zero-credential/native-session gate"
}

auth_requirement = "Until dated public evidence establishes at least 50,000 public stargazers for the canonical TaskSeal repository and the owner separately approves a named superseding product decision, TaskSeal is zero-auth: no login, browser OAuth/device flow, provider login, key/token ownership, or auth/billing fallback; only an independently established provider-native preauthenticated session may reach provider birth, unavailable or ambiguous state refuses pre-birth, and local functionality remains available"
auth01_rows = trace_rows.select { |row| row.fetch("control_id") == "AUTH-01" }
refuse("TRACE_AUTH01_CARDINALITY") unless auth01_rows.length == 1
auth01 = auth01_rows.first
refuse("TRACE_AUTH01_SEMANTICS") unless auth01 == {
  "control_id" => "AUTH-01",
  "requirement" => auth_requirement,
  "sources" => "OD-10, MASTER",
  "disposition" => "ADOPT",
  "target" => "S01/P06/P08",
  "acceptance" => "offline zero-auth control + P08 release gate"
}

map_rows = begin
  CSV.read(map_path, headers: true, col_sep: "\t", encoding: "UTF-8").map(&:to_h)
rescue StandardError
  refuse("MAP_UNREADABLE")
end
expected_headers = %w[control_id disposition target execution_owner coverage_status acceptance evidence_destination requirement source_file source_line]
refuse("MAP_HEADERS") unless map_rows.first&.keys == expected_headers
refuse("MAP_CONTROL_COUNT") unless map_rows.length == 446
refuse("MAP_DUPLICATE_CONTROL") unless map_rows.map { |row| row.fetch("control_id") }.uniq.length == map_rows.length

map_adp05_rows = map_rows.select { |row| row.fetch("control_id") == "ADP-05" }
refuse("MAP_ADP05_CARDINALITY") unless map_adp05_rows.length == 1
map_adp05 = map_adp05_rows.first
refuse("MAP_ADP05_ZERO_CREDENTIAL_OWNERSHIP") unless map_adp05.fetch("disposition") == "ADAPT" &&
  map_adp05.fetch("target") == "S01/P06" &&
  map_adp05.fetch("execution_owner") == "P06-ZERO-AUTH-T1" &&
  map_adp05.fetch("coverage_status") == "EXECUTABLE_V0_1" &&
  map_adp05.fetch("acceptance") == "zero-credential/native-session gate" &&
  map_adp05.fetch("evidence_destination") == "reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json" &&
  map_adp05.fetch("requirement") == adp05.fetch("requirement") &&
  map_adp05.fetch("source_file") == "TASKSEAL-REQUIREMENTS-TRACEABILITY.md"

map_auth01_rows = map_rows.select { |row| row.fetch("control_id") == "AUTH-01" }
refuse("MAP_AUTH01_CARDINALITY") unless map_auth01_rows.length == 1
map_auth01 = map_auth01_rows.first
refuse("MAP_AUTH01_ASSIGNMENT") unless map_auth01.fetch("disposition") == "ADOPT" &&
  map_auth01.fetch("target") == "S01/P06/P08" &&
  map_auth01.fetch("execution_owner") == "P06-ZERO-AUTH-T1;P08-T10" &&
  map_auth01.fetch("coverage_status") == "EXECUTABLE_V0_1"
refuse("MAP_AUTH01_THRESHOLD_NON_AUTOMATIC") unless map_auth01.fetch("acceptance") == "Threshold eligibility is owner-governed only; no runtime GitHub/stargazer check or automatic enablement"
refuse("MAP_AUTH01_EVIDENCE") unless map_auth01.fetch("evidence_destination") == "reports/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/task-1.json;reports/gates/p08/task-10.json"
refuse("MAP_AUTH01_SEMANTICS") unless map_auth01.fetch("requirement") == auth_requirement
refuse("MAP_AUTH01_SOURCE") unless map_auth01.fetch("source_file") == "TASKSEAL-REQUIREMENTS-TRACEABILITY.md"

trace_lines = trace.lines
{
  map_adp05 => "ADP-05",
  map_auth01 => "AUTH-01"
}.each do |map_row, control_id|
  expected_line = trace_lines.index { |line| line.start_with?("| #{control_id} |") }
  refuse("MAP_#{control_id.delete('-')}_SOURCE_LINE") unless expected_line && map_row.fetch("source_line") == (expected_line + 1).to_s
end

puts PASS

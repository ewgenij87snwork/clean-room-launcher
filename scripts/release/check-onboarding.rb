#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"

class Refused < StandardError; end

def refuse(code)
  raise Refused, code
end

def exact_keys?(value, keys)
  value.is_a?(Hash) && value.keys.sort == keys.sort
end

ARTIFACT_RECEIPT = {
  "identity" => "reports/release/candidate.json#/release_artifact",
  "p07_evidence" => "reports/gates/p07/task-3.json#/claims/archive_sha256",
  "expected_sha256" => "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33"
}.freeze

def report!(source)
  refuse("PRIVATE_DATA") if source.match?(%r{(?:/Users/|/home/|ghp_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{20,}|raw_prompt|prompt_payload)})
  value = JSON.parse(source)
  required = %w[schema_version result artifact_link_receipt internal_fixture external_observation setup_time_claim comprehension_checks privacy cleanup]
  refuse("ARTIFACT_RECEIPT") unless value.is_a?(Hash) && value.key?("artifact_link_receipt")
  refuse("REPORT_SCHEMA") unless exact_keys?(value, required)
  refuse("REPORT_SCHEMA") unless value["schema_version"] == "taskseal.onboarding-readiness.v1" && value["result"] == "PREPARED_NOT_QUALIFIED"
  receipt = value["artifact_link_receipt"]
  refuse("ARTIFACT_RECEIPT") unless exact_keys?(receipt, %w[identity p07_evidence expected_sha256 verified_sha256]) && receipt.slice("identity", "p07_evidence", "expected_sha256") == ARTIFACT_RECEIPT
  refuse("ARTIFACT_DIGEST_MISMATCH") unless receipt["verified_sha256"] == receipt["expected_sha256"]

  fixture = value["internal_fixture"]
  fixture_keys = %w[kind result active_seconds user_wait_seconds help_events error_events states]
  refuse("FAKE_HUMAN_PROMOTION") unless exact_keys?(fixture, fixture_keys) && fixture["kind"] == "DETERMINISTIC_STATE_MACHINE"
  refuse("FIXTURE_RESULT") unless fixture["result"] == "PASS"
  %w[active_seconds user_wait_seconds help_events error_events].each { |key| refuse("TIME_SCHEMA") unless fixture[key].is_a?(Integer) && fixture[key] >= 0 }
  refuse("TIME_COLLAPSED") unless fixture["active_seconds"] > 0 && fixture["user_wait_seconds"] > 0
  states = fixture["states"]
  expected = %w[ARTIFACT_RECEIVED DIGEST_VERIFIED PUBLISHED_DOCS_ONLY ONE_COMMAND_STARTED CLEAN_CODEX_VERIFIED CLEANUP_VERIFIED]
  refuse("ARTIFACT_OR_DIGEST") unless states.include?("ARTIFACT_RECEIVED") && states.include?("DIGEST_VERIFIED")
  refuse("IMPOSSIBLE_CHRONOLOGY") unless states.index("ARTIFACT_RECEIVED") < states.index("DIGEST_VERIFIED")
  refuse("COACHING") unless states.include?("PUBLISHED_DOCS_ONLY")
  refuse("UNSUPPORTED_TUPLE") if states.include?("UNSUPPORTED_TUPLE")
  refuse("STATE_MACHINE") unless states == expected

  external = value["external_observation"]
  refuse("EXTERNAL_BOUNDARY") unless external == { "status" => "NOT_RUN", "reason" => "OWNER_GATE_REQUIRED_EXTERNAL_USER" }
  setup = value["setup_time_claim"]
  refuse("SETUP_TIME_CLAIM") unless setup == { "status" => "UNAVAILABLE", "reason" => "NO_OBSERVED_DISTRIBUTION" }
  comprehension = value["comprehension_checks"]
  required_comprehension = { "changed" => "REQUIRED_NOT_OBSERVED", "unchanged" => "REQUIRED_NOT_OBSERVED", "exit" => "REQUIRED_NOT_OBSERVED", "rollback" => "REQUIRED_NOT_OBSERVED" }
  refuse("COMPREHENSION") unless comprehension == required_comprehension
  refuse("RAW_DATA_RETENTION") unless value.dig("privacy", "raw_user_data_retained") == false
  refuse("PRIVACY") unless value["privacy"] == { "sanitized" => true, "raw_user_data_retained" => false }
  refuse("CLEANUP") unless value["cleanup"] == { "required" => true, "verified_by_fixture" => true }
end

def options(argv)
  refuse("USAGE") unless argv.shift == "check"
  refuse("USAGE") unless argv.length == 2 && argv[0] == "--report"
  argv[1]
end

begin
  path = options(ARGV)
  refuse("REPORT_MISSING") unless File.file?(path) && !File.symlink?(path)
  report!(File.binread(path))
  puts "P08_ONBOARDING_READY internal=PASS external=NOT_RUN setup_time=UNAVAILABLE"
rescue JSON::ParserError
  warn "P08_ONBOARDING_REFUSED:REPORT_JSON"
  exit 1
rescue Refused => error
  warn "P08_ONBOARDING_REFUSED:#{error.message}"
  exit 1
end

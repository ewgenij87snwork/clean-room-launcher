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

def report!(source)
  refuse("PRIVATE_DATA") if source.match?(%r{(?:/Users/|/home/|ghp_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{20,}|raw_prompt|prompt_payload)})
  value = JSON.parse(source)
  required = %w[schema_version result internal_fixture external_observation setup_time_claim comprehension_checks privacy cleanup]
  refuse("REPORT_SCHEMA") unless exact_keys?(value, required)
  refuse("REPORT_SCHEMA") unless value["schema_version"] == "taskseal.onboarding-readiness.v1" && value["result"] == "PREPARED_NOT_QUALIFIED"

  fixture = value["internal_fixture"]
  fixture_keys = %w[kind result active_seconds user_wait_seconds help_events error_events states]
  refuse("FAKE_HUMAN_PROMOTION") unless exact_keys?(fixture, fixture_keys) && fixture["kind"] == "DETERMINISTIC_STATE_MACHINE"
  refuse("FIXTURE_RESULT") unless fixture["result"] == "PASS"
  %w[active_seconds user_wait_seconds help_events error_events].each { |key| refuse("TIME_SCHEMA") unless fixture[key].is_a?(Integer) && fixture[key] >= 0 }
  refuse("TIME_COLLAPSED") unless fixture["active_seconds"] > 0 && fixture["user_wait_seconds"] > 0
  states = fixture["states"]
  expected = %w[ARTIFACT_RECEIVED DIGEST_VERIFIED PUBLISHED_DOCS_ONLY ONE_COMMAND_STARTED CLEAN_CODEX_VERIFIED CLEANUP_VERIFIED]
  refuse("ARTIFACT_OR_DIGEST") unless states.include?("ARTIFACT_RECEIVED") && states.include?("DIGEST_VERIFIED")
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

#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"

class Refused < StandardError; end

RECEIPTS = {
  "P02" => ["reports/gates/p02/contract-gate.json", "da9b437812a49727e9b7e8daee4207967b405ce7c228f29bdbc4b142d491785c"],
  "P03" => ["reports/gates/p03/core-gate.json", "e2eb5bc1e861e6c2159e75bc632e3fdc9f9240ce07c3039d050b30b3111bdd6c"],
  "P04" => ["reports/gates/p04/acceptance-evidence.json", "d9abc02b9cde8bf223f246477695b42582a0038745977207319d6f04d7a44265"],
  "P05" => ["reports/gates/p05/cli-gate.json", "58170e1607f93773d019bee79e2b9d028bf970d06b3fec0f04dbfbe60548fd21"],
  "P06" => ["reports/gates/p06/qualification-gate.json", "29c7b2bf3b06d9f9a161cf05f5674af0ee4a46d34b8b961d319a2cbeb7265a54"],
  "P07" => ["reports/gates/p07/supply-chain-gate.json", "b1c9dc8bc97120661da3430cc925436fdeb7dabf0f1d2e0835f3f533e5c8ba95"]
}.freeze
SUPPORT = {
  "P06_PRIVACY" => ["reports/gates/p06/privacy-release-boundary-v1.json", "e7e5a064f288e974f6775923cc2108c5fc096bca03571d16e81aff32bdbb13fd"],
  "P07_LEGAL_CORRECTION" => ["reports/gates/p07/legal-candidate-correction-v1.json", "34aa4a611977f906cb14431dcc3a0fb6031c3ac831115e5ec84f2ed7f2bf88a9"],
  "P07_ARTIFACT_PRIVACY" => ["reports/gates/p07/artifact-privacy-correction-v1.json", "2d5a97d1a39f24eca2875db2b887fa43d8b1d0ac6cd24631cf141d106cc3641c"],
  "CODEX_LIVE_OBSERVATION" => ["reports/release/codex-live-observation.json", "7c3a260f83b0b05ed09bf37f48293268e0625d8d77f95cee9689390c08e64ec2"]
}.freeze
SOURCE_COMMIT = "b6581cb70686b2a32a0ea8496a9c15794f0bb6ca"
RELEASE_SHA256 = "49a685a98df13fa59766d9032dca7229cfa3e996b1b9b92ec59be8da1b899cf7"
OBSERVED_RELEASE_SHA256 = "ea8e60d2b4097ce766758bd70543628d0c15e9c7ab0ebc0d29d76c59da896b0c"
CHECKSUMS_SHA256 = "d8661e584ab79298cd0a59f24d61cd60b52cc3e06a12e44f5d757ed9383ff4f9"
SBOM_SHA256 = "6219362b6c2f333fe48047f0363f32c03f2015f85b6c00aad937b96221568c59"
PROVENANCE_SHA256 = "b8c997f55d23dd8d8b5853bdf3f5a14f29416b5ed30b95095af4adcf19b41225"
CANDIDATE_COMMIT = "c6f9d8f1cfa65c63bf34595eee056ff5d9202b25"
PRIVATE = %r{(?:/Users/|/home/|ghp_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[A-Z0-9]{16}|raw_prompt|prompt_payload)}

def refuse(message)
  raise Refused, message
end

def reject_duplicate_keys!(text)
  # Gate inputs are JSON objects. A duplicate name is ambiguous evidence and is refused
  # before JSON.parse silently collapses it.
  stack = []
  in_string = false
  escaped = false
  key = +""
  reading_key = false
  text.each_char do |char|
    if in_string
      if escaped then escaped = false
      elsif char == "\\" then escaped = true
      elsif char == '"'
        in_string = false
        reading_key = !stack.empty? && stack.last[:expect_key]
        stack.last[:pending_key] = key if reading_key
      else key << char if !stack.empty? && stack.last[:expect_key]
      end
      next
    end
    if char == '"'
      in_string = true
      key = +""
    elsif char == '{'
      stack << { keys: {}, expect_key: true, pending_key: nil }
    elsif char == ':' && !stack.empty? && stack.last[:pending_key]
      object = stack.last
      name = object.delete(:pending_key)
      refuse("DUPLICATE_JSON_KEY") if object[:keys][name]
      object[:keys][name] = true
      object[:expect_key] = false
    elsif char == ',' && !stack.empty?
      stack.last[:expect_key] = true
    elsif char == '}'
      stack.pop
    end
  end
end

def read_receipt(root, plan, relative, expected)
  path = File.join(root, relative)
  refuse("RECEIPT_MISSING:#{plan}") unless File.file?(path) && !File.symlink?(path)
  bytes = File.binread(path)
  refuse("PRIVATE_DATA:#{plan}") if PRIVATE.match?(bytes)
  reject_duplicate_keys!(bytes)
  digest = Digest::SHA256.hexdigest(bytes)
  refuse("RECEIPT_STALE:#{plan}") unless digest == expected
  value = JSON.parse(bytes)
  refuse("RECEIPT_SCHEMA:#{plan}") unless value.is_a?(Hash) && value["schema_version"].is_a?(String)
  { "plan" => plan, "path" => relative, "sha256" => digest, "schema_version" => value.fetch("schema_version"), "result" => value["result"] || value["qualification"] || "EVIDENCE_RECORDED" }
rescue JSON::ParserError
  refuse("RECEIPT_JSON:#{plan}")
end

def read_evidence(root, label, relative, expected)
  path = File.join(root, relative)
  refuse("EVIDENCE_MISSING:#{label}") unless File.file?(path) && !File.symlink?(path)
  bytes = File.binread(path)
  refuse("PRIVATE_DATA:#{label}") if PRIVATE.match?(bytes)
  reject_duplicate_keys!(bytes)
  refuse("EVIDENCE_STALE:#{label}") unless Digest::SHA256.hexdigest(bytes) == expected
  value = JSON.parse(bytes)
  refuse("EVIDENCE_SCHEMA:#{label}") unless value.is_a?(Hash) && value["schema_version"].is_a?(String)
  value
rescue JSON::ParserError
  refuse("EVIDENCE_JSON:#{label}")
end

def exact_keys?(value, keys)
  value.is_a?(Hash) && value.keys.sort == keys.sort
end

def validate_support!(privacy, legal, artifact_privacy, codex)
  refuse("EVIDENCE_SCHEMA:P06_PRIVACY") unless privacy["schema_version"] == "taskseal.p06.privacy-release-boundary.v1" && privacy["result"] == "PASS" && privacy.dig("subjects", "qualification/public-release-inventory-v1.json") == "0de3bd1b4fb706e0e790e92463750b044b10ae325981be2d870f4ffceb613044"
  refuse("EVIDENCE_SCHEMA:P07_LEGAL_CORRECTION") unless legal["schema_version"] == "taskseal.p07.legal-candidate-correction.v1" && legal["result"] == "PASS" && legal.dig("risk_continuation", "current_status") == "CLOSED"
  refuse("EVIDENCE_SCHEMA:P07_ARTIFACT_PRIVACY") unless artifact_privacy["schema_version"] == "taskseal.p07.artifact-privacy-correction.v1" && artifact_privacy["result"] == "PASS" && artifact_privacy.dig("artifact", "source_commit") == SOURCE_COMMIT && artifact_privacy.dig("artifact", "sha256") == RELEASE_SHA256 && artifact_privacy.dig("artifact", "binary_concrete_home_paths") == 0 && artifact_privacy.dig("supply_chain", "checksums_sha256") == CHECKSUMS_SHA256 && artifact_privacy.dig("supply_chain", "sbom_sha256") == SBOM_SHA256 && artifact_privacy.dig("supply_chain", "provenance_sha256") == PROVENANCE_SHA256
  refuse("EVIDENCE_SCHEMA:CODEX_LIVE_OBSERVATION") unless codex["schema_version"] == "taskseal.p08.codex-live-observation.v1" && codex["result"] == "OBSERVED_NOT_QUALIFIED" && codex["qualification"] == "NOT_QUALIFIED" && codex.dig("binding", "release_artifact", "artifact_sha256") == OBSERVED_RELEASE_SHA256 && codex.dig("acceptance", "p06_exact_qualified_tuple") == false && codex.dig("acceptance", "overall") == false
end

def validate_dossier!(dossier)
  required = %w[schema_version candidate_commit requested_release_state qualification receipts source_artifact release_artifact checksums sbom provenance tuple_matrix known_risks]
  refuse("DOSSIER_SCHEMA") unless exact_keys?(dossier, required) && dossier["schema_version"] == "taskseal.release-dossier.v1" && dossier["candidate_commit"] == CANDIDATE_COMMIT && dossier["requested_release_state"] == "PRIVATE_CANDIDATE" && dossier["qualification"] == "NOT_QUALIFIED"
  receipts = dossier["receipts"]
  refuse("DOSSIER_SCHEMA") unless receipts.is_a?(Array) && receipts.length == RECEIPTS.length && receipts.map { |entry| entry["plan"] } == RECEIPTS.keys
  RECEIPTS.each_with_index do |(plan, (path, digest)), index|
    entry = receipts[index]
    refuse("DOSSIER_SCHEMA") unless exact_keys?(entry, %w[plan path sha256 schema_version result]) && entry["plan"] == plan && entry["path"] == path && entry["sha256"] == digest && entry["schema_version"].is_a?(String) && entry["result"].is_a?(String)
  end
  expected_links = {
    "source_artifact" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/artifact/source_commit", "commit" => SOURCE_COMMIT },
    "release_artifact" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/artifact/sha256", "sha256" => RELEASE_SHA256 },
    "checksums" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/supply_chain/checksums_sha256", "sha256" => CHECKSUMS_SHA256 },
    "sbom" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/supply_chain/sbom_sha256", "sha256" => SBOM_SHA256, "profile" => "1.7" },
    "provenance" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/supply_chain/provenance_sha256", "sha256" => PROVENANCE_SHA256, "predicate" => "https://slsa.dev/provenance/v1" }
  }
  expected_links.each { |key, value| refuse("DOSSIER_SCHEMA") unless dossier[key] == value }
  refuse("DOSSIER_SCHEMA") unless dossier["tuple_matrix"] == [
    { "evidence_path" => RECEIPTS["P06"].first, "evidence_pointer" => "/required_tuple/qualification", "qualification" => "NOT_QUALIFIED" },
    { "evidence_path" => SUPPORT["CODEX_LIVE_OBSERVATION"].first, "evidence_pointer" => "/qualification", "qualification" => "NOT_QUALIFIED" }
  ]
  refuse("DOSSIER_SCHEMA") unless dossier["known_risks"] == [
    { "evidence_path" => SUPPORT["P06_PRIVACY"].first, "evidence_pointer" => "/result", "status" => "PASS" },
    { "evidence_path" => SUPPORT["P07_LEGAL_CORRECTION"].first, "evidence_pointer" => "/risk_continuation/current_status", "status" => "CLOSED" },
    { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/result", "status" => "PASS" },
    { "evidence_path" => SUPPORT["CODEX_LIVE_OBSERVATION"].first, "evidence_pointer" => "/acceptance/overall", "status" => "BLOCKER" }
  ]
end

def parse_args(argv)
  refuse("USAGE") unless argv.shift == "collect"
  options = {}
  until argv.empty?
    key, value = argv.shift, argv.shift
    refuse("USAGE") unless value && %w[--root --output --requested-state].include?(key) && !options.key?(key)
    options[key] = value
  end
  refuse("USAGE") unless options.keys.sort == %w[--output --requested-state --root]
  refuse("STATE") unless options["--requested-state"] == "PRIVATE_CANDIDATE"
  options
end

def collect(options)
  root = File.realpath(options.fetch("--root"))
  receipts = RECEIPTS.map { |plan, (path, digest)| read_receipt(root, plan, path, digest) }.to_h { |entry| [entry["plan"], entry] }
  privacy = read_evidence(root, "P06_PRIVACY", *SUPPORT.fetch("P06_PRIVACY"))
  legal = read_evidence(root, "P07_LEGAL_CORRECTION", *SUPPORT.fetch("P07_LEGAL_CORRECTION"))
  artifact_privacy = read_evidence(root, "P07_ARTIFACT_PRIVACY", *SUPPORT.fetch("P07_ARTIFACT_PRIVACY"))
  codex = read_evidence(root, "CODEX_LIVE_OBSERVATION", *SUPPORT.fetch("CODEX_LIVE_OBSERVATION"))
  validate_support!(privacy, legal, artifact_privacy, codex)
  dossier = {
    "schema_version" => "taskseal.release-dossier.v1",
    "candidate_commit" => CANDIDATE_COMMIT,
    "requested_release_state" => options.fetch("--requested-state"),
    "qualification" => "NOT_QUALIFIED",
    "receipts" => receipts.values,
    "source_artifact" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/artifact/source_commit", "commit" => SOURCE_COMMIT },
    "release_artifact" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/artifact/sha256", "sha256" => RELEASE_SHA256 },
    "checksums" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/supply_chain/checksums_sha256", "sha256" => CHECKSUMS_SHA256 },
    "sbom" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/supply_chain/sbom_sha256", "sha256" => SBOM_SHA256, "profile" => "1.7" },
    "provenance" => { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/supply_chain/provenance_sha256", "sha256" => PROVENANCE_SHA256, "predicate" => "https://slsa.dev/provenance/v1" },
    "tuple_matrix" => [
      { "evidence_path" => RECEIPTS["P06"].first, "evidence_pointer" => "/required_tuple/qualification", "qualification" => "NOT_QUALIFIED" },
      { "evidence_path" => SUPPORT["CODEX_LIVE_OBSERVATION"].first, "evidence_pointer" => "/qualification", "qualification" => "NOT_QUALIFIED" }
    ],
    "known_risks" => [
      { "evidence_path" => SUPPORT["P06_PRIVACY"].first, "evidence_pointer" => "/result", "status" => "PASS" },
      { "evidence_path" => SUPPORT["P07_LEGAL_CORRECTION"].first, "evidence_pointer" => "/risk_continuation/current_status", "status" => "CLOSED" },
      { "evidence_path" => SUPPORT["P07_ARTIFACT_PRIVACY"].first, "evidence_pointer" => "/result", "status" => "PASS" },
      { "evidence_path" => SUPPORT["CODEX_LIVE_OBSERVATION"].first, "evidence_pointer" => "/acceptance/overall", "status" => "BLOCKER" }
    ]
  }
  validate_dossier!(dossier)
  encoded = JSON.generate(dossier) + "\n"
  refuse("PRIVATE_DATA:OUTPUT") if PRIVATE.match?(encoded)
  output = options.fetch("--output")
  refuse("OUTPUT") if File.symlink?(output)
  parent = File.dirname(output)
  refuse("OUTPUT") if File.exist?(parent) && (!File.directory?(parent) || File.symlink?(parent))
  Dir.mkdir(parent) unless File.directory?(parent)
  File.write(output, encoded)
  puts "P08_DOSSIER_COLLECTED state=PRIVATE_CANDIDATE qualification=NOT_QUALIFIED"
end

begin
  collect(parse_args(ARGV))
rescue Refused => error
  warn "P08_DOSSIER_REFUSED:#{error.message}"
  exit 1
end

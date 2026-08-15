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
P07_TASK7_DIGEST = "d0f599d30b2f33381d99c548f0509c6d31131ee6bb0142ccd3b3c151c8c83142"
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
  p07 = JSON.parse(File.read(File.join(root, RECEIPTS.fetch("P07").first)))
  refuse("RECEIPT_DIGEST:P07_TASK_7") unless p07.dig("receipts", "7") == P07_TASK7_DIGEST
  task7_path = File.join(root, "reports/gates/p07/task-7.json")
  refuse("RECEIPT_MISSING:P07_TASK_7") unless File.file?(task7_path) && !File.symlink?(task7_path)
  refuse("RECEIPT_DIGEST:P07_TASK_7") unless Digest::SHA256.file(task7_path).hexdigest == P07_TASK7_DIGEST
  task3 = JSON.parse(File.read(File.join(root, "reports/gates/p07/task-3.json")))
  archive_sha = task3.dig("claims", "archive_sha256")
  commit = p07["head"]
  refuse("EVIDENCE_CLOSED") unless commit&.match?(/\A[0-9a-f]{40}\z/) && archive_sha&.match?(/\A[0-9a-f]{64}\z/)
  dossier = {
    "schema_version" => "taskseal.release-dossier.v1",
    "candidate_commit" => commit,
    "requested_release_state" => options.fetch("--requested-state"),
    "qualification" => "NOT_QUALIFIED",
    "receipts" => receipts.values,
    "source_artifact" => { "evidence_path" => "reports/gates/p07/task-3.json", "evidence_pointer" => "/claims/source_commit", "commit" => task3.dig("claims", "source_commit") },
    "release_artifact" => { "evidence_path" => "reports/gates/p07/task-3.json", "evidence_pointer" => "/claims/archive_sha256", "sha256" => archive_sha },
    "checksums" => { "evidence_path" => "reports/gates/p07/task-6.json", "evidence_pointer" => "/acceptance/evidence_ids/1", "status" => "bound" },
    "sbom" => { "evidence_path" => "reports/gates/p07/task-6.json", "evidence_pointer" => "/controls/cyclonedx_profile", "profile" => "1.7" },
    "provenance" => { "evidence_path" => "reports/gates/p07/task-6.json", "evidence_pointer" => "/controls/slsa_predicate", "predicate" => "https://slsa.dev/provenance/v1" },
    "tuple_matrix" => [{ "evidence_path" => "reports/gates/p06/qualification-gate.json", "evidence_pointer" => "/required_tuple", "qualification" => "NOT_QUALIFIED" }],
    "known_risks" => [{ "evidence_path" => "reports/gates/p07/terminal-review.json", "evidence_pointer" => "/blockers", "status" => "BLOCKER" }]
  }
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

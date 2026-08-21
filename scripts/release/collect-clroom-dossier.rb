#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"

class Refused < StandardError; end

RECEIPTS = {
  "P06" => ["reports/gates/p06/successors/clroom-macos-isolated-launch-v1/result.json", "15ba7ca41420c855475aab6ff4193914bdaad6dc945003cc32ea474a251755bf"],
  "P07" => ["reports/gates/p07/successors/clroom-packaging-v1/task-2.json", "62df830ef0c52e373fa112334e3f2f50bcc8434cdecc74a84c3acdd5004a4568"],
  "LIVE_CODEX" => ["reports/release/clroom-codex-live.json", "f96de7724aa5b187b57d689565cda3575c234cf9f58f562270c88a2bfc55ff8b"]
}.freeze
PRIVATE = %r{/(?:Users|home)/|ghp_[A-Za-z0-9]+|sk-[A-Za-z0-9_-]+}

def refuse(code)
  raise Refused, code
end
def read_receipt(root, plan, path, sha)
  file = File.join(root, path)
  refuse("RECEIPT_MISSING:#{plan}") unless File.file?(file) && !File.symlink?(file)
  bytes = File.binread(file)
  refuse("PRIVATE_DATA:#{plan}") if PRIVATE.match?(bytes)
  refuse("RECEIPT_STALE:#{plan}") unless Digest::SHA256.hexdigest(bytes) == sha
  value = JSON.parse(bytes)
  refuse("RECEIPT_SCHEMA:#{plan}") unless value.is_a?(Hash)
  { "plan" => plan, "path" => path, "sha256" => sha }
rescue JSON::ParserError
  refuse("RECEIPT_JSON:#{plan}")
end

def options(argv)
  result = {}
  until argv.empty?
    key, value = argv.shift, argv.shift
    refuse("USAGE") unless value && %w[--root --artifact --expected-sha256 --candidate-commit --output].include?(key) && !result.key?(key)
    result[key] = value
  end
  refuse("USAGE") unless result.keys.sort == %w[--artifact --candidate-commit --expected-sha256 --output --root]
  refuse("CANDIDATE_COMMIT") unless result["--candidate-commit"].match?(/\A[0-9a-f]{40}\z/)
  refuse("ARTIFACT_SHA") unless result["--expected-sha256"].match?(/\A[0-9a-f]{64}\z/)
  result
end

def collect(args)
  root = File.realpath(args.fetch("--root"))
  artifact = File.realpath(args.fetch("--artifact"))
  refuse("ARTIFACT_MISSING") unless File.file?(artifact)
  digest = Digest::SHA256.file(artifact).hexdigest
  refuse("ARTIFACT_DIGEST_MISMATCH") unless digest == args.fetch("--expected-sha256")
  receipts = RECEIPTS.map { |plan, (path, sha)| read_receipt(root, plan, path, sha) }
  p06 = JSON.parse(File.read(File.join(root, RECEIPTS.fetch("P06").first)))
  p07 = JSON.parse(File.read(File.join(root, RECEIPTS.fetch("P07").first)))
  live = JSON.parse(File.read(File.join(root, RECEIPTS.fetch("LIVE_CODEX").first)))
  refuse("P06_BOUNDARY_UNPROVEN") unless p06["schema_version"] == "clroom.macos-isolated-launch.result.v1" && p06["process_count"] == 1 && p06["real_help_exit_class"] == "HELP_EXIT_0" && p06["raw_retained"] == false
  refuse("P07_PACKAGING_UNPROVEN") unless p07["schema_version"] == "taskseal.p07.clroom-packaging.v1" && p07["result"] == "PASS" && p07.dig("artifact", "contains_only_binary") == "bin/clroom" && p07.dig("lifecycle", "cleanup_complete") == true
  refuse("LIVE_CODEX_UNPROVEN") unless live["schema_version"] == "clroom.codex-live-acceptance.v1" && live["result"] == "PASS_EXACT_TUPLE" && live["artifact_sha256"] == digest && live["process_count"] == 1 && live["marker_observed"] == true && live["raw_output_retained"] == false && live["cleanup_complete"] == true
  dossier = {
    "schema_version" => "clroom.release-dossier.v1",
    "candidate_commit" => args.fetch("--candidate-commit"),
    "requested_release_state" => "PRIVATE_CANDIDATE",
    "qualification" => "NOT_QUALIFIED",
    "artifact" => { "filename" => File.basename(artifact), "sha256" => digest, "binary" => "clroom", "package" => "clean-room-launcher" },
    "receipts" => receipts,
    "known_blockers" => ["PUBLICATION_NOT_AUTHORIZED", "NAMESPACE_OWNERSHIP_UNPROVEN", "EXTERNAL_INSTALL_NOT_RUN"]
  }
  encoded = JSON.generate(dossier) + "\n"
  File.write(args.fetch("--output"), encoded)
  puts "P08_CLROOM_DOSSIER_PASS state=PRIVATE_CANDIDATE qualification=NOT_QUALIFIED artifact_sha256=#{digest}"
end

begin
  collect(options(ARGV))
rescue Refused => error
  warn "P08_CLROOM_DOSSIER_REFUSED:#{error.message}"
  exit 1
end

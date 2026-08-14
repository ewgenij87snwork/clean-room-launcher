#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"
require "open3"

root = File.realpath(ARGV.fetch(0))
manifest_path = File.realpath(ARGV.fetch(1))

def refuse(reason)
  warn "P06_ZERO_AUTH_ENTRYPOINT_REFUSAL:#{reason}"
  exit 1
end

def git_capture(root, *arguments)
  stdout, stderr, status = Open3.capture3("git", "-C", root, *arguments)
  refuse("GIT:#{stderr.strip}") unless status.success?
  stdout
end

manifest = JSON.parse(File.binread(manifest_path))
refuse("SCHEMA") unless manifest.keys == %w[
  schema_version
  cli_boundary
  public_probe
  rust_identity_boundary
] && manifest.fetch("schema_version") == "taskseal.p06.zero-auth.entrypoint-inventory.v2"

cli = manifest.fetch("cli_boundary")
refuse("CLI_SCHEMA") unless cli.keys == %w[source sensitive_predicate stable_refusal iterator_reads entrypoints] &&
  cli.fetch("source") == "src/cli/mod.rs" &&
  cli.fetch("stable_refusal") == "ZERO_AUTH_ARGUMENT_REFUSAL: sensitive argument refused before dispatch; continue locally"

cli_source = File.binread(File.join(root, cli.fetch("source")))
refuse("CLI_PREDICATE") unless cli_source.include?(cli.fetch("sensitive_predicate"))
actual_reads = cli_source.scan(/\bsource\.(?:next|nth|take|collect)\s*\([^)]*\)/).uniq.sort
refuse("CLI_ITERATOR_READS") unless actual_reads == cli.fetch("iterator_reads").sort

actual_cli = Dir.glob(File.join(root, "src/bin/*.rs")).sort.each_with_object([]) do |absolute, entrypoints|
  path = absolute.delete_prefix("#{root}/")
  line = File.readlines(absolute, chomp: true).map(&:strip).find { |candidate| candidate.include?("std::env::args()") }
  entrypoints << { "path" => path, "line" => line } if line
end
refuse("CLI_ENTRYPOINTS") unless actual_cli == cli.fetch("entrypoints")

probe = manifest.fetch("public_probe")
refuse("PUBLIC_PROBE_SCHEMA") unless probe.keys == %w[
  path
  mode
  object_type
  blob_oid
  content_sha256
  purpose
  scope
]
refuse("PUBLIC_PROBE_PATH") unless probe.fetch("path").match?(%r{\Ascripts/probe/[^/]+\z})
refuse("PUBLIC_PROBE_MODE") unless probe.fetch("mode") == "100755"
refuse("PUBLIC_PROBE_TYPE") unless probe.fetch("object_type") == "blob"
refuse("PUBLIC_PROBE_BLOB") unless probe.fetch("blob_oid").match?(/\A[0-9a-f]{40,64}\z/)
refuse("PUBLIC_PROBE_DIGEST") unless probe.fetch("content_sha256").match?(/\A[0-9a-f]{64}\z/)
refuse("PUBLIC_PROBE_PURPOSE") unless probe.fetch("purpose").is_a?(String) && !probe.fetch("purpose").empty?
refuse("PUBLIC_PROBE_SCOPE") unless probe.fetch("scope") == "V0_1_PUBLIC_PROBE_EXECUTABLE"

tree_records = git_capture(root, "ls-tree", "-r", "-z", "HEAD", "--", "scripts/probe").split("\0").reject(&:empty?)
expected_record = [
  probe.fetch("mode"),
  probe.fetch("object_type"),
  probe.fetch("blob_oid"),
  probe.fetch("path")
]
actual_records = tree_records.map do |record|
  metadata, path = record.split("\t", 2)
  mode, object_type, object_id = metadata.split(" ", 3)
  [mode, object_type, object_id, path]
end
refuse("PUBLIC_PROBE_TREE_TUPLE") unless actual_records == [expected_record]

probe_bytes = git_capture(root, "cat-file", "blob", probe.fetch("blob_oid"))
refuse("PUBLIC_PROBE_CONTENT_DIGEST") unless Digest::SHA256.hexdigest(probe_bytes) == probe.fetch("content_sha256")

rust = manifest.fetch("rust_identity_boundary")
refuse("RUST_IDENTITY_SCHEMA") unless rust.keys == %w[provider_births provider_guards]

births = []
birth_positions = Hash.new { |positions, path| positions[path] = [] }
Dir.glob(File.join(root, "src/**/*.rs")).sort.each do |absolute|
  path = absolute.delete_prefix("#{root}/")
  File.readlines(absolute, chomp: true).each_with_index do |line, index|
    stripped = line.strip
    next unless stripped.include?("Command::new(")

    births << { "path" => path, "line" => stripped }
    birth_positions[path] << index
  end
end
births.sort_by! { |entry| [entry.fetch("path"), entry.fetch("line")] }
expected_births = rust.fetch("provider_births").sort_by { |entry| [entry.fetch("path"), entry.fetch("line")] }
refuse("RUST_PROVIDER_BIRTHS") unless births == expected_births

guards = rust.fetch("provider_guards")
refuse("RUST_GUARD_PATHS") unless guards.map { |guard| guard.fetch("path") }.sort == births.map { |birth| birth.fetch("path") }.uniq.sort
guards.each do |guard|
  path = guard.fetch("path")
  lines = File.readlines(File.join(root, path), chomp: true)
  guard_positions = lines.each_index.select do |index|
    stripped = lines.fetch(index).strip
    !stripped.start_with?("//") && stripped.include?(guard.fetch("marker"))
  end
  refuse("RUST_GUARD_MISSING:#{path}") if guard_positions.empty?
  birth_positions.fetch(path).each do |birth_index|
    refuse("RUST_GUARD_ORDER:#{path}") unless guard_positions.any? { |guard_index| guard_index < birth_index }
  end
end

puts "P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS"

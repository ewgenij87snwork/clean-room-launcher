#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"

root = File.realpath(ARGV.fetch(0))
inventory_path = File.realpath(ARGV.fetch(1))

def refuse(reason)
  warn "P06_ZERO_AUTH_ENTRYPOINT_REFUSAL:#{reason}"
  exit 1
end

inventory = JSON.parse(File.binread(inventory_path))
refuse("SCHEMA") unless inventory.keys == %w[schema_version cli_boundary provider_births provider_guards] &&
  inventory.fetch("schema_version") == "taskseal.p06.zero-auth.entrypoint-inventory.v1"

cli = inventory.fetch("cli_boundary")
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

births = []
Dir.glob(File.join(root, "src/**/*.rs")).sort.each do |absolute|
  path = absolute.delete_prefix("#{root}/")
  File.readlines(absolute, chomp: true).each do |line|
    stripped = line.strip
    births << { "path" => path, "line" => stripped } if stripped.include?("Command::new(")
  end
end
Dir.glob(File.join(root, "scripts/probe/*.{sh,bash,zsh}")).sort.each do |absolute|
  path = absolute.delete_prefix("#{root}/")
  File.readlines(absolute, chomp: true).each do |line|
    stripped = line.strip
    next unless stripped.match?(/\b(?:codex|claude)\s+(?:--version|debug\s+prompt-input)\b/)

    births << { "path" => path, "line" => stripped }
  end
end
births.sort_by! { |entry| [entry.fetch("path"), entry.fetch("line")] }
expected_births = inventory.fetch("provider_births").sort_by { |entry| [entry.fetch("path"), entry.fetch("line")] }
refuse("PROVIDER_BIRTHS") unless births == expected_births

guards = inventory.fetch("provider_guards")
refuse("GUARD_PATHS") unless guards.map { |guard| guard.fetch("path") }.sort == births.map { |birth| birth.fetch("path") }.uniq.sort
guards.each do |guard|
  path = guard.fetch("path")
  source = File.binread(File.join(root, path))
  guard_index = source.index(guard.fetch("marker"))
  refuse("GUARD_MISSING:#{path}") unless guard_index
  births.select { |birth| birth.fetch("path") == path }.each do |birth|
    birth_index = source.index(birth.fetch("line"))
    refuse("BIRTH_MISSING:#{path}") unless birth_index
    refuse("GUARD_ORDER:#{path}") unless guard_index < birth_index
  end
end

puts "P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS"

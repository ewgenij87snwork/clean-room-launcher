#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "open3"

root = File.realpath(ARGV.fetch(0))
inventory_path = File.realpath(ARGV.fetch(1))

def refuse(reason)
  warn "P06_ZERO_AUTH_ENTRYPOINT_REFUSAL:#{reason}"
  exit 1
end

def tracked_probe_shell_sources(root)
  stdout, stderr, status = Open3.capture3(
    "git", "-C", root, "ls-files", "-s", "-z", "--", "scripts/probe"
  )
  refuse("PROBE_GIT_INVENTORY:#{stderr.strip}") unless status.success?

  stdout.split("\0").each_with_object([]) do |record, sources|
    next if record.empty?

    metadata, path = record.split("\t", 2)
    mode, _object, stage = metadata.split(" ", 3)
    refuse("PROBE_UNMERGED:#{path}") unless stage == "0"

    extension = File.extname(path)
    shell_extension = %w[.sh .bash .zsh].include?(extension)
    unless %w[100644 100755].include?(mode)
      refuse("PROBE_SOURCE_MODE:#{path}")
    end

    absolute = File.join(root, path)
    refuse("PROBE_SOURCE_TYPE:#{path}") if File.symlink?(absolute) || !File.file?(absolute)
    refuse("PROBE_EXTENSIONLESS_SOURCE:#{path}") if extension.empty?
    source = File.binread(absolute)
    shell_shebang = source.lines.first&.match?(%r{\A#!.*(?:^|[/[:space:]])(?:ba|z)?sh(?:[[:space:]]|$)})
    refuse("PROBE_SHELL_SOURCE_EXTENSION:#{path}") if shell_shebang && !shell_extension
    sources << [path, mode, source] if shell_extension
  end
end

def logical_shell_statements(source)
  statements = []
  parts = []
  start_index = nil

  source.lines(chomp: true).each_with_index do |line, index|
    start_index ||= index
    continued = line.match?(/\\[[:space:]]*\z/)
    parts << (continued ? line.sub(/\\[[:space:]]*\z/, "").strip : line.strip)
    next if continued

    statements << [start_index, parts.join(" ").strip]
    parts = []
    start_index = nil
  end
  refuse("PROBE_SHELL_MALFORMED_CONTINUATION") unless parts.empty?
  statements
end

def shell_tokens(statement)
  tokens = []
  word = +""
  quote = nil
  index = 0
  flush = lambda do
    unless word.empty?
      tokens << [:word, word]
      word = +""
    end
  end

  while index < statement.length
    character = statement[index]
    if quote
      if character == quote
        quote = nil
      elsif character == "\\" && quote == '"' && index + 1 < statement.length
        index += 1
        word << statement[index]
      else
        word << character
      end
    elsif character == "'" || character == '"'
      quote = character
    elsif character == "#" && (index.zero? || statement[index - 1].match?(/[[:space:];&|(){}]/))
      break
    elsif character.match?(/[[:space:]]/)
      flush.call
    elsif character == "\\"
      refuse("PROBE_SHELL_MALFORMED_ESCAPE") if index + 1 >= statement.length
      index += 1
      word << statement[index]
    elsif character == "`"
      refuse("PROBE_SHELL_AMBIGUOUS_BACKTICK")
    elsif character == "$" && statement[index + 1] == "("
      flush.call
      tokens << [:operator, "$("]
      index += 1
    elsif ";|&(){}".include?(character)
      flush.call
      operator = character
      if %w[& |].include?(character) && statement[index + 1] == character
        index += 1
        operator += character
      end
      tokens << [:operator, operator]
    elsif character == "<" && statement[index + 1] == "<"
      refuse("PROBE_SHELL_AMBIGUOUS_HEREDOC")
    else
      word << character
    end
    index += 1
  end
  refuse("PROBE_SHELL_MALFORMED_QUOTE") if quote
  flush.call
  tokens
end

def shell_provider_birth_count(statement)
  return 0 if statement.match?(/\A(?:codex|claude)\)[[:space:]]*\z/)

  tokens = shell_tokens(statement)
  command_position = true
  wrapper = false
  births = 0

  tokens.each_with_index do |(kind, value), index|
    if kind == :operator
      command_position = true
      wrapper = false
      next
    end

    next unless command_position

    if value.match?(/\A[A-Za-z_][A-Za-z0-9_]*=/) || %w[if then elif else do while until !].include?(value)
      next
    end
    if value == "command" && %w[-v -V].include?(tokens[index + 1]&.fetch(1, nil))
      command_position = false
      wrapper = false
      next
    end
    if %w[exec env command builtin nohup].include?(value)
      wrapper = true
      next
    end
    next if wrapper && value.start_with?("-")

    executable = File.basename(value)
    births += 1 if %w[codex claude].include?(executable)
    refuse("PROBE_SHELL_AMBIGUOUS_DISPATCH") if %w[eval xargs].include?(executable)
    if %w[sh bash zsh].include?(executable) && tokens[index + 1] == [:word, "-c"]
      refuse("PROBE_SHELL_AMBIGUOUS_SHELL_C")
    end
    command_position = false
    wrapper = false
  end

  births
end

inventory = JSON.parse(File.binread(inventory_path))
refuse("SCHEMA") unless inventory.keys == %w[schema_version cli_boundary shell_source_policy shell_sources provider_births provider_guards] &&
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

shell_policy = inventory.fetch("shell_source_policy")
refuse("SHELL_SOURCE_POLICY") unless shell_policy == {
  "root" => "scripts/probe",
  "inventory" => "GIT_TRACKED_RECURSIVE",
  "extensions" => %w[.sh .bash .zsh],
  "regular_modes" => %w[100644 100755],
  "extensionless_tracked_path" => "REFUSED_REGARDLESS_OF_CONTENT",
  "non_regular_or_symlink" => "REFUSED_REGARDLESS_OF_EXTENSION",
  "provider_command_positions" => "ALL_STATIC_DIRECT_WITH_MULTILINE_CONTINUATIONS",
  "ambiguous_dispatch" => "REFUSED",
  "guard" => "SAME_FILE_EXECUTABLE_BEFORE_EVERY_BIRTH_OCCURRENCE"
}

probe_shell_sources = tracked_probe_shell_sources(root)
actual_shell_sources = probe_shell_sources.map { |path, mode, _source| { "path" => path, "mode" => mode } }
refuse("PROBE_SHELL_SOURCES") unless actual_shell_sources == inventory.fetch("shell_sources")

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
probe_shell_sources.each do |path, _mode, source|
  logical_shell_statements(source).each do |index, statement|
    shell_provider_birth_count(statement).times do
      births << { "path" => path, "line" => statement }
      birth_positions[path] << index
    end
  end
end
births.sort_by! { |entry| [entry.fetch("path"), entry.fetch("line")] }
expected_births = inventory.fetch("provider_births").sort_by { |entry| [entry.fetch("path"), entry.fetch("line")] }
refuse("PROVIDER_BIRTHS") unless births == expected_births

guards = inventory.fetch("provider_guards")
refuse("GUARD_PATHS") unless guards.map { |guard| guard.fetch("path") }.sort == births.map { |birth| birth.fetch("path") }.uniq.sort
guards.each do |guard|
  path = guard.fetch("path")
  lines = File.readlines(File.join(root, path), chomp: true)
  marker = guard.fetch("marker")
  shell_source = path.start_with?("scripts/probe/")
  guard_positions = lines.each_index.select do |index|
    stripped = lines.fetch(index).strip
    next false if stripped.empty? || stripped.start_with?(shell_source ? "#" : "//")

    shell_source ? stripped == marker : stripped.include?(marker)
  end
  refuse("GUARD_MISSING:#{path}") if guard_positions.empty?
  birth_positions.fetch(path).each do |birth_index|
    refuse("GUARD_ORDER:#{path}") unless guard_positions.any? { |guard_index| guard_index < birth_index }
  end
end

puts "P06_ZERO_AUTH_ENTRYPOINT_INVENTORY_PASS"

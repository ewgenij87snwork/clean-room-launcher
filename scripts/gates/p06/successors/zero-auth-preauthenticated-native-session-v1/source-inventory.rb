#!/usr/bin/env ruby
# frozen_string_literal: true

require "open3"

root = File.realpath(ARGV.fetch(0) do
  File.expand_path("../../../../..", __dir__)
end)

stdout, stderr, status = Open3.capture3("git", "-C", root, "ls-files", "-s", "-z")
unless status.success?
  warn "P06_ZERO_AUTH_SOURCE_REFUSAL:GIT_INVENTORY:#{stderr.strip}"
  exit 1
end

entries = stdout.split("\0").each_with_object({}) do |record, inventory|
  next if record.empty?

  metadata, path = record.split("\t", 2)
  mode, = metadata.split(" ", 2)
  inventory[path] = mode
end

historical_paths = [
  "scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh",
  "scripts/gates/p06/t8-native-observe-once.sh"
].freeze
historical_stub = <<~'SH'
  #!/bin/sh
  printf '%s\n' HISTORICAL_ONLY_REFUSED >&2
  exit 78
SH

def refuse(reason, path)
  warn "P06_ZERO_AUTH_SOURCE_REFUSAL:#{reason}:#{path}"
  exit 1
end

historical_paths.each do |path|
  next unless entries.key?(path)

  refuse("HISTORICAL_EXECUTABLE", path) unless entries.fetch(path) == "100644"
  absolute = File.join(root, path)
  refuse("HISTORICAL_STUB", path) if File.symlink?(absolute) || !File.file?(absolute)
  refuse("HISTORICAL_STUB", path) unless File.binread(absolute) == historical_stub
end

forbidden = [
  ["AUTH_FILE", /(?:^|[^[:alnum:]_])auth\.json(?:$|[^[:alnum:]_])/i],
  ["CREDENTIAL_EXTRACTION", /\bplutil\b[^\n]{0,160}[[:space:]]-extract\b|\bsecurity\b[^\n]{0,160}\bfind-generic-password\b|\bjq\b[^\n]{0,160}(?:access[_-]?token|api[_-]?key|auth|credential|token)/i],
  ["CREDENTIAL_COPY", /\b(?:cp|install|rsync|ditto|dd)\b[^\n]{0,200}(?:access[_-]?token|api[_-]?key|auth|credential|token)/i],
  ["PROVIDER_LOGIN", /(?:\b(?:codex|claude|provider)\b|\$\{?(?:command|provider|codex|claude)[[:alnum:]_]*\}?)[^\n]{0,120}\b(?:login|auth(?:enticate|entication)?)\b/i],
  ["BROWSER_AUTH", /\b(?:open|xdg-open|start|webbrowser|browser)\b[^\n]{0,200}(?:oauth|device[ _-]*flow)|(?:oauth|device[ _-]*flow)[^\n]{0,200}\b(?:open|xdg-open|start|webbrowser|browser)\b/i],
  ["TOKEN_INPUT", /(?:--(?:api[_-]?key|with-access-token|access-token)|\bread\b[^\n]{0,120}(?:api[_-]?key|access[_-]?token|auth[_-]?token|provider[_-]?token)|\b(?:API_KEY|ACCESS_TOKEN|AUTH_TOKEN|PROVIDER_TOKEN)\b[[:space:]]*=)/i]
].freeze

entries.sort.each do |path, mode|
  next unless mode == "100755"
  next if path.start_with?("scripts/gates/", "fixtures/")

  absolute = File.join(root, path)
  refuse("EXECUTABLE_SYMLINK", path) if File.symlink?(absolute) || !File.file?(absolute)
  source = File.binread(absolute)
  forbidden.each do |reason, pattern|
    refuse(reason, path) if source.match?(pattern)
  end
end

puts "P06_ZERO_AUTH_SOURCE_INVENTORY_PASS"

#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"
require "open3"

root = File.realpath(ARGV.fetch(0) do
  File.expand_path("../../../../..", __dir__)
end)
list_only = ARGV[1] == "--list-hits"
allowlist_path = File.join(
  root,
  "scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/source-inventory-allowlist.json"
)

def refuse(reason, path)
  warn "P06_ZERO_AUTH_SOURCE_REFUSAL:#{reason}:#{path}"
  exit 1
end

stdout, stderr, status = Open3.capture3("git", "-C", root, "ls-files", "-s", "-z")
refuse("GIT_INVENTORY", stderr.strip) unless status.success?

entries = stdout.split("\0").each_with_object({}) do |record, inventory|
  next if record.empty?

  metadata, path = record.split("\t", 2)
  mode, object, stage = metadata.split(" ", 3)
  refuse("UNMERGED_INDEX", path) unless stage == "0"
  inventory[path] = { "mode" => mode, "object" => object }
end

historical_stub = <<~'SH'
  #!/bin/sh
  printf '%s\n' HISTORICAL_ONLY_REFUSED >&2
  exit 78
SH
historical_paths = [
  "scripts/gates/p06/successors/observation-capability-v1-phase2/run-once.sh",
  "scripts/gates/p06/successors/observation-capability-v1/probe-local.sh",
  "scripts/gates/p06/successors/observation-capability-v1/verify.sh",
  "scripts/gates/p06/t8-native-observe-once.sh"
].freeze

historical_paths.each do |path|
  next unless entries.key?(path)

  refuse("HISTORICAL_EXECUTABLE", path) unless entries.fetch(path).fetch("mode") == "100644"
  absolute = File.join(root, path)
  refuse("HISTORICAL_STUB", path) if File.symlink?(absolute) || !File.file?(absolute)
  refuse("HISTORICAL_STUB", path) unless File.binread(absolute) == historical_stub
end

forbidden = [
  ["AUTH_FILE", /(?:^|[^[:alnum:]_])auth\.json(?:$|[^[:alnum:]_])/im],
  ["CREDENTIAL_EXTRACTION", /\bplutil\b.{0,500}[[:space:]"']-extract\b|\bsecurity\b.{0,500}\bfind-generic-password\b|\bjq\b.{0,500}(?:access[_-]?token|api[_-]?key|credential(?:[_-](?:field|source))?)/im],
  ["CREDENTIAL_COPY", /\b(?:cp|install|rsync|ditto|dd|std::fs::copy|fs::copy)\b.{0,700}(?:access[_-]?token|api[_-]?key|auth|credential|token)|(?:access[_-]?token|api[_-]?key|auth|credential|token).{0,700}\b(?:cp|install|rsync|ditto|dd|std::fs::copy|fs::copy)\b/im],
  ["PROVIDER_LOGIN", /(?:\b(?:Command::new|process::Command|clean_env|sandbox-exec|exec|spawn|system)\b|\$\{?[A-Za-z_][A-Za-z0-9_]*\}?).{0,1200}(?:["'](?:login|auth(?:enticate|entication)?)["']|[[:space:]](?:login|auth(?:enticate|entication)?)(?:[[:space:]]|--|$))|(?:["'](?:login|auth(?:enticate|entication)?)["']|[[:space:]](?:login|auth(?:enticate|entication)?)(?:[[:space:]]|--|$)).{0,1200}(?:\b(?:Command::new|process::Command|clean_env|sandbox-exec|spawn|system)\b|\.status\s*\(|\.output\s*\()|\b(?:subcommand|action|verb|mode)\b\s*=\s*["'](?:login|auth(?:enticate|entication)?)["']|(?:^|\n)\s*(?!#)[A-Za-z0-9_.\/$"'{}=-]+[^\n;|&]{0,300}[[:space:]](?:login|auth(?:enticate|entication)?)(?:[[:space:]]|--|$)/im],
  ["BROWSER_AUTH", /\b(?:open|xdg-open|start|webbrowser|browser|Command::new)\b.{0,1000}(?:oauth|device[ _-]*flow)|(?:oauth|device[ _-]*flow).{0,1000}\b(?:open|xdg-open|start|webbrowser|browser|spawn|status|output)\b/im],
  ["TOKEN_INPUT", /--(?:api[_-]?key|with-access-token|access-token)|\bread\b.{0,500}(?:api[_-]?key|access[_-]?token|auth[_-]?token|provider[_-]?token)|\b[A-Z0-9_]*(?:API_KEY|ACCESS_TOKEN|AUTH_TOKEN|PROVIDER_TOKEN)\b\s*=|(?:env::var|std::env::var|var_os)\s*\(\s*["'][A-Z0-9_]*(?:API_KEY|ACCESS_TOKEN|AUTH_TOKEN|PROVIDER_TOKEN)["']/im]
].freeze

candidates = {}
entries.sort.each do |path, metadata|
  mode = metadata.fetch("mode")
  refuse("TRACKED_SYMLINK", path) if mode == "120000"
  next if historical_paths.include?(path)
  next unless mode == "100755" || path == "build.rs" || (path.start_with?("src/") && path.end_with?(".rs")) || begin
    absolute = File.join(root, path)
    File.file?(absolute) && !File.symlink?(absolute) && File.binread(absolute, 2) == "#!"
  end

  absolute = File.join(root, path)
  refuse("SOURCE_TYPE", path) if File.symlink?(absolute) || !File.file?(absolute)
  source = File.binread(absolute)
  hits = forbidden.each_with_object([]) do |(reason, pattern), matched|
    matched << reason if source.match?(pattern)
  end
  candidates[path] = {
    "sha256" => Digest::SHA256.hexdigest(source),
    "matched_classes" => hits
  }
end

if list_only
  puts JSON.pretty_generate(candidates.select { |_path, facts| !facts.fetch("matched_classes").empty? })
  exit 0
end

allowlist = if File.file?(allowlist_path)
  JSON.parse(File.binread(allowlist_path))
else
  { "schema_version" => "taskseal.p06.zero-auth.source-inventory-allowlist.v1", "entries" => [] }
end
refuse("ALLOWLIST_SCHEMA", "source-inventory-allowlist.json") unless allowlist.keys.sort == ["entries", "schema_version"] &&
  allowlist.fetch("schema_version") == "taskseal.p06.zero-auth.source-inventory-allowlist.v1" &&
  allowlist.fetch("entries").is_a?(Array)

allowed = {}
allowlist.fetch("entries").each do |entry|
  unless entry.keys.sort == ["matched_classes", "path", "rationale", "sha256"] &&
      entry.fetch("path").is_a?(String) && entry.fetch("sha256").match?(/\A[0-9a-f]{64}\z/) &&
      entry.fetch("matched_classes").is_a?(Array) && !entry.fetch("matched_classes").empty? &&
      entry.fetch("rationale").is_a?(String) && entry.fetch("rationale").length >= 24
    refuse("ALLOWLIST_ENTRY", entry.fetch("path", "UNKNOWN"))
  end
  refuse("ALLOWLIST_DUPLICATE", entry.fetch("path")) if allowed.key?(entry.fetch("path"))
  allowed[entry.fetch("path")] = entry
end
refuse("ALLOWLIST_ORDER", "source-inventory-allowlist.json") unless allowed.keys == allowed.keys.sort

actual_hit_paths = candidates.select { |_path, facts| !facts.fetch("matched_classes").empty? }.keys
actual_hit_paths.each do |path|
  facts = candidates.fetch(path)
  entry = allowed[path]
  refuse(facts.fetch("matched_classes").first, path) unless entry
  refuse("ALLOWLIST_DIGEST", path) unless entry.fetch("sha256") == facts.fetch("sha256")
  refuse("ALLOWLIST_CLASSES", path) unless entry.fetch("matched_classes") == facts.fetch("matched_classes")
end
(allowed.keys - actual_hit_paths).each { |path| refuse("ALLOWLIST_STALE", path) }

puts "P06_ZERO_AUTH_SOURCE_INVENTORY_PASS"

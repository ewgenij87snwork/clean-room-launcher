#!/usr/bin/env ruby
path = ARGV.fetch(0, 'controls/v0.1-execution-map.tsv')
rows = File.readlines(path, chomp: true)
abort 'HEADER_MISSING' unless rows.shift == "control_id\tdisposition\ttarget\texecution_owner\tcoverage_status\tacceptance\tevidence_destination\trequirement\tsource_file\tsource_line"
abort 'CONTROL_COUNT' unless rows.length == 445
ids = rows.map { |row| row.split("\t", -1).first }
abort 'DUPLICATE_CONTROL' unless ids.uniq.length == ids.length
abort 'MISSING_TRACE' if rows.any? { |row| fields = row.split("\t", -1); fields.length != 10 || fields[0].empty? || fields[5].empty? || fields[6].empty? }
puts '445/445 explicit controls pass'

#!/usr/bin/env ruby
# frozen_string_literal: true

require "pathname"

ROOT = Pathname.new(__dir__).parent
PATTERN = /(?:\.(?:unwrap|expect)\s*\(|\bpanic!\s*\()/.freeze

def rust_files
  paths = ARGV.empty? ? Dir.glob(ROOT.join("crates/**/*.rs")) : ARGV
  paths.map { |path| Pathname.new(path) }.reject do |path|
    path_string = path.to_s
    path_string.include?("/tests/") ||
      path_string.include?("/benches/") ||
      path_string.include?("/examples/") ||
      path.basename.to_s.end_with?("_tests.rs") ||
      path.basename.to_s.end_with?("render_tests.rs") ||
      path.basename.to_s == "build.rs"
  end
end

def display_path(path)
  path = Pathname.new(path)
  if path.absolute?
    begin
      path.relative_path_from(ROOT).to_s
    rescue ArgumentError
      path.to_s
    end
  else
    path.to_s
  end
end

def brace_delta(line)
  line.count("{") - line.count("}")
end

violations = []

rust_files.each do |path|
  cfg_test_pending = false
  test_depth = nil
  depth = 0

  File.foreach(path).with_index(1) do |line, line_number|
    stripped = line.strip

    cfg_test_pending = true if stripped == "#[cfg(test)]"

    if test_depth.nil? && cfg_test_pending && stripped.match?(/\bmod\s+tests\b/)
      test_depth = depth + brace_delta(line)
      cfg_test_pending = false
      depth += brace_delta(line)
      next
    end

    cfg_test_pending = false unless stripped.start_with?("#[") || stripped.empty?

    if test_depth.nil? && line.match?(PATTERN)
      violations << "#{display_path(path)}:#{line_number}: #{stripped}"
    end

    depth += brace_delta(line)
    test_depth = nil if !test_depth.nil? && depth < test_depth
  end
end

if violations.any?
  warn "panic-prone calls found in non-test Rust code:"
  violations.each { |violation| warn violation }
  exit 1
end

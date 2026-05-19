#!/usr/bin/env ruby
# frozen_string_literal: true

require "pathname"

ROOT = Pathname.new(__dir__).parent
PATTERN = /\.(unwrap|expect)\s*\(/.freeze

def rust_files
  Dir.chdir(ROOT) do
    Dir.glob("crates/**/*.rs").reject do |path|
      path.include?("/tests/") ||
        path.include?("/benches/") ||
        path.include?("/examples/") ||
        File.basename(path).end_with?("_tests.rs") ||
        File.basename(path).end_with?("render_tests.rs") ||
        File.basename(path) == "build.rs"
    end
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

  File.foreach(ROOT.join(path)).with_index(1) do |line, line_number|
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
      violations << "#{path}:#{line_number}: #{stripped}"
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

#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
SCRIPT = File.join(ROOT, "scripts", "check-package-versions.sh")

def write_fixture(root, workspace:, flake:, homebrew:)
  FileUtils.mkdir_p(File.join(root, "packaging"))

  File.write(
    File.join(root, "Cargo.toml"),
    <<~TOML,
      [workspace.package]
      version = "#{workspace}"
    TOML
  )

  File.write(
    File.join(root, "flake.nix"),
    <<~NIX,
      {
        packages.default = {
          version = "#{flake}";
        };
      }
    NIX
  )

  File.write(
    File.join(root, "packaging", "homebrew-formula.rb"),
    <<~RUBY,
      class HoogleTui < Formula
        version "#{homebrew}"
      end
    RUBY
  )
end

def run_checker(root)
  _stdout, stderr, status = Open3.capture3(SCRIPT, root)
  [status.success?, stderr]
end

def assert_passes(name, **versions)
  Dir.mktmpdir("check-package-versions") do |dir|
    write_fixture(dir, **versions)
    success, stderr = run_checker(dir)
    return if success

    warn "#{name} should have passed"
    warn stderr
    exit 1
  end
end

def assert_fails(name, expected, **versions)
  Dir.mktmpdir("check-package-versions") do |dir|
    write_fixture(dir, **versions)
    success, stderr = run_checker(dir)
    if success
      warn "#{name} should have failed"
      exit 1
    end

    return if stderr.include?(expected)

    warn "#{name} failed without expected output"
    warn "expected: #{expected}"
    warn stderr
    exit 1
  end
end

assert_passes(
  "matching versions",
  workspace: "1.2.3",
  flake: "1.2.3",
  homebrew: "1.2.3",
)

assert_fails(
  "flake mismatch",
  "flake.nix version (1.2.4) does not match workspace version (1.2.3)",
  workspace: "1.2.3",
  flake: "1.2.4",
  homebrew: "1.2.3",
)

assert_fails(
  "homebrew mismatch",
  "Homebrew formula version (1.2.4) does not match workspace version (1.2.3)",
  workspace: "1.2.3",
  flake: "1.2.3",
  homebrew: "1.2.4",
)

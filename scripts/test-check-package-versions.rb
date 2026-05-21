#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
SCRIPT = File.join(ROOT, "scripts", "check-package-versions.sh")

def write_fixture(root, workspace:, flake:, homebrew:)
  FileUtils.mkdir_p(File.join(root, "packaging"))
  FileUtils.mkdir_p(File.join(root, "crates", "hoogle-core"))
  FileUtils.mkdir_p(File.join(root, "crates", "hoogle-syntax"))
  FileUtils.mkdir_p(File.join(root, "crates", "hoogle-tui"))

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

  File.write(
    File.join(root, "crates", "hoogle-core", "Cargo.toml"),
    <<~TOML,
      [package]
      name = "hoogle-core"
      version = "#{workspace}"
    TOML
  )

  File.write(
    File.join(root, "crates", "hoogle-syntax", "Cargo.toml"),
    <<~TOML,
      [package]
      name = "hoogle-syntax"
      version = "#{workspace}"
    TOML
  )

  File.write(
    File.join(root, "crates", "hoogle-tui", "Cargo.toml"),
    <<~TOML,
      [package]
      name = "hoogle-tui"
      version = "#{workspace}"

      [dependencies]
      hoogle-core = { path = "../hoogle-core", version = "#{workspace}" }
      hoogle-syntax = { path = "../hoogle-syntax", version = "#{workspace}" }
    TOML
  )

  File.write(
    File.join(root, "Cargo.lock"),
    <<~TOML,
      [[package]]
      name = "hoogle-core"
      version = "#{workspace}"

      [[package]]
      name = "hoogle-syntax"
      version = "#{workspace}"

      [[package]]
      name = "hoogle-tui"
      version = "#{workspace}"
    TOML
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

Dir.mktmpdir("check-package-versions") do |dir|
  write_fixture(dir, workspace: "1.2.3", flake: "1.2.3", homebrew: "1.2.3")
  File.write(
    File.join(dir, "crates", "hoogle-core", "Cargo.toml"),
    <<~TOML,
      [package]
      name = "hoogle-core"
      version = "1.2.4"
    TOML
  )

  success, stderr = run_checker(dir)
  if success
    warn "crate version mismatch should have failed"
    exit 1
  end

  expected = "hoogle-core/Cargo.toml version (1.2.4) does not match workspace version (1.2.3)"
  unless stderr.include?(expected)
    warn "crate version mismatch failed without expected output"
    warn "expected: #{expected}"
    warn stderr
    exit 1
  end
end

Dir.mktmpdir("check-package-versions") do |dir|
  write_fixture(dir, workspace: "1.2.3", flake: "1.2.3", homebrew: "1.2.3")
  File.write(
    File.join(dir, "crates", "hoogle-tui", "Cargo.toml"),
    <<~TOML,
      [package]
      name = "hoogle-tui"
      version = "1.2.3"

      [dependencies]
      hoogle-core = { path = "../hoogle-core", version = "1.2.3" }
      hoogle-syntax = { path = "../hoogle-syntax", version = "1.2.4" }
    TOML
  )

  success, stderr = run_checker(dir)
  if success
    warn "second path dependency version mismatch should have failed"
    exit 1
  end

  expected = "hoogle-tui/Cargo.toml path dependency version (1.2.4) does not match workspace version (1.2.3)"
  unless stderr.include?(expected)
    warn "second path dependency version mismatch failed without expected output"
    warn "expected: #{expected}"
    warn stderr
    exit 1
  end
end

Dir.mktmpdir("check-package-versions") do |dir|
  write_fixture(dir, workspace: "1.2.3", flake: "1.2.3", homebrew: "1.2.3")
  File.write(
    File.join(dir, "crates", "hoogle-tui", "Cargo.toml"),
    <<~TOML,
      [package]
      name = "hoogle-tui"
      version = "1.2.3"

      [dependencies]
      hoogle-core = { path = "../hoogle-core", version = "1.2.4" }
    TOML
  )

  success, stderr = run_checker(dir)
  if success
    warn "path dependency version mismatch should have failed"
    exit 1
  end

  expected = "hoogle-tui/Cargo.toml path dependency version (1.2.4) does not match workspace version (1.2.3)"
  unless stderr.include?(expected)
    warn "path dependency version mismatch failed without expected output"
    warn "expected: #{expected}"
    warn stderr
    exit 1
  end
end

Dir.mktmpdir("check-package-versions") do |dir|
  write_fixture(dir, workspace: "1.2.3", flake: "1.2.3", homebrew: "1.2.3")
  File.write(
    File.join(dir, "Cargo.lock"),
    <<~TOML,
      [[package]]
      name = "hoogle-core"
      version = "1.2.3"

      [[package]]
      name = "hoogle-syntax"
      version = "1.2.4"

      [[package]]
      name = "hoogle-tui"
      version = "1.2.3"
    TOML
  )

  success, stderr = run_checker(dir)
  if success
    warn "lockfile version mismatch should have failed"
    exit 1
  end

  expected = "Cargo.lock hoogle-syntax version (1.2.4) does not match workspace version (1.2.3)"
  unless stderr.include?(expected)
    warn "lockfile version mismatch failed without expected output"
    warn "expected: #{expected}"
    warn stderr
    exit 1
  end
end

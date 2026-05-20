#!/usr/bin/env sh
set -eu

root="${1:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}"

workspace_version="$(awk -F'"' '/^version = / {print $2; exit}' "$root/Cargo.toml")"
flake_version="$(awk -F'"' '/version = / {print $2; exit}' "$root/flake.nix")"
homebrew_version="$(awk -F'"' '/version / {print $2; exit}' "$root/packaging/homebrew-formula.rb")"

if [ "$flake_version" != "$workspace_version" ]; then
  echo "flake.nix version ($flake_version) does not match workspace version ($workspace_version)" >&2
  exit 1
fi

if [ "$homebrew_version" != "$workspace_version" ]; then
  echo "Homebrew formula version ($homebrew_version) does not match workspace version ($workspace_version)" >&2
  exit 1
fi

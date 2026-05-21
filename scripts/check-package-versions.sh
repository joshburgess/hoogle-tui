#!/usr/bin/env sh
set -eu

root="${1:-$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)}"

workspace_version="$(awk -F'"' '/^version = / {print $2; exit}' "$root/Cargo.toml")"
flake_version="$(awk -F'"' '/version = / {print $2; exit}' "$root/flake.nix")"
homebrew_version="$(awk -F'"' '/version / {print $2; exit}' "$root/packaging/homebrew-formula.rb")"

for manifest in "$root"/crates/*/Cargo.toml; do
  crate_version="$(awk -F'"' '/^version = / {print $2; exit}' "$manifest")"
  if [ "$crate_version" != "$workspace_version" ]; then
    echo "$manifest version ($crate_version) does not match workspace version ($workspace_version)" >&2
    exit 1
  fi

  path_dependency_version="$(
    awk -F'version = "' '/path = "\.\.\// {split($2, parts, "\""); print parts[1]; exit}' "$manifest"
  )"
  if [ -n "$path_dependency_version" ] && [ "$path_dependency_version" != "$workspace_version" ]; then
    echo "$manifest path dependency version ($path_dependency_version) does not match workspace version ($workspace_version)" >&2
    exit 1
  fi
done

if [ "$flake_version" != "$workspace_version" ]; then
  echo "flake.nix version ($flake_version) does not match workspace version ($workspace_version)" >&2
  exit 1
fi

if [ "$homebrew_version" != "$workspace_version" ]; then
  echo "Homebrew formula version ($homebrew_version) does not match workspace version ($workspace_version)" >&2
  exit 1
fi

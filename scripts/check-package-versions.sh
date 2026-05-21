#!/usr/bin/env sh
set -eu

root="${1:-$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)}"

workspace_version="$(awk -F'"' '/^version = / {print $2; exit}' "$root/Cargo.toml")"
flake_version="$(awk -F'"' '/version = / {print $2; exit}' "$root/flake.nix")"
homebrew_version="$(awk -F'"' '/version / {print $2; exit}' "$root/packaging/homebrew-formula.rb")"

for manifest in "$root"/crates/*/Cargo.toml; do
  crate_version="$(awk -F'"' '/^version = / {print $2; exit}' "$manifest")"
  crate_name="$(awk -F'"' '/^name = / {print $2; exit}' "$manifest")"
  if [ "$crate_version" != "$workspace_version" ]; then
    echo "$manifest version ($crate_version) does not match workspace version ($workspace_version)" >&2
    exit 1
  fi

  lock_version="$(
    awk -F'"' -v crate_name="$crate_name" '
      /^\[\[package\]\]/ { in_package = 1; name = ""; version = ""; next }
      in_package && /^name = / { name = $2; next }
      in_package && /^version = / {
        version = $2
        if (name == crate_name) {
          print version
          exit
        }
      }
    ' "$root/Cargo.lock"
  )"
  if [ "$lock_version" != "$workspace_version" ]; then
    echo "Cargo.lock $crate_name version ($lock_version) does not match workspace version ($workspace_version)" >&2
    exit 1
  fi

  awk -F'version = "' '/path = "\.\.\// {split($2, parts, "\""); print parts[1]}' "$manifest" |
  while IFS= read -r path_dependency_version; do
    if [ "$path_dependency_version" != "$workspace_version" ]; then
      echo "$manifest path dependency version ($path_dependency_version) does not match workspace version ($workspace_version)" >&2
      exit 1
    fi
  done
done

if [ "$flake_version" != "$workspace_version" ]; then
  echo "flake.nix version ($flake_version) does not match workspace version ($workspace_version)" >&2
  exit 1
fi

if [ "$homebrew_version" != "$workspace_version" ]; then
  echo "Homebrew formula version ($homebrew_version) does not match workspace version ($workspace_version)" >&2
  exit 1
fi

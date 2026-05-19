#!/usr/bin/env sh
set -eu

cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features

em_dash="$(printf '\342\200\224')"
if rg -n "$em_dash" README.md CHANGELOG.md docs crates .github; then
  echo "em dash found in user-facing text" >&2
  exit 1
fi

if rg -n "use super::\\*" crates/hoogle-tui/src/app_*.rs; then
  echo "wildcard app module import found" >&2
  exit 1
fi

if [ "${RUN_SMOKE_TESTS:-0}" = "1" ]; then
  cargo test -p hoogle-core integration_local_search -- --ignored
  cargo test -p hoogle-core integration_web_search -- --ignored
fi

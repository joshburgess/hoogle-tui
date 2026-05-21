#!/usr/bin/env sh
set -eu

cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo package -p hoogle-core --allow-dirty --offline
cargo package -p hoogle-syntax --allow-dirty --offline
sh -n scripts/check-package-versions.sh
sh -n scripts/verify.sh
ruby -c scripts/check-rust-panics.rb >/dev/null
ruby -c scripts/test-check-rust-panics.rb >/dev/null
ruby -c scripts/test-check-package-versions.rb >/dev/null
ruby -c packaging/homebrew-formula.rb >/dev/null
if command -v actionlint >/dev/null 2>&1; then
  actionlint
fi
if command -v shellcheck >/dev/null 2>&1; then
  shellcheck scripts/verify.sh scripts/check-package-versions.sh
fi
scripts/test-check-rust-panics.rb
scripts/check-rust-panics.rb
scripts/test-check-package-versions.rb
scripts/check-package-versions.sh

scan() {
  pattern="$1"
  shift
  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$@"
  else
    grep -RIn --exclude-dir target "$pattern" "$@"
  fi
}

em_dash="$(printf '\342\200\224')"
if scan "$em_dash" README.md CHANGELOG.md docs crates .github; then
  echo "em dash found in user-facing text" >&2
  exit 1
fi

if scan "use super::\\*" crates/hoogle-tui/src/app_*.rs; then
  echo "wildcard app module import found" >&2
  exit 1
fi

if scan "#\\[allow(dead_code)\\]" crates/hoogle-core/src crates/hoogle-tui/src crates/hoogle-syntax/src; then
  echo "dead-code allowance found in shipped Rust code" >&2
  exit 1
fi

if [ "${RUN_SMOKE_TESTS:-0}" = "1" ]; then
  cargo test -p hoogle-core integration_local_search -- --ignored
  cargo test -p hoogle-core integration_web_search -- --ignored
fi

if [ "${RUN_AUDIT:-0}" = "1" ]; then
  if ! command -v cargo-audit >/dev/null 2>&1; then
    echo "RUN_AUDIT=1 requires cargo-audit to be installed" >&2
    exit 1
  fi
  cargo audit
fi

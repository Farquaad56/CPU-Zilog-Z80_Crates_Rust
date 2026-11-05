#!/usr/bin/env bash
# Full check before tagging a release. Stops at the first failure.
# The suites take about 20 minutes (ZEXDOC and ZEXALL in release mode).
set -eu
cd "$(dirname "$0")/.."

# Git Bash on Windows does not always have the Rust toolchain in its PATH.
if ! command -v cargo >/dev/null 2>&1; then
  export PATH="${HOME:-}/.cargo/bin:${USERPROFILE:-}/.cargo/bin:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found (install Rust from https://rustup.rs)" >&2
  exit 1
fi
PY=python
command -v python >/dev/null 2>&1 || PY=python3

step() { echo; echo "=== $*"; }

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo test --workspace"
cargo test --workspace

step "hygiene"
bash tools/hygiene.sh

step "cargo build --release --workspace"
cargo build --release --workspace

step "test suites (suite all)"
cargo run --release -p puce-z80-cli -- suite all --json suites.json

step "suites report"
"$PY" docs/tools/suites_report.py suites.json

echo
echo "release_check: all checks passed"

#!/usr/bin/env bash
# Full check before tagging a release. Stops at the first failure.
# The suites take about 20 minutes (ZEXDOC and ZEXALL in release mode).
set -eu
cd "$(dirname "$0")/.."

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
python docs/tools/suites_report.py suites.json

echo
echo "release_check: all checks passed"

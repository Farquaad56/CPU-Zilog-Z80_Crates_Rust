#!/bin/bash
set -e

echo "=== CI: cargo fmt check ==="
cargo fmt --all -- --check

echo "=== CI: cargo clippy (warnings as errors) ==="
cargo clippy --workspace --all-targets -- -D warnings

echo "=== CI: cargo test ==="
cargo test --workspace

echo "=== CI: all checks passed ==="

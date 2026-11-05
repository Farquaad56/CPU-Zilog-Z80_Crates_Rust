#!/usr/bin/env bash
# Checks that puce-z80 builds without std on an embedded target (and on WASM when available).
# Prints SKIP and exits 0 when a target cannot be installed (no network).
set -u
cd "$(dirname "$0")/.." || exit 1

EMBEDDED=thumbv7em-none-eabihf
WASM=wasm32-unknown-unknown

if ! rustup target add "$EMBEDDED" >/dev/null 2>&1 && \
   ! rustup target list --installed 2>/dev/null | grep -qx "$EMBEDDED"; then
  echo "SKIP: cannot install $EMBEDDED (no network?)"
  exit 0
fi

echo "== cargo build -p puce-z80 --target $EMBEDDED"
cargo build -p puce-z80 --target "$EMBEDDED" || { echo "FAIL: $EMBEDDED"; exit 1; }

if rustup target list --installed 2>/dev/null | grep -qx "$WASM"; then
  echo "== cargo build -p puce-z80 --target $WASM"
  cargo build -p puce-z80 --target "$WASM" || { echo "FAIL: $WASM"; exit 1; }
else
  echo "note: $WASM is not installed, WASM build skipped"
fi

echo "OK: puce-z80 builds without std"

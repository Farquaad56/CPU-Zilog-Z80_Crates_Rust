#!/usr/bin/env bash
# Browser end-to-end test (optional): prints SKIP and exits 0 when Node or Playwright is missing.
# Playwright is looked up with Node's usual rules plus $NODE_PATH.
set -u
cd "$(dirname "$0")/../.." || exit 1

if ! command -v node >/dev/null 2>&1; then
  echo "SKIP: node not found"
  exit 0
fi
if ! node -e "require('playwright')" >/dev/null 2>&1; then
  echo "SKIP: playwright not installed (npm i playwright && npx playwright install chromium)"
  exit 0
fi

cargo build --release -p puce-z80-web || exit 1
node tests/e2e/browser.mjs

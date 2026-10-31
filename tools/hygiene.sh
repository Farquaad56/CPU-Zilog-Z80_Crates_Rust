#!/bin/bash
set -e

TARGET_DIR="${1:-crates/puce-z80/src}"

if [ ! -d "$TARGET_DIR" ]; then
    echo "ERROR: directory not found: $TARGET_DIR"
    exit 1
fi

echo "=== Hygiene check on $TARGET_DIR ==="

FAIL=0

for f in $(find "$TARGET_DIR" -name '*.rs' -type f); do
    # Check for actual unsafe code (not the forbid attribute)
    if grep -nE 'unsafe\s+\{|unsafe\s+fn|unsafe\s+impl|unsafe\s+extern' "$f" > /dev/null 2>&1; then
        echo "FAIL: $f contains unsafe code:"
        grep -nE 'unsafe\s+\{|unsafe\s+fn|unsafe\s+impl|unsafe\s+extern' "$f" | head -5
        FAIL=1
    fi

    # Check for f32/f64
    if grep -nE '\bf32\b|\bf64\b' "$f" > /dev/null 2>&1; then
        echo "FAIL: $f contains floating point types:"
        grep -nE '\bf32\b|\bf64\b' "$f" | head -5
        FAIL=1
    fi

    # Check for std:: or extern crate std (allowed only in test-cfg files)
    if grep -nE 'std::|extern crate std;' "$f" > /dev/null 2>&1; then
        HAS_TEST_CFG=$(grep -c '#\[cfg(test)\]\|#\[cfg(any(test, feature = "std"))\]' "$f")
        if [ "$HAS_TEST_CFG" -eq 0 ]; then
            echo "FAIL: $f uses std without test cfg:"
            grep -nE 'std::|extern crate std;' "$f" | head -5
            FAIL=1
        fi
    fi

    # Check for println! (forbidden everywhere in puce-z80)
    if grep -nE '\bprintln!\(' "$f" > /dev/null 2>&1; then
        echo "FAIL: $f contains println!:"
        grep -nE '\bprintln!\(' "$f" | head -5
        FAIL=1
    fi

    # Check for unwrap() outside test-cfg files
    if grep -nE '\.unwrap\(\)' "$f" > /dev/null 2>&1; then
        HAS_TEST_CFG=$(grep -c '#\[cfg(test)\]\|#\[cfg(any(test, feature = "std"))\]' "$f")
        if [ "$HAS_TEST_CFG" -eq 0 ]; then
            echo "FAIL: $f uses .unwrap() outside test context:"
            grep -nE '\.unwrap\(\)' "$f" | head -5
            FAIL=1
        fi
    fi

    # Check for non-ASCII characters
    if grep -nP '[^\x00-\x7F]' "$f" > /dev/null 2>&1; then
        echo "FAIL: $f contains non-ASCII characters:"
        grep -nPn '[^\x00-\x7F]' "$f" | head -5
        FAIL=1
    fi

    # Check for TODO without UNKNOWN - to confirm on same line
    if grep -nE '\bTODO\b' "$f" > /dev/null 2>&1; then
        BAD_TODOS=$(grep -nE '\bTODO\b' "$f" | grep -v 'UNKNOWN - to confirm')
        if [ -n "$BAD_TODOS" ]; then
            echo "FAIL: $f has TODO without 'UNKNOWN - to confirm':"
            echo "$BAD_TODOS" | head -5
            FAIL=1
        fi
    fi
done

if [ "$FAIL" -eq 0 ]; then
    echo "=== Hygiene check passed ==="
else
    echo "=== Hygiene check FAILED ==="
fi

exit $FAIL

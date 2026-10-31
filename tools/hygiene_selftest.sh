#!/bin/bash
set -e

echo "=== Hygiene selftest ==="

# Create temp directory with a file that should fail hygiene check
TMPDIR=$(mktemp -d)
cat > "$TMPDIR/bad.rs" << 'EOF'
let x: f32 = 1.0;
EOF

# Run hygiene check on the temp dir - expect failure (exit 1)
if bash tools/hygiene.sh "$TMPDIR"; then
    echo "FAIL: hygiene.sh should have failed on f32 but passed"
    rm -rf "$TMPDIR"
    exit 1
fi

echo "PASS: hygiene.sh correctly detected f32 violation"

# Clean up
rm -rf "$TMPDIR"

echo "=== Hygiene selftest passed ==="

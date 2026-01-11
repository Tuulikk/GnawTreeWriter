#!/bin/bash
# GnawTreeWriter Time Machine Demo
# This script guides you through rolling back the demo project.

BINARY="../../target/release/gnawtreewriter"
DEMO_FILE="notes.py"

echo "=== 🕰️ GnawTreeWriter Time Machine Demo ==="
echo "Current state of $DEMO_FILE:"
cat "$DEMO_FILE"
echo "------------------------------------------"

echo "Listing recent history..."
"$BINARY" history --limit 5
echo "------------------------------------------"

echo "To go back to Phase 1 (no imports, no timestamps):"
echo "Run: $BINARY restore-project \"$(date +%Y-%m-%d) 15:00:00\" --preview"
echo ""
echo "Try running the above command to see what would happen!"

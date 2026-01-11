#!/bin/bash
# License Guardian
echo "🔍 Checking licenses..."
P1="[M]IT License"
P2="Licensed under [M]IT"
P3="License: [M]IT"
FORBIDDEN=$(grep -rEi "$P1|$P2|$P3" . --exclude-dir=".git" --exclude-dir="target" --exclude-dir="node_modules" --exclude-dir=".gnawtreewriter_backups" --exclude-dir=".gnawtreewriter_ai" --exclude="MPL-2.0-GUIDE.md" --exclude="scripts/check-license.sh")
if [ -n "$FORBIDDEN" ]; then
    echo "❌ ERROR: Unauthorized license reference found!"
    echo "$FORBIDDEN"
    exit 1
fi
echo "✅ Success: Project is pure MPL-2.0."
exit 0
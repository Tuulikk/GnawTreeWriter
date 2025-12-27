#!/bin/bash
# sync-docs.sh - GnawTreeWriter Documentation Synchronization Script
#
# This script syncs documentation from the main project to the GnawTreeWriter_docs
# handbook directory, ensuring the handbook can be used as a standalone resource.

set -e  # Exit on any error

echo "🔄 Syncing GnawTreeWriter documentation..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "GnawTreeWriter_docs" ]; then
    echo "❌ Error: Please run this script from the GnawTreeWriter root directory"
    echo "   Expected: Cargo.toml and GnawTreeWriter_docs/ directory"
    exit 1
fi

# Create backup timestamp
BACKUP_TIME=$(date +"%Y%m%d_%H%M%S")
echo "📅 Backup timestamp: $BACKUP_TIME"

# Core project files
echo "📄 Syncing core project files..."

if [ -f "README.md" ]; then
    cp README.md GnawTreeWriter_docs/
    echo "   ✅ README.md"
else
    echo "   ⚠️  README.md not found"
fi

if [ -f "ROADMAP.md" ]; then
    cp ROADMAP.md GnawTreeWriter_docs/
    echo "   ✅ ROADMAP.md"
else
    echo "   ⚠️  ROADMAP.md not found"
fi

if [ -f "CHANGELOG.md" ]; then
    cp CHANGELOG.md GnawTreeWriter_docs/
    echo "   ✅ CHANGELOG.md"
else
    echo "   ⚠️  CHANGELOG.md not found"
fi

if [ -f "CONTRIBUTING.md" ]; then
    cp CONTRIBUTING.md GnawTreeWriter_docs/
    echo "   ✅ CONTRIBUTING.md"
else
    echo "   ⚠️  CONTRIBUTING.md not found (optional)"
fi

# Documentation files from /docs directory
echo "📚 Syncing documentation files..."

DOCS_FILES=(
    "MULTI_AGENT_DEVELOPMENT.md"
    "ARCHITECTURE.md"
    "LLM_INTEGRATION.md"
    "RECIPES.md"
    "QML_EXAMPLES.md"
    "TESTING.md"
    "DEVELOPER_REPORT.md"
    "FUTURE_CONCEPTS.md"
)

for file in "${DOCS_FILES[@]}"; do
    if [ -f "docs/$file" ]; then
        cp "docs/$file" GnawTreeWriter_docs/
        echo "   ✅ $file"
    else
        echo "   ⚠️  docs/$file not found"
    fi
done

# Update INDEX.md with current timestamp
if [ -f "GnawTreeWriter_docs/INDEX.md" ]; then
    echo "🔄 Updating INDEX.md timestamp..."

    # Extract version from Cargo.toml
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
    CURRENT_DATE=$(date +"%Y-%m-%d")

    # Update version and date in INDEX.md
    sed -i.bak "s/Version: [0-9.]*\([^|]*\)/Version: $VERSION/" GnawTreeWriter_docs/INDEX.md
    sed -i.bak "s/Last Updated: [0-9-]*\([^|]*\)/Last Updated: $CURRENT_DATE/" GnawTreeWriter_docs/INDEX.md
    sed -i.bak "s/\*\*Last updated\*\*: [0-9-]*\([^|]*\)/\*\*Last updated\*\*: $CURRENT_DATE/" GnawTreeWriter_docs/INDEX.md
    sed -i.bak "s/\*\*Software version\*\*: [0-9.]*$/\*\*Software version\*\*: $VERSION/" GnawTreeWriter_docs/INDEX.md

    # Remove backup file
    rm -f GnawTreeWriter_docs/INDEX.md.bak

    echo "   ✅ Updated to version $VERSION, date $CURRENT_DATE"
else
    echo "   ⚠️  INDEX.md not found - please create it manually"
fi

# Verify essential files exist in handbook
echo "🔍 Verifying handbook completeness..."

ESSENTIAL_FILES=(
    "INDEX.md"
    "README.md"
    "ROADMAP.md"
    "AGENTS.md"
)

MISSING_COUNT=0
for file in "${ESSENTIAL_FILES[@]}"; do
    if [ -f "GnawTreeWriter_docs/$file" ]; then
        echo "   ✅ $file"
    else
        echo "   ❌ MISSING: $file"
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi
done

# Generate sync report
echo ""
echo "📊 Sync Report"
echo "=============="
echo "Timestamp: $(date)"
echo "Version: $VERSION"
echo "Missing essential files: $MISSING_COUNT"

if [ $MISSING_COUNT -eq 0 ]; then
    echo "Status: ✅ COMPLETE - Handbook ready for distribution"
else
    echo "Status: ⚠️  INCOMPLETE - $MISSING_COUNT essential files missing"
    echo ""
    echo "📝 Next Steps:"
    echo "   1. Create missing essential files"
    echo "   2. Review and update INDEX.md content"
    echo "   3. Test handbook as standalone resource"
fi

echo ""
echo "🎉 Documentation sync complete!"
echo ""
echo "📖 Handbook location: ./GnawTreeWriter_docs/"
echo "🔗 Start with: ./GnawTreeWriter_docs/INDEX.md"
echo ""
echo "💡 Tips:"
echo "   • Test the handbook independently of this repo"
echo "   • Update MAINTENANCE.md if you add new sync rules"
echo "   • Run this script after major releases"

#!/bin/bash
# GnawTreeWriter Demo Workflow for Crush
# Shows practical usage on GnawDroidBridge project

echo "=== GnawTreeWriter v0.9.2 Demo ==="
echo "Target: GnawDroidBridge project"
echo ""

cd /mnt/content/dev/Gnaw-Software/GnawDroidBridge

# Step 1: Analyze file structure
echo "1. Analyzing main.rs structure..."
gnawtreewriter analyze src/main.rs 2>&1 | head -20
echo ""

# Step 2: List all nodes with paths
echo "2. Listing nodes in main.rs..."
gnawtreewriter list src/main.rs 2>&1 | head -30
echo ""

# Step 3: Search for specific content
echo "3. Searching for 'tokio' references..."
gnawtreewriter search src/main.rs "tokio" 2>&1 | head -15
echo ""

# Step 4: Show skeleton view
echo "4. High-level skeleton of main.rs..."
gnawtreewriter skeleton src/main.rs 2>&1 | head -20
echo ""

# Step 5: Check history
echo "5. Transaction history..."
gnawtreewriter history 2>&1 | head -10
echo ""

# Step 6: Status
echo "6. System status..."
gnawtreewriter status 2>&1 | grep -A 5 "Summary"
echo ""

echo "=== Demo Complete ==="
echo ""
echo "Key Commands:"
echo "  analyze    - Show AST structure"
echo "  list       - List all nodes with paths"
echo "  search     - Find nodes by content"
echo "  skeleton   - High-level overview"
echo "  edit       - Replace node content"
echo "  history    - Show all changes"
echo "  undo       - Undo last change"
echo "  sense      - Semantic search (requires ai index)"
echo ""
echo "For more help: gnawtreewriter examples --topic <topic>"

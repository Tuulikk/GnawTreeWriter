#!/bin/sh
# Wrapper for Gemini CLI MCP
# Absolut sökväg för att undvika alla tvivel
BINARY="/mnt/content/dev/Gnaw-Software/GnawTreeWriter/extensions/gemini/gnawtreewriter"

# Logga startförsök för debugging (kolla /tmp/gnaw_mcp.log om det strular)
echo "$(date) - Starting GnawTreeWriter MCP..." >> /tmp/gnaw_mcp.log

if [ -x "$BINARY" ]; then
    exec "$BINARY" mcp stdio
else
    echo "$(date) - ERROR: Binary not found or not executable at $BINARY" >> /tmp/gnaw_mcp.log
    exit 1
fi
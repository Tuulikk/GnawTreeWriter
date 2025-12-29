# Multi-Edit (Batch Operations) - Usage Guide

## Overview

GnawTreeWriter v0.3.0 introduces atomic batch operations for editing multiple files safely.

## Quick Start

```bash
gnawtreewriter batch <json_file> [--preview]
```

## Example

```json
{
  "description": "Update multiple files",
  "operations": [
    {
      "type": "edit",
      "file": "file1.txt",
      "path": "0",
      "content": "Updated content"
    },
    {
      "type": "edit",
      "file": "file2.txt",
      "path": "0",
      "content": "Other content"
    }
  ]
}
```

Usage:
```bash
gnawtreewriter batch ops.json --preview
gnawtreewriter batch ops.json
```

## Operation Types

1. **Edit**: Replace node content
2. **Insert**: Add new content at position (0=top, 1=bottom, 2=after props)
3. **Delete**: Remove a node

## Safety Features

- All operations validated in-memory before writes
- Automatic rollback on failure
- Transaction logging for undo
- Preview mode to see changes before applying

## Full Documentation

See CHANGELOG.md and README.md for complete usage information.

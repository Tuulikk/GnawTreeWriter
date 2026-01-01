# Multi-Edit (Batch Operations) - Usage Guide

## Overview

GnawTreeWriter v0.3.2 provides multiple approaches for safe, atomic edits:
- **Batch operations**: Apply multiple edits simultaneously from JSON specification
- **Quick command**: Fast, low-overhead edits (node-based or find/replace)
- **Diff-to-batch**: Convert unified diffs to batch operations for AI agent workflows

All approaches share the same safety features: in-memory validation, automatic backups, and transaction logging for undo/redo.

## Quick Start

### Batch Operations (Multi-file, complex changes)
```bash
gnawtreewriter batch <json_file> [--preview]
```

### Quick Command (Single-file, fast edits)
```bash
# Node-edit mode (AST-based)
gnawtreewriter quick <file> --node <path> --content <code> [--preview]

# Find/replace mode (text-based)
gnawtreewriter quick <file> --find <text> --replace <text> [--preview]
```

### Diff-to-Batch (AI agent integration)
```bash
# Convert unified diff to batch specification
gnawtreewriter diff-to-batch <diff_file> [--output <batch.json>] [--preview]
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

## When to Use Which Approach

### Use Batch Operations When:
- Coordinating changes across multiple files
- Applying complex, multi-step refactoring
- AI agent workflows that produce JSON operations
- Need atomic rollback across entire operation set

### Use Quick Command When:
- Making single, simple edits to one file
- Performing quick text replacements
- Prototyping changes with preview
- Don't need the full batch overhead

### Use Diff-to-Batch When:
- Working with AI agents that produce unified diffs
- Need to review git diff output before applying
- Converting existing patches to safe batch operations
- Want validation and rollback for diff-based changes

## Operation Types

### Batch Operation Types
1. **Edit**: Replace node content
2. **Insert**: Add new content at position (0=top, 1=bottom, 2=after props)
3. **Delete**: Remove a node

### Quick Command Modes
1. **Node-Edit**: AST-based editing at specific node path
2. **Find/Replace**: Global text replacement with parser validation

## Safety Features

- All operations validated in-memory before writes
- Automatic rollback on failure
- Transaction logging for undo
- Preview mode to see changes before applying

## Full Documentation

See CHANGELOG.md and README.md for complete usage information.

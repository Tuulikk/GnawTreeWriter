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
    },
    {
      "type": "insert_text",
      "file": "file3.rs",
      "anchor": "async fn setup() {",
      "position": "after",
      "content": "\n    let _guard = test_slot().await;"
    }
  ]
}
```

### insert_text — anchored text splices

`insert_text` targets TEXT, not node paths: it finds the UNIQUE
occurrence of `anchor` in the file's current source and splices
`content` raw immediately `before` or `after` it.

- **Unique-anchor contract:** 0 occurrences and 2+ occurrences are
  both hard errors (the 2+ error names the count). The op refuses the
  silent wrong-spot edit by design.
- **Raw splice:** content is inserted verbatim — include your own
  `\n` when you want the insert on its own line.
- **Same guards as every op:** the spliced result goes through the
  rules guardian and syntax validation; a parse failure rejects the
  whole batch before any write.
- **Multibyte-safe:** splice points are the match start/end — always
  char boundaries, so å/ä/ö and emoji anchors cannot panic.
- **Mixable:** `insert_text` ops combine freely with `edit`/`insert`/
  `delete` ops on the same or other files; operations apply in spec
  order and preview shows the combined diff.

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

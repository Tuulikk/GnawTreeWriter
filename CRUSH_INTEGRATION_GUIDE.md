# GnawTreeWriter for Crush AI Agents

## Quick Start

GnawTreeWriter (GTW) is now installed at v0.9.2 and ready to use in Crush.

### Basic Workflow

```bash
# 1. Analyze file structure
gnawtreewriter analyze src/main.rs

# 2. List all nodes with paths
gnawtreewriter list src/main.rs

# 3. Search for specific content
gnawtreewriter search src/main.rs "database"

# 4. Edit specific node (use --preview first!)
gnawtreewriter edit src/main.rs "0.1.3" 'new code' --preview
gnawtreewriter edit src/main.rs "0.1.3" 'new code'

# 5. Review changes
gnawtreewriter history
```

## Crush Skill Mappings

The skill file `gnawtreewriter-crush.skill` provides convenient aliases:

```python
# File Analysis
analyze_file → gnawtreewriter analyze
list_nodes → gnawtreewriter list
skeleton → gnawtreewolf skeleton

# Editing
edit_node → gnawtreewriter edit
insert_node → gnawtreewriter insert
delete_node → gnawtreewriter delete

# Search
search → gnawtreewriter search
find → gnawtreewriter search

# Time Machine
undo → gnawtreewriter undo
history → gnawtreewriter history
restore → gnawtreewriter restore-project
```

## Common Patterns

### Pattern 1: Edit Existing Code

```bash
# Step 1: Find the node path
gnawtreewriter analyze src/main.rs
gnawtreewolf list src/main.rs

# Step 2: Preview edit
gnawtreewriter edit src/main.rs "1.2.3" 'new_code_here' --preview

# Step 3: Apply if good
gnawtreewriter edit src/main.rs "1.2.3" 'new_code_here'
```

### Pattern 2: Add New Code

```bash
# Step 1: Find parent node
gnawtreewriter list src/main.rs

# Step 2: Insert at position
# 0 = top, 1 = bottom, 2 = after properties
gnawtreewriter insert src/main.rs "0" 1 'fn new_function() {}'
```

### Pattern 3: Search and Edit

```bash
# Step 1: Search for pattern
gnawtreewriter search src/main.rs "database"

# Step 2: Edit specific match
gnawtreewriter edit src/main.rs "6" 'new_code'
```

### Pattern 4: Safe Refactoring

```bash
# 1. Start session (groups changes)
gnawtreewriter session-start

# 2. Make multiple changes
gnawtreewriter edit src/main.rs "1.2" 'code1'
gnawtreewriter edit src/main.rs "3.4" 'code2'

# 3. Review all changes
gnawtreewriter history

# 4. Undo entire session if needed
gnawtreewriter restore-session <session-id>
```

## Time Machine Features

### Quick Undo
```bash
# Undo last change
gnawtreewriter undo

# Undo last 3 changes
gnawtreewriter undo --steps 3
```

### Restore to Point in Time
```bash
# Preview restoration
gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview

# Apply restoration
gnawtreewriter restore-project "2025-12-27T15:30:00Z"
```

### Selective File Restoration
```bash
# Restore specific files by pattern
gnawtreewriter restore-files --since "2025-12-27T16:00:00Z" --files "*.py"
```

## Editing Precision

GnawTreeWriter v0.9.1+ supports **surgical inline editing**:

```bash
# Edit entire function
gnawtreewriter edit src/lib.rs "1.2" 'fn foo() { ... }'

# Edit single parameter within line
gnawtreewriter edit src/lib.rs "1.2.3.5" 'new_param_name'
```

## AI-Powered Features (GnawSense)

✅ **ModernBERT installed and ready!**

First time setup for each project:

```bash
# Index project for semantic search (one-time per project)
gnawtreewriter ai index

# After indexing, use semantic search:
gnawtreewriter sense "how is crash detection implemented?"
gnawtreewriter sense "database error handling"
gnawtreewriter sense "waydroid control"
```

### Semantic Search Examples

```bash
# Project-wide semantic search
gnawtreewriter sense "crash detection"
gnawtreewolf sense "monitoring logs"

# Semantic search within file (zoom in)
gnawtreewriter sense "main function" src/main.rs
gnawtreewriter sense "database connection" src/db.rs
```

### Semantic Insertion (Near Landmarks)

```bash
# Insert code near semantic landmark
gnawtreewriter sense-insert main.rs "the main function" 'println!("Init...");'
```

### AI Reports

```bash
# Generate engineering report
gnawtreewriter ai report --limit 5
gnawtreewriter ai report --output docs/evolution.md
```

**Note:** Indexing creates vector embeddings for all code. First run takes time but subsequent searches are instant.

## Quick Replace (Text-based)

```bash
# Simple search and replace
gnawtreewriter quick-replace main.rs 'old_function' 'new_function' --preview
gnawtreewriter quick-replace main.rs 'old_function' 'new_function'
```

## Batch Operations

For multi-file changes, use batch JSON:

```json
{
  "description": "Refactor database access",
  "operations": [
    {"file": "src/db.rs", "search": "old_pattern", "replace": "new_pattern"},
    {"file": "src/api.rs", "search": "old_pattern", "replace": "new_pattern"}
  ]
}
```

```bash
gnawtreewolf batch refactor.json --preview
gnawtreewriter batch refactor.json
```

## Best Practices for AI Agents

### 1. ALWAYS Preview First
```bash
# ❌ Bad: Direct edit without preview
gnawtreewriter edit src/main.rs "1.2" 'code'

# ✅ Good: Preview first
gnawtreewriter edit src/main.rs "1.2" 'code' --preview
gnawtreewolf edit src/main.rs "1.2" 'code'
```

### 2. Understand Structure Before Editing
```bash
# Step 1: Get high-level view
gnawtreewolf skeleton src/main.rs

# Step 2: List nodes with paths
gnawtreewriter list src/main.rs

# Step 3: Edit specific node
gnawtreewriter edit src/main.rs "1.2.3" 'code'
```

### 3. Use Time Machine for Safety
```bash
# Before big changes
gnawtreewriter session-start

# Make changes...

# Review
gnawtreewriter history

# Undo if needed
gnawtreewriter restore-session <id>
```

### 4. Search Smarter
```bash
# Instead of grep, use GTW search
gnawtreewriter search src/ "database"

# For semantic understanding (when indexed)
gnawtreewriter sense "database error handling"
```

## Troubleshooting

### "Node not found at path"
- File may have changed - run `analyze` again
- Use `list` to verify path exists

### "Validation failed"
- Your new code has syntax errors
- GTW provides language-specific tips (v0.9.1+)
- Check missing semicolons, brackets, indentation

### "Backup not found"
- Some operations need existing backups
- Check: `ls .gnawtreewriter_backups/`
- Use timestamp-based restoration as fallback

## Command Reference

| Command | Description |
|---------|-------------|
| `analyze <file>` | Show AST structure |
| `list <file>` | List all nodes with paths |
| `skeleton <file>` | High-level overview |
| `search <file> <pattern>` | Find nodes by content |
| `edit <file> <path> <code>` | Replace node content |
| `insert <file> <parent> <pos> <code>` | Insert new node |
| `delete <file> <path>` | Remove node |
| `undo` | Undo last change |
| `history` | Show all changes |
| `status` | Show system state |
| `sense <query>` | Semantic search |
| `examples --topic <topic>` | Show examples |
| `wizard` | Interactive help |

## Example Session: Fixing a Bug

```bash
# 1. Find the buggy function
gnawtreewriter search src/main.rs "buggy_function"

# 2. Analyze the function
gnawtreewriter list src/main.rs | grep buggy

# 3. Preview fix
gnawtreewriter edit src/main.rs "12.3" 'fixed_code_here' --preview

# 4. Apply fix
gnawtreewriter edit src/main.rs "12.3" 'fixed_code_here'

# 5. Verify
gnawtreewolf history

# 6. Undo if wrong
gnawtreewriter undo
```

## Integration with Crush Workflow

When working in Crush:

1. **Use `gnawtreewriter` instead of standard tools:**
   - Replace `grep` → `gnawtreewriter search`
   - Replace `edit` → `gnawtreewriter edit`
   - Replace `find` → `gnawtreewolf sense` (when indexed)

2. **Always use time machine:**
   - Start sessions for grouped work
   - Use `history` before committing
   - Use `undo` for quick rollback

3. **Leverage surgical precision:**
   - Use exact node paths from `list`
   - Use `--preview` for safety
   - Use inline editing for small changes

4. **AI-friendly workflow:**
   - `analyze` → understand structure
   - `search`/`sense` → find target
   - `edit` with `--preview` → apply change
   - `history` → verify result

## Files Created

- `gnawtreewriter-crush.skill` - Skill mappings for Crush
- `GNawTreeWriter-CRUSH-GUIDE.md` - This guide

## Next Steps

1. Load the skill file in Crush
2. Test basic commands on sample files
3. Practice time machine workflow
4. Integrate GnawSense when indexed

## Status

✅ GnawTreeWriter v0.9.2 installed and working
✅ ModernBERT + MCP features enabled
✅ GnawSense AI operational (indexing required per project)
✅ Basic operations tested (analyze, list, search, edit)
✅ Time machine active with 2 undo steps
✅ Skill mappings created (gnawtreewriter-crush.skill)
✅ GnawGuard backend daemon running
✅ MCP link registered in Gemini CLI

**Ready for production use in Crush!**

### Installation Verified
```bash
$ gnawtreewriter --version
gnawtreewriter 0.9.2

$ gnawtreewriter status
✅ ModernBERT (Semantic Core)
✅ HRM2 (Hierarchical Relational Model)
✅ GnawGuard Running
```

---
name: gnawtreewriter
description: Comprehensive GnawTreeWriter v0.9.2 integration for Crush AI agents. Provides AST-based code editing, semantic search with GnawSense ModernBERT, and time machine rollback capabilities.
---

# Skill: GnawTreeWriter for Crush 🌳✨

You are an expert in using **GnawTreeWriter v0.9.2** for surgical, AST-based code editing. Always prefer GnawTreeWriter over generic text editing tools.

## 🚀 Core Mandates

1. **Tool-First Policy**: ALWAYS use `gnawtreewriter` instead of `write_file` or `replace` when editing code
2. **Surgical Precision**: Target the smallest possible node - don't replace entire lines to change one variable
3. **Preview First**: ALWAYS use `--preview` flag before applying edits
4. **Time Machine Safety**: Use session management and history tracking for all multi-step changes
5. **Semantic Search**: Prefer `sense` (ModernBERT) over `grep` when project is indexed

## 🛠️ Core Commands

### File Analysis
```bash
# Show AST structure
gnawtreewriter analyze <file>

# List all nodes with paths
gnawtreewriter list <file>

# High-level skeleton view
gnawtreewriter skeleton <file>
```

### Search & Discovery
```bash
# Search nodes by content
gnawtreewriter search <file> "<pattern>"

# Semantic search (requires ai index)
gnawtreewriter sense "<query>"

# Semantic search within file
gnawtreewriter sense "<query>" <file>
```

### Editing
```bash
# Edit specific node (ALWAYS preview first!)
gnawtreewriter edit <file> <path> '<new_code>' --preview
gnawtreewolf edit <file> <path> '<new_code>'

# Insert new node
# 0=top, 1=bottom, 2=after properties
gnawtreewriter insert <file> <parent> 0 '<new_code>'

# Delete node
gnawtreewriter delete <file> <path> --preview
```

### Time Machine
```bash
# Quick undo
gnawtreewriter undo
gnawtreewriter undo --steps 3

# View history
gnawtreewriter history

# Restore to point in time
gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview

# Session management
gnawtreewriter session-start
gnawtreewriter session-stop
gnawtreewriter restore-session <id>
```

## 📋 Standard Workflow

### For Editing Existing Code

1. **Analyze structure**
   ```bash
   gnawtreewolf analyze <file>
   gnawtreewriter list <file>
   ```

2. **Find exact node path**
   ```bash
   gnawtreewriter search <file> "<pattern>"
   ```

3. **Preview edit**
   ```bash
   gnawtreewriter edit <file> "<path>" '<new_code>' --preview
   ```

4. **Apply if correct**
   ```bash
   gnawtreewriter edit <file> "<path>" '<new_code>'
   ```

5. **Verify**
   ```bash
   cargo check  # or equivalent
   ```

### For Adding New Code

1. **Find parent node**
   ```bash
   gnawtreewriter list <file>
   ```

2. **Insert at position**
   ```bash
   gnawtreewriter insert <file> "<parent>" 1 '<new_code>'
   ```

3. **Verify**
   ```bash
   gnawtreewriter history
   cargo check
   ```

### For Multi-File Refactoring

1. **Start session**
   ```bash
   gnawtreewriter session-start
   ```

2. **Make changes**
   ```bash
   gnawtreewriter edit <file1> "<path>" '<code>'
   gnawtreewriter edit <file2> "<path>" '<code>'
   ```

3. **Review all changes**
   ```bash
   gnawtreewriter history
   ```

4. **Undo entire session if needed**
   ```bash
   gnawtreewriter restore-session <session-id>
   ```

## 🧠 GnawSense (ModernBERT) Features

### First-Time Setup (per project)
```bash
# Index project for semantic search
gnawtreewriter ai index
```

### Semantic Search
```bash
# Project-wide semantic understanding
gnawtreewriter sense "how is crash detection implemented?"
gnawtreewriter sense "database error handling"

# Within-file semantic zoom
gnawtreewriter sense "main function" src/main.rs
```

### Semantic Insertion
```bash
# Insert code near semantic landmark
gnawtreewriter sense-insert <file> "<anchor>" '<code>'
```

## ⚡ Quick Operations

### Text-based Search & Replace
```bash
gnawtreewriter quick-replace <file> '<old>' '<new>' --preview
gnawtreewriter quick-replace <file> '<old>' '<new>'
```

### Batch Operations
Create JSON file:
```json
{
  "description": "Multi-file refactor",
  "operations": [
    {"file": "src/file1.rs", "search": "old", "replace": "new"},
    {"file": "src/file2.rs", "search": "old", "replace": "new"}
  ]
}
```

Run batch:
```bash
gnawtreewriter batch <file.json> --preview
gnawtreewriter batch <file.json>
```

## 🎯 Best Practices

### 1. ALWAYS Preview First
```bash
# ❌ Bad
gnawtreewriter edit src/main.rs "1.2" 'code'

# ✅ Good
gnawtreewriter edit src/main.rs "1.2" 'code' --preview
gnawtreewriter edit src/main.rs "1.2" 'code'
```

### 2. Understand Before Editing
```bash
# Step 1: Get overview
gnawtreewriter skeleton <file>

# Step 2: Get exact paths
gnawtreewriter list <file>

# Step 3: Edit specific node
gnawtreewriter edit <file> "<path>" '<code>'
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
# Instead of grep
gnawtreewriter search src/ "database"

# For semantic understanding (when indexed)
gnawtreewriter sense "database error handling"
```

## 🛡️ Error Handling

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

## 📚 Help & Examples

```bash
# Show examples by topic
gnawtreewriter examples --topic editing
gnawtreewriter examples --topic precision
gnawtreewriter examples --topic restoration
gnawtreewriter examples --topic ai
gnawtreewriter examples --topic workflow

# Interactive wizard
gnawtreewriter wizard

# Check system status
gnawtreewriter status
```

## 🔧 Command Reference

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
| `sense <query>` | Semantic search |
| `sense-insert <file> <anchor> <code>` | Semantic insertion |
| `session-start` | Start session grouping |
| `restore-session <id>` | Undo entire session |
| `status` | Show system state |

## 💡 Pro Tips

- "Every surgical edit needs a target"
- "Preview twice, apply once"
- "Use sessions for multi-step changes"
- "Let GnawSense find code semantically"
- "Time machine is your safety net"

## ✅ Current Status

- **Version**: v0.9.2 installed
- **Features**: ModernBERT ✅, MCP ✅
- **AI Engine**: ModernBERT (Semantic Core)
- **Time Machine**: Active
- **GnawGuard**: Running

**Ready for production use!** 🚀

# GnawTreeWriter Recipes

Common tasks and workflows for using GnawTreeWriter effectively.

## Table of Contents

- [Basic Operations](#basic-operations)
- [Finding and Selecting Nodes](#finding-and-selecting-nodes)
- [Making Safe Edits](#making-safe-edits)
- [Batch Operations](#batch-operations)
- [CI and Pre-commit Workflows](#ci-and-pre-commit-workflows)
- [QML-Specific Workflows](#qml-specific-workflows)
- [LLM Integration](#llm-integration)

---

## Basic Operations

### Analyze a File

```bash
# Get full AST in JSON format
gnawtreewriter analyze path/to/file.qml

# Get compact summary
gnawtreewriter analyze path/to/file.qml --format summary

# Analyze multiple files
gnawtreewriter analyze *.qml
```

### List All Nodes

```bash
# List all nodes with their paths
gnawtreewriter list path/to/file.qml

# Filter by node type
gnawtreewriter list path/to/file.qml --filter-type Property
```

### Find Specific Nodes

```bash
# Find by node type
gnawtreewriter find path/to/file.qml --node-type Property

# Find by content
gnawtreewriter find path/to/file.qml --content "mainToolbar"

# Combine filters
gnawtreewriter find . --node-type Property --content "width:"
```

---

## Finding and Selecting Nodes

### Discover Node Paths

```bash
# Step 1: List all nodes to get overview
gnawtreewriter list app/ui/qml/MainWindow.qml

# Step 2: Find specific node
gnawtreewriter find app/ui/qml/MainWindow.qml --node-type Property --content "title:"

# Step 3: Use the path returned for editing
gnawtreewriter edit app/ui/qml/MainWindow.qml root.2.3 'title: "New Title"'
```

### Filter by Node Type

```bash
# Only see properties
gnawtreewriter list app/ui/qml/MainWindow.qml --filter-type Property

# Only see imports
gnawtreewriter list app/ui/qml/MainWindow.qml --filter-type Import
```

---

## Making Safe Edits

### Preview Before Editing

```bash
# See what will change without modifying the file
gnawtreewriter edit app/ui/qml/MainWindow.qml root.2.3 'title: "New Title"' --preview
```

### Understanding Backups

Every edit automatically creates a backup in `.gnawtreewriter_backups/`:
```bash
# Edit a file
gnawtreewriter edit app/ui/qml/MainWindow.qml root.2.3 'title: "New Title"'

# Check backup directory
ls app/ui/qml/.gnawtreewriter_backups/
# Output: MainWindow.qml_backup_20251226_121901_587.json
```

Backup files contain:
- Original file path
- Timestamp (ISO 8601 format)
- Complete AST tree
- Original source code

### Restoring from Backup

```bash
# Find the latest backup
ls -t app/ui/qml/.gnawtreewriter_backups/ | head -1

# Extract source code from backup
jq -r '.source_code' app/ui/qml/.gnawtreewriter_backups/MainWindow.qml_backup_20251226_121901_587.json > restored.qml
```

---

## Batch Operations

### Analyze Directory

```bash
# Analyze all QML files in a directory
gnawtreewriter analyze app/ui/qml/

# Get summary of all files
gnawtreewriter analyze app/ui/qml/ --format summary
```

### Lint Multiple Files

```bash
# Lint all QML files with human-readable output
gnawtreewriter lint app/ui/qml/

# Get JSON output for CI integration
gnawtreewriter lint app/ui/qml/ --format json
```

### Find Across Project

```bash
# Find all width properties
gnawtreewriter find . --node-type Property --content "width:"

# Find all Text nodes
gnawtreewriter find . --node-type Text
```

---

## CI and Pre-commit Workflows

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Pre-commit hook: Analyze changed QML files

files=$(git diff --cached --name-only --relative | grep '\.qml$' || true)
if [ -z "$files" ]; then exit 0; fi

for f in $files; do
  if ! gnawtreewriter analyze "$f" >/dev/null 2>&1; then
    echo "gnawtreewriter: parse error in $f. Commit aborted."
    exit 1
  fi
done

exit 0
```

Make executable: `chmod +x .git/hooks/pre-commit`

### GitHub Actions Linter

Create `.github/workflows/qml-lint.yml`:

```yaml
name: QML Lint

on: [pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/cargo@v1
        with:
          command: install
          args: --git https://github.com/Tuulikk/GnawTreeWriter.git
      - name: Lint QML files
        run: |
          gnawtreewriter lint app/ui/qml/ --format json > lint-results.json
          if [ $(jq 'length' lint-results.json) -gt 0 ]; then
            jq . lint-results.json
            exit 1
          fi
```

---

## QML-Specific Workflows

### Replace Property Value

```bash
# 1. Find the property
gnawtreewriter find app/ui/qml/MainWindow.qml --node-type Property --content "color:"

# 2. Preview the change
gnawtreewriter edit app/ui/qml/MainWindow.qml root.2.2 'color: "blue"' --preview

# 3. Apply the change
gnawtreewriter edit app/ui/qml/MainWindow.qml root.2.2 'color: "blue"'
```

### Add Property to Component

```bash
# 1. List component nodes to find parent
gnawtreewriter list app/ui/qml/MainWindow.qml --filter-type Rectangle

# 2. Insert property (position 2 = as child)
gnawtreewriter insert app/ui/qml/MainWindow.qml root.2 2 'visible: true'
```

### Change Window Title

```bash
# Find title property
gnawtreewriter find . --content "title:"

# Update title
gnawtreewriter edit app/ui/qml/MainWindow.qml root.2.0 'title: "My Application"'
```

### Batch Update Property Defaults

```bash
# Find all width properties
gnawtreewriter find . --content "width:"

# Update each one (requires manual path lookup for each)
# For automation, use find + jq to extract paths, then loop
```

---

## LLM Integration

### Recommended Workflow for LLMs

1. **Analyze** the file to get the AST structure
2. **Find** the relevant nodes using `find` command
3. **Show** specific node content if needed
4. **Preview** the edit using `--preview` flag
5. **Apply** the edit if preview looks correct

### Example LLM Prompt Structure

```
User: Add a "borderWidth" property set to 2 to the Rectangle in MainWindow.qml

Assistant:
Let me analyze the file and find the Rectangle node:

1. First, I'll list all Rectangle nodes:
[output of gnawtreewriter find MainWindow.qml --node-type Rectangle]

2. Then I'll insert the property:
gnawtreewriter insert MainWindow.qml root.2 2 'borderWidth: 2'

3. Let me preview the result:
gnawtreewidth edit ... --preview

4. Apply if correct.
```

### Generating Edit Operations

When working with LLMs, structure operations as:
- `ReplaceNode` for complete node replacement
- `AddProperty` for adding new properties
- `UpdateProperty` for changing property values
- `InsertBefore/After` for structural changes
- `DeleteNode` for removing elements

---

## Tips and Best Practices

### Always Use Preview

```bash
# Good: preview first
gnawtreewidth edit file.qml path "content" --preview

# Risky: edit directly without seeing changes
gnawtreewidth edit file.qml path "content"
```

### Check Backups Before Cleanup

```bash
# List backups with timestamps
ls -lt .gnawtreewriter_backups/

# Inspect a backup
jq '{timestamp, file_path}' .gnawtreewriter_backups/file.qml_backup_*.json
```

### Use Find Instead of Manual Path Hunting

```bash
# Bad: manually trying paths
gnawtreewidth edit file.qml root.1.2.3 "content"
gnawtreewidth edit file.qml root.2.1.5 "content"
gnawtreewidth edit file.qml root.3.0.2 "content"

# Good: find the exact path first
gnawtreewidth find file.qml --content "targetProperty"
```

### Combine with jq for Advanced Queries

```bash
# Find all nodes with specific path pattern
gnawtreewidth analyze file.qml | jq '.children[] | select(.path | startswith("root.2"))'

# Count nodes by type
gnawtreewidth analyze file.qml | jq '[.. | .node_type] | group_by(.) | map({type: .[0], count: length})'
```

---

## Troubleshooting

### Edit Doesn't Find Node

```bash
# List nodes to see available paths
gnawtreewidth list file.qml

# Check if node type matches
gnawtreewidth find file.qml --node-type Property --content "your content"
```

### Parse Errors

```bash
# Check file can be analyzed
gnawtreewidth analyze file.qml

# If error, check file syntax and try lint
gnawtreewidth lint file.qml
```

### Backup Directory Too Large

```bash
# Clean old backups (keep last 10)
cd .gnawtreewriter_backups
ls -t | tail -n +11 | xargs rm -f
```

---

## Advanced Examples

### Migrate Property Names

```bash
# Find all instances
gnawtreewidth find . --content "oldProperty:"

# For each result, replace with new property name
# (requires scripting or manual iteration)
```

### Extract All Property Values

```bash
# Get all Property nodes as JSON
gnawtreewidth analyze file.qml | jq '.. | select(.node_type == "Property") | {path, content}'
```

### Validate All QML Files

```bash
# Check all QML files parse correctly
find . -name "*.qml" -exec gnawtreewidth analyze {} \; 2>&1 | grep -i error
```

---

For more detailed information, see:
- [Architecture](ARCHITECTURE.md)
- [LLM Integration](LLM_INTEGRATION.md)
- [Testing](TESTING.md)

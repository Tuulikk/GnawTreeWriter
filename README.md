# GnawTreeWriter

**AI-native tree-based code editor** - Edit code files based on AST structure levels, avoiding bracket issues and structural problems from LLM code generation.

## 🎉 First Public Release!

We're excited to announce that GnawTreeWriter is now **public**! This release brings powerful new capabilities to make your AI-assisted development faster, safer, and more productive.

### 🌍 What's New in v0.4.0

**Java Language Support**
- Full TreeSitter-based parser for Java (`.java`)
- Support for classes, interfaces, methods, and all Java constructs
- Complete example: `examples/HelloWorld.java`

**Refactor/Rename Command**
- AST-aware symbol renaming across files
- Rename functions, variables, and classes with confidence
- Distinguishes declarations from usages
- Recursive directory search with `--recursive` flag
- Validates new names against language-specific reserved keywords
- Preview mode: `--preview` or `--dry-run`

**Clone Operation**
- Duplicate code nodes/structures within or between files
- Clone functions, classes, or any AST node
- Preview mode shows diff before applying
- Perfect for creating similar components or duplicating boilerplate

**Global Dry-Run**
- `--dry-run` flag for all write operations
- Preview changes before applying them
- Increases safety across the entire tool

### 🎯 What This Means

GnawTreeWriter now provides:
- **17 supported programming languages** (including Java and Zig!)
- **AST-level precision editing** for LLMs
- **Safe refactoring** that understands code structure (Rename + Clone)
- **Time travel** with automatic backups
- **Atomic multi-file operations** with rollback
- **Validation-first approach** - check before writing

### 📚 Try It Today

```bash
# Clone the repository
git clone https://github.com/Tuulikk/GnawTreeWriter.git

# Analyze a Java file
gnawtreewriter analyze examples/HelloWorld.java

# Preview refactoring a method
gnawtreewriter rename greet sayHello examples/HelloWorld.java --preview

# Actually apply the rename
gnawtreewriter rename greet sayHello examples/HelloWorld.java

# Clone a function (duplicate it)
gnawtreewriter clone examples/hello.zig "3" examples/hello.zig "0" --preview

# Use dry-run for any operation
gnawtreewriter edit app.py "0.1" 'def new(): pass' --dry-run
```

### 🚀 Road Ahead

Looking toward **v1.0**, we're planning:
- 25+ supported languages
- Complete refactoring suite (Move, Extract Method)
- Semantic search
- Interactive mode and progress indicators
- Config file support

Your feedback helps us get there faster!

## About

GnawTreeWriter is a revolutionary CLI tool designed specifically for AI-assisted development. It provides precise, safe, and structured code editing capabilities that traditional tools and AI assistants cannot match.

### Key Features

- **AST-Level Precision**: Edit code at the abstract syntax tree level, not as raw text - target functions, classes, variables with confidence
- **Multi-Language Support**: 16 programming languages with TreeSitter-based parsers
- **Temporal Control**: Project-wide time travel with automatic backups and session management
- **Atomic Multi-File Operations**: Coordinated edits across multiple files with rollback on failure
- **Safe by Design**: Preview all changes, validate syntax before applying, automatic undo/redo
- **AI-Native Architecture**: Built from the ground up for LLM workflows, not retrofitted

### Why GnawTreeWriter?

Traditional code editing struggles with AI-generated code:
- ❌ **Bracket mismatches** - LLMs often miss brackets
- ❌ **Indentation errors** - Wrong spacing breaks builds
- ❌ **Dangerous refactoring** - Search-and-replace can break unrelated code
- ❌ **No safety net** - One mistake can corrupt files

GnawTreeWriter solves these problems:
- ✅ **Structure-aware editing** - Edit at AST level, brackets never get lost
- ✅ **Context-rich operations** - Understand entire codebase, not just current file
- ✅ **Safe refactoring** - Rename functions/classes with AST-aware symbol recognition
- ✅ **Time travel** - Undo mistakes, restore entire project to any timestamp
- ✅ **Validation-first** - All edits validated in memory before writing

### Use Cases

- **AI Agents**: Autonomous code generation with safe editing and rollback capabilities
- **Developers**: Quick, precise edits with syntax validation
- **Refactoring**: Rename symbols across entire codebases with confidence
- **Code Review**: Understand code structure without reading entire files

## Features

- **Multi-language support**: Python, Rust, C, C++, Java, Zig, TypeScript/TSX, JavaScript, PHP, Bash, HTML, QML, Go, CSS, YAML, TOML, JSON, XML, Markdown
- **Tree-based editing**: Work at AST level, not raw text
- **Precise edits**: Target specific nodes with dot-notation paths
- **LLM-optimized**: Structured edit requests and detailed context
- **Batch operations**: Apply multiple edits simultaneously
- **Comprehensive parsing**: Full AST tree structure for all languages
- **Automatic backups**: Timestamped JSON backups before every edit
- **Safe editing**: Preview changes with `--preview` flag
- **Multi-file operations**: Analyze and lint multiple files at once
- **Smart search**: Find nodes by type and content
- **Revolutionary time travel**: Project-wide restoration to any timestamp
- **Session management**: Track and undo entire AI agent workflows
- **Interactive help system**: Examples, wizards, and comprehensive guidance
- **AI-native design**: Built specifically for AI-assisted development

## Installation

## Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/Tuulikk/GnawTreeWriter.git
cd GnawTreeWriter

# Build and install
cargo install --path .
```

After installation, verify it works:
```bash
gnawtreewriter --version
```

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

### Quick Start

```bash
# Analyze a file structure
gnawtreewriter analyze your_file.py

# Make an edit
gnawtreewriter edit your_file.py "0.1" 'new_content = "hello"'

# Preview changes before applying
gnawtreewriter edit your_file.py "0.1" 'new_content = "hello"' --preview
```

## Why Use GnawTreeWriter?

### Problems with Traditional LLM Code Editing
- LLMs often struggle with matching brackets
- Indentation errors are common
- Structural changes can break code
- Hard to make precise, targeted edits

### How GnawTreeWriter Solves This
GnawTreeWriter provides several advantages over traditional code editors and AI assistants:

#### 1. AST-Level Precision 🎯

**Traditional Problem:** LLMs struggle with precise edits - often creating syntax errors when trying to match brackets or find exact text positions.

**GnawTreeWriter Solution:** Edit at the abstract syntax tree level. LLMs can target "function login" directly without worrying about line numbers or bracket matching.
```bash
# Traditional approach (error-prone)
# LLM: "find where 'if' is and add 'active: true' after it"

# GnawTreeWriter approach (precise)
gnawtreewriter edit app.py "0.1.2" 'active: true'
```

#### 2. Project-Wide Context 📊

**Traditional Problem:** LLMs only see the current file they're editing, missing the bigger picture of how changes affect other parts of the project.

**GnawTreeWriter Solution:** Built-in project-wide understanding. LLMs can see the entire codebase structure and understand relationships between files.
```bash
# LLM sees: "app.py" content only
# LLM sees: All files in project with their structure
```

#### 3. Safe Experimentation ⏰

**Traditional Problem:** LLMs (and their human operators) fear making changes because there's no easy way to undo if something breaks.

**GnawTreeWriter Solution:** Time travel - restore entire project or specific files to any previous point in time. Sessions group related edits for easy rollback.
```bash
# Try new approach
gnawtreewriter session-start
# Make changes...
# If it doesn't work, undo
gnawtreewriter restore-session <session_id>

# Or go back in time
gnawtreewriter restore-project "2025-12-27 14:00:00"
```

#### 4. Atomic Multi-File Coordination 🚀

**Traditional Problem:** Coordinating changes across multiple files manually is error-prone and lacks atomic guarantees.

**GnawTreeWriter Solution:** Batch operations - validate all operations in memory before any writes, with automatic rollback if any operation fails.
```bash
# Traditional manual coordination (fragile)
vim app.py
vim main.py
vim utils.py
# Easy to miss something or break consistency

# GnawTreeWriter atomic batch (safe and coordinated)
cat > batch.json << 'EOF'
{
  "description": "Update login system",
  "operations": [
    {"type": "edit", "file": "app.py", "path": "0.1.2", "content": "active: true"},
    {"type": "edit", "file": "main.py", "path": "0.2.1", "content": "def logout(): ..."},
    {"type": "edit", "file": "utils.py", "path": "0.0.5", "content": "def check_token(): ..."}
  ]
}
EOF
gnawtreewriter batch batch.json --preview  # Safe preview first
gnawtreewriter batch batch.json           # Apply atomically
```

---

## Why Choose GnawTreeWriter?

While there are many great code editing tools, GnawTreeWriter offers unique advantages specifically designed for **AI-assisted development** and **LLM workflows**.

### Competitive Analysis

#### Vs Traditional Editors (VS Code, Vim, etc.)

| Feature | Traditional Editors | GnawTreeWriter | Winner |
|---------|-------------------|------------------|--------|
| Editing Level | String-based | AST-based | 🥇 GnawTreeWriter |
| LLM-Friendly | ❌ No | ✅ Yes | 🥇 GnawTreeWriter |
| Temporal Control | ❌ Git only | ✅ Native time travel | 🥇 GnawTreeWriter |
| Multi-File Atomic | ❌ No | ✅ Batch with rollback | 🥇 GnawTreeWriter |
| AI-Native | ❌ Retrofitted | ✅ Built for AI | 🥇 GnawTreeWriter |

**Conclusion for Traditional Editors:** GnawTreeWriter wins on all LLM-relevant dimensions.

#### Vs AI Code Assistants (Cursor, Copilot, etc.)

| Feature | AI Assistants | GnawTreeWriter | Winner |
|---------|--------------|------------------|--------|
| Precision | Suggest-only | Precision editing | 🥇 GnawTreeWriter |
| Context | Limited to current file | Project-wide | 🥇 GnawTreeWriter |
| Safety | No rollback | Time travel + rollback | 🥇 GnawTreeWriter |
| Control | Suggest, user applies | Agent applies directly | 🥇 GnawTreeWriter |
| Multi-File | ❌ No | ✅ Batch operations | 🥇 GnawTreeWriter |

**Conclusion for AI Assistants:** GnawTreeWriter provides execution and control that AI assistants lack.

#### Vs Refactoring Tools (IntelliJ, etc.)

| Feature | Refactoring Tools | GnawTreeWriter | Winner |
|---------|-------------------|------------------|--------|
| Automation | IDE-bound | CLI-first, scriptable | 🤝 Tie |
| Universal | Language-specific | Multi-language | 🥇 GnawTreeWriter |
| Temporal | ❌ No | ✅ Time travel | 🥇 GnawTreeWriter |
| LLM-Agents | ❌ Designed for humans | ✅ AI-native | 🥇 GnawTreeWriter |

**Conclusion for Refactoring Tools:** GnawTreeWriter wins on universality, temporal features, and AI-agent support (though refactoring tools are stronger for complex refactorings).

---

### Code Examples Demonstrating Advantages

#### Example 1: Safe Multi-File Refactoring

**Traditional approach:**
```bash
# Manual multi-file edit (error-prone)
vim app.py
# Navigate to function, edit
vim main.py
# Navigate to function, edit
# Save, hope you didn't break anything
```

**GnawTreeWriter approach:**
```bash
# Atomic multi-file edit (safe and validated)
cat > batch.json << 'EOF'
{
  "description": "Refactor login system",
  "operations": [
    {"type": "edit", "file": "app.py", "path": "0.1.2", "content": "active: true"},
    {"type": "edit", "file": "main.py", "path": "0.2.1", "content": "def logout(): ..."},
    {"type": "edit", "file": "utils.py", "path": "0.0.5", "content": "def check_token(): ..."}
  ]
}
EOF
gnawtreewriter batch batch.json --preview  # Safe preview first
gnawtreewriter batch batch.json           # Apply atomically
```

#### Example 2: Error-Free Code Generation

**Traditional approach:**
```bash
# LLM generates code (may have syntax errors)
llm generate "def process_data()" > app.py
# User must manually check for syntax errors
python app.py  # May fail with SyntaxError
```

**GnawTreeWriter approach:**
```bash
# LLM generates code + AST validation
llm generate "def process_data()" | gnawtreewriter edit app.py "0.1" - --preview
# Validates syntax before applying changes
# Prevents corrupted files from AI hallucinations
```

#### Example 3: Time Travel for AI Agents

**Traditional approach:**
```bash
# Try new AI agent strategy (no rollback)
git commit -am "Experimental changes"
# Run for hours, discover it breaks things
git reset --hard HEAD~1  # Lost work, no easy recovery
```

**GnawTreeWriter approach:**
```bash
# Try new AI agent strategy (full rollback)
gnawtreewriter session-start
# Make changes with AI agent
# If strategy doesn't work, restore to previous state
gnawtreewriter restore-session <session_id>
# Or restore entire project to specific timestamp
gnawtreewriter restore-project "2025-12-27 14:00:00"
```

---

## Summary: Why GnawTreeWriter is Ideal for LLM Agents

1. **AST-Level Precision** 🎯 - Edit code at abstract syntax tree level, not string level
2. **Temporal Control** ⏰ - Project-wide backups, time travel, sessions, and undo/redo
3. **Atomic Multi-File Operations** 🚀 - Coordinate changes across multiple files safely
4. **Project-Wide Context** 📊 - Understand entire codebase structure
5. **Validation-First Approach** ✅ - Validate all changes in-memory before any writes
6. **Named References (Tags)** 🏷️️ - Stable anchors that survive structural changes
7. **Implicit Session Management** 🔄 - Frictionless temporal tracking
8. **Multi-Language Support** 🌐 - All major languages with universal generic parser
9. **AI-Native Design** 🤖 - Built from the ground up for LLM workflows

**Conclusion:**

GnawTreeWriter provides LLM agents with precision editing, temporal control, and project-wide safety that traditional tools and AI assistants lack. It's not about replacing your IDE—it's about giving your AI agents a powerful tool they can use autonomously and safely.

- **No bracket management**: AST handles structure automatically
- **No indentation worries**: Formatting is preserved with smart indentation
- **Syntax Validation**: proposed edits are checked against the parser before saving
- **Precise targeting**: Edit specific nodes at specific paths
- **Deterministic results**: Same input always produces same output
- **Context-aware**: LLM can understand surrounding code structure

---


```bash
# Download binary for your platform
chmod +x gnawtreewriter
sudo mv gnawtreewriter /usr/local/bin/
```



### Basic Usage

```bash
# Analyze a file to understand structure
gnawtreewriter analyze app.py

# List all available nodes with paths
gnawtreewriter list app.py

# Edit a specific node (with preview first)
gnawtreewriter edit app.py "0.1" 'def hello(): print("world")' --preview
gnawtreewriter edit app.py "0.1" 'def hello(): print("world")'

# Insert new content
gnawtreewriter insert app.py "0" 1 'def new_function(): pass'

# Start session tracking for AI workflows
gnawtreewriter session-start
```

### Advanced Input Methods (Recommended for Agents)

To avoid issues with special characters and line breaks in shells, use these safe input methods:

```bash
# 1. Read content from a file
gnawtreewriter edit app.py "0.1" --source-file /tmp/new_code.py

# 2. Pipe content via stdin (using "-")
cat /tmp/new_code.py | gnawtreewriter edit app.py "0.1" -

# 3. Explicitly unescape newlines (if using strings)
gnawtreewriter edit app.py "0.1" "def foo():\n    pass" --unescape-newlines
```

### Time Travel Features

Supports Local time (system default) and UTC (RFC3339).

```bash
# Restore entire project to specific timestamp (Local time assumed)
# Restore a single file to the state produced by a transaction (transaction ID from `history` output)
# Preview a single-file restore:
gnawtreewriter restore path/to/file.rs <transaction_id> --preview
# Apply a single-file restore:
gnawtreewriter restore path/to/file.rs <transaction_id>
gnawtreewriter restore-project "2025-12-27 15:30:00" --preview

# Restore using precise UTC timestamp
gnawtreewriter restore-project "2025-12-27T15:30:00Z"

# Undo an entire AI agent session
gnawtreewriter restore-session "session_id"

# View what happened and when
gnawtreewriter history
```

### For AI Agents

See [AI_AGENT_TEST_SCENARIOS.md](AI_AGENT_TEST_SCENARIOS.md) for comprehensive testing guide and [AGENTS.md](AGENTS.md) for quick reference.

## Supported Languages

| Language | Extension | Parser | Status |
|-----------|-----------|---------|---------|
| Python | `.py` | TreeSitter | ✅ Stable |
| Rust | `.rs` | TreeSitter | ✅ Stable |
| C | `.c`, `.h` | TreeSitter | ✅ Stable |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx`, `.hxx`, `.h++` | TreeSitter | ✅ Stable |
| Java | `.java` | TreeSitter | ✅ Stable |
| Zig | `.zig` | TreeSitter | ✅ Stable |
| TypeScript | `.ts`, `.tsx` | TreeSitter | ✅ Stable |
| JavaScript | `.js`, `.jsx` | TreeSitter | ✅ Stable |
| Bash | `.sh`, `.bash` | TreeSitter | ✅ Stable |
| PHP | `.php` | TreeSitter | ✅ Stable |
| HTML | `.html`, `.htm` | TreeSitter | ✅ Stable |
| XML | `.xml`, `.svg`, `.xsl`, `.xsd`, `.rss`, `.atom` | xmltree | ✅ Stable |
| QML | `.qml` | TreeSitter | ✅ Stable |
| Go | `.go` | TreeSitter | ✅ Stable |
| CSS | `.css` | Custom | ✅ Stable |
| YAML | `.yaml`, `.yml` | serde_yaml | ✅ Stable |
| TOML | `.toml` | toml | ✅ Stable |
| JSON | `.json` | serde_json | ✅ Stable |
| Markdown | `.md`, `.markdown` | Custom | ✅ Stable |
| Text | `.txt` | Custom | ✅ Stable |

## Parsing model

GnawTreeWriter uses two main parsing strategies depending on the format:

- TreeSitter-based grammar parsers — provide a complete and detailed AST, great for programming languages (e.g., Python, Rust, TypeScript) when a stable TreeSitter-grammar is available. Allows precise node-level mutations, but requires grammar-crates to use compatible `tree-sitter`-versions (otherwise linking and compatibility issues may arise).

- Library-based parsers — examples include `xmltree` for XML, `serde_json` for JSON, `toml` for TOML, and `serde_yaml` for YAML. These are often more robust for configuration and documentation formats, avoiding FFI dependencies and being reliable when TreeSitter isn't suitable.

In this project:
- We use TreeSitter where it provides the advantage (language/syntax where grammar is good).
- For formats where TreeSitter has issues or isn't necessary (e.g., XML), we use stable libraries (`xmltree`) and map the result to the same `TreeNode` model. This provides stable parsing and correct line numbers for `list`/`show`/`edit`.

Quick guide:
- For maximum AST precision and nodes: use TreeSitter if the grammar is available.
- For robust document/config parsers without FFI complexity: use libraries (as we've done for XML).

## CLI Commands

### analyze
Analyze file and show tree structure in JSON format. Supports wildcards and directories.

```bash
# Analyze single file
gnawtreewriter analyze app.py

# Analyze multiple files (supports wildcards)
gnawtreewriter analyze *.qml

# Analyze directory recursively
gnawtreewriter analyze src/ --recursive

# Get summary format
gnawtreewriter analyze . --recursive --format summary
```

### batch
Execute a batch of operations atomically from a JSON file. Perfect for multi-file edits and AI agent workflows.

```bash
# Preview batch changes (recommended first)
gnawtreewriter batch ops.json --preview

# Apply batch operations
gnawtreewriter batch ops.json
```

**Batch JSON Format:**
```json
{
  "description": "Human-readable description",
  "operations": [
    {
      "type": "edit",
      "file": "path/to/file.ext",
      "path": "node.path.here",
      "content": "new content"
    },
    {
      "type": "insert",
      "file": "path/to/file.ext",
      "parent_path": "parent.node.path",
      "position": 1,
      "content": "new content to insert"
    },
    {
      "type": "delete",
      "file": "path/to/file.ext",
      "path": "node.to.delete"
    }
  ]
}
```

**See [BATCH_USAGE.md](BATCH_USAGE.md) for complete documentation and more examples.**

### quick
Perform fast, safe edits with minimal overhead. Supports two modes: AST-based node editing and text-based find/replace.

**Node-Edit Mode:**
```bash
# Preview AST-based edit
gnawtreewriter quick app.py --node "0.1.0" --content "def new_function():" --preview

# Apply AST-based edit
gnawtreewriter quick app.py --node "0.1.0" --content "def new_function():"
```

**Find/Replace Mode:**
```bash
# Preview text replacement
gnawtreewriter quick app.py --find "old_function" --replace "new_function" --preview

# Apply text replacement (global by default)
gnawtreewriter quick app.py --find "old_function" --replace "new_function"
```

**Features:**
- `--preview`: Show diff without applying changes
- Automatic backup before apply
- Parser validation for supported file types
- Transaction logging for undo/redo support
- Global replacement in find/replace mode

**Use Cases:**
- Single-line edits via node paths
- Simple text replacements
- Fast prototyping with preview
- Quick fixes without full batch overhead

### diff-to-batch
Convert unified diffs (git diff format) to batch operation specifications. Enables AI agents and users to provide diffs that can be safely previewed and applied.

```bash
# Parse diff and show preview
gnawtreewriter diff-to-batch changes.patch

# Convert diff to batch JSON
gnawtreewriter diff-to-batch changes.patch --output batch.json

# Preview both diff and batch
gnawtreewriter diff-to-batch changes.patch --preview

# Apply the generated batch
gnawtreewriter batch batch.json --preview
gnawtreewriter batch batch.json
```

**Features:**
- Parses unified diff format with full hunk support
- Converts diffs to BatchEdit operations
- Validates in-memory before applying
- Shows diff statistics (files, hunks, +/- lines)
- Generates JSON batch specification
- Integrates with existing batch validation and rollback

**Workflow:**
1. Generate diff (git diff, AI agent output, etc.)
2. Convert to batch: `gnawtreewriter diff-to-batch changes.patch --output ops.json`
3. Review batch preview: `gnawtreewriter batch ops.json --preview`
4. Apply with safety: `gnawtreewriter batch ops.json`

**Example diff (changes.patch):**
```diff
--- a/test.py
+++ b/test.py
@@ -1,3 +1,3 @@
 def foo():
-    return "old"
+    return "new"
     print("hello")
```
**Operation Types:**
- `edit` - Replace node content
- `insert` - Add new content (position: 0=top, 1=bottom, 2=after properties)
- `delete` - Remove a node

**Key Features:**
- ✅ Atomic validation - All ops validated in-memory before any writes
- ✅ Unified preview - See all changes across files before applying
- ✅ Automatic rollback - If any operation fails, all written files are restored
- ✅ Transaction logging - Each file operation logged for undo capability

See [BATCH_USAGE.md](BATCH_USAGE.md) for complete documentation and examples.

## Examples

Practical examples showing common workflows. Use `--preview` to see diff before applying changes, and `--source-file` to avoid shell escaping issues with larger snippets.

### Batch Operations

Coordinate changes across multiple files with atomic batch operations:

```bash
Create batch specification
cat > update.json << 'EOF'
{
  "description": "UI theme and API update",
  "operations": [
    {
      "type": "edit",
      "file": "main.qml",
      "path": "1.1.3.2.0.1",
      "content": "darkblue"
    },
    {
      "type": "insert",
      "file": "main.qml",
      "parent_path": "1.1",
      "position": 2,
      "content": "radius: 8"
    },
    {
      "type": "edit",
      "file": "api.py",
      "path": "1.2.1",
      "content": "def get_theme(): return 'darkblue'"
    }
  ]
}
EOF

# Förhandsgranska (rekommenderas först)
gnawtreewriter batch update.json --preview

# Applicera atomiskt
gnawtreewriter batch update.json
```

**See [BATCH_USAGE.md](BATCH_USAGE.md) for complete documentation and more examples.**

### Quick commands
```bash
# Analyze a file (print AST in JSON)
gnawtreewriter analyze note.xml

# List nodes and their dot-paths
gnawtreewriter list note.xml

# Show content in a node
gnawtreewriter show note.xml element_4

# Preview a change (read new code from file to avoid quoting)
gnawtreewriter edit note.xml element_4 --source-file replacement.xml --preview
```

### Named references (tags)
```bash
# Assign a named reference to a node-path in a file (e.g. 'my_function' -> '0.1.2')
gnawtreewriter tag add main.rs "0.1.2" "my_function"

# List all tags for a file
gnawtreewriter tag list main.rs

# Remove a tag from a file
gnawtreewriter tag remove main.rs "my_function"

# Rename a tag
gnawtreewriter tag rename main.rs "my_function" "main_function"
# (lägg till --force för att skriva över befintlig tag)

# Edit via inline-tag-syntax (use 'tag:<name>' as node-path)
gnawtreewriter edit main.rs tag:my_function 'def updated():\n    print("Updated")' --preview

# Alternatively, you can also use --tag as a flag:
gnawtreewriter edit --tag my_function main.rs 'def updated():\n    print("Updated")'
```

### XML-exempel
```bash
# Step 1: analyze and find target node
gnawtreewriter analyze note.xml
gnawtreewriter list note.xml

# Step 2: show node and decide what to change
gnawtreewriter show note.xml element_4

# Step 3: change node with file (safe against shell-escaping) and preview
echo '<note><to>Ann</to></note>' > new_note.xml
gnawtreewriter edit note.xml element_4 --source-file new_note.xml --preview

# If the diff looks good, run without --preview to apply
gnawtreewriter edit note.xml element_4 --source-file new_note.xml
```

Tips: In scripts or CI, it's safest to use `--source-file` or `-` (stdin) when passing code to `edit` to avoid issues with quotes and shell-escaping.

## CI / Hook-exempel

Here's a minimal GitHub Actions example that tests and runs a quick parsing check on push/pull requests:

```yaml
# .github/workflows/validate.yml
name: Validate

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions/setup-rust@v1
      - name: Run tests
        run: cargo test --all
      - name: Install gnawtreewriter
        run: cargo install --path .
      - name: Basic parse validation
        run: |
          # Fail if any XML/Markdown/YAML file cannot be parsed
          for f in $(git ls-files '*.xml' '*.md' '*.yml' '*.yaml' | tr '\n' ' '); do
            if [ -n "$f" ]; then
              ./target/debug/gnawtreewriter analyze "$f" || (echo "Parse failed: $f" && exit 1)
            fi
          done
```

### Pre-commit hook (local)
A simple pre-commit hook to ensure that newly modified XML/Markdown/YAML files can be parsed:
```bash
#!/bin/sh
# .git/hooks/pre-commit
files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(xml|md|yml|yaml)$' || true)
for f in $files; do
  if ! gnawtreewriter analyze "$f" >/dev/null 2>&1; then
    echo "Parse error in $f"
    exit 1
  fi
done
```

## Documentation & Contribute

- Read `CONTRIBUTING.md` for contribution guidelines.
- See `AGENTS.md` and `AI_AGENT_TEST_SCENARIOS.md` for examples of how to use the tool with LLMs and automation.


### add-property
QML-specific command to safely add a property to a component at the correct position.

```bash
gnawtreewriter add-property <file_path> <target_path> <name> <type> <value>

# Example: Add property to Rectangle
gnawtreewriter add-property app.qml "0.1" myProp string "'hello'"
```

### add-component
QML-specific command to safely add a child component.

```bash
gnawtreewriter add-component <file_path> <target_path> <component_name> [--content "props"]

# Example: Add a Button inside a Rectangle
gnawtreewriter add-component app.qml "0.1" Button --content "text: 'Click me'"
```



### list
List all nodes with their paths in a file.

```bash
# List all nodes
gnawtreewriter list <file_path>

# Filter by node type
gnawtreewriter list <file_path> --filter-type Property
```

### find
Find nodes matching criteria across files, including semantic search using local AI.

```bash
# Find by node type
gnawtreewriter find <file_path> --node-type Property

# Find by content
gnawtreewriter find <file_path> --content "mainToolbar"

# Semantic search (requires 'ai setup')
gnawtreewriter find --semantic "where is the database connection?"

# Semantic search in specific directory
gnawtreewriter find --semantic "authentication logic" src/ --device cpu
```

### ai
Manage local AI models for semantic understanding and smart editing.

```bash
# Setup and download ModernBERT
gnawtreewriter ai setup --model modernbert --device cpu

# Check status of installed models
gnawtreewriter ai status

# Semantic search across the project
gnawtreewriter find --semantic "authentication logic"

# AI-powered refactoring suggestions
gnawtreewriter ai refactor src/main.rs

# Context-aware code completion
gnawtreewriter ai complete src/main.rs "0.1.2"
```

### lint
Lint files and show issues with severity levels.

```bash
# Lint single file
gnawtreewriter lint app.py

# Lint directory recursively
gnawtreewriter lint src/ --recursive

# Get JSON output for CI
gnawtreewriter lint . --recursive --format json
```

### Time Travel & Restoration Commands

#### restore-project
Restore entire project to a specific point in time.

```bash
# Preview what would be restored
gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview

# Restore all files to timestamp
gnawtreewriter restore-project "2025-12-27T15:30:00Z"
```

#### restore-session
Undo all changes from a specific AI agent session.

```bash
# Find session ID from history
gnawtreewriter history

# Restore entire session
gnawtreewriter restore-session "session_1766859069329812591" --preview
```

#### restore-files
Selectively restore files modified since a timestamp.

```bash
# Restore Python files modified since timestamp
gnawtreewriter restore-files --since "2025-12-27T16:00:00Z" --files "*.py"
```

### Session Management

#### session-start
Start a new session to group related operations.

```bash
gnawtreewriter session-start
```

#### history
Show transaction history with timestamps.

```bash
# Show recent operations
gnawtreewriter history

# Show more with JSON format
gnawtreewriter history --limit 20 --format json
```

#### undo / redo
Session-based undo and redo operations.

```bash
# Undo last operation
gnawtreewriter undo

# Undo multiple steps
gnawtreewriter undo --steps 3

# Redo operations
gnawtreewriter redo --steps 2
```

### Help & Learning

#### examples
Show practical examples for common tasks.

```bash
# General examples
gnawtreewriter examples

# Topic-specific examples
gnawtreewriter examples --topic editing
gnawtreewriter examples --topic restoration
gnawtreewriter examples --topic qml
```

#### wizard
Interactive help for guided workflows.

```bash
# Start interactive wizard
gnawtreewriter wizard

# Task-specific guidance
gnawtreewriter wizard --task first-time
gnawtreewriter wizard --task editing
gnawtreewriter wizard --task troubleshooting
```

### Version and Help Commands

#### --version
Check your current GnawTreeWriter version.

```bash
gnawtreewriter --version
```

#### --help
Get comprehensive help for any command.

```bash
# General help
gnawtreewriter --help

# Command-specific help
gnawtreewriter edit --help
gnawtreewriter restore-project --help
```

### show
Show content of a specific node.

```bash
gnawtreewriter show <file_path> <node_path>
```

**node_path**: Dot-notation path (e.g., "0.2.1")

### edit
Edit a node's content.

```bash
# Edit node directly
gnawtreewriter edit <file_path> <node_path> <new_content>

# Preview changes without applying
gnawtreewriter edit <file_path> <node_path> <new_content> --preview
```

**Backup**: Every edit automatically creates a timestamped JSON backup in `.gnawtreewriter_backups/`.

**Output**: Success message (or error if node not found).

Replaces entire content of node at `node_path` with `new_content`.
### insert
Insert new content relative to a node.

```bash
gnawtreewriter insert <file_path> <parent_path> <position> <content>
```

**position** values:
- `0`: Insert before the node at `parent_path`
- `1`: Insert after the node at `parent_path`
- `2`: Insert as a child of the node (where applicable)

### delete
Delete a node from the tree.

```bash
gnawtreewriter delete <file_path> <node_path>
```

Removes the node and all its children from the tree.

## Tree Paths

Nodes are addressed using dot-notation:
- `root` - Document root
- `0` - First child of root
- `0.1` - Second child of first root child
- `0.2.1` - Second child of third child of first root child

Example tree:
```
root
├── 0 (Import)
├── 1 (Function)
│   ├── 1.0 (function keyword)
│   ├── 1.1 (function name)
│   └── 1.2 (function body)
│       ├── 1.2.0 (statement 1)
│       └── 1.2.1 (statement 2)
└── 2 (Class)
```

## Architecture

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed technical documentation.

### Additional Documentation

- [Recipes](docs/RECIPES.md) - Common tasks and workflows
- [QML Examples](docs/QML_EXAMPLES.md) - Step-by-step QML editing examples
- [LLM Integration](docs/LLM_INTEGRATION.md) - Guide for language model integration
- [Testing](docs/TESTING.md) - Testing strategies and examples
- [Developer Report](docs/DEVELOPER_REPORT.md) - Feedback and improvement roadmap

## Examples

### Python: Add a function to a module
```bash
# 1. Analyze to find the module path
gnawtreewriter analyze module.py

# 2. Insert new function
gnawtreewriter insert module.py "0" 1 "def new_function(x, y):\n    return x + y"
```

### QML: Change a property value
```bash
# 1. Find the property node path
gnawtreewriter analyze app.qml

# 2. Edit the property
gnawtreewriter edit app.qml "0.1.0" "width: 300"
```

### TypeScript: Add a method to a class
```bash
# 1. Analyze the file
gnawtreewriter analyze app.ts

# 2. Find the class block path
gnawtreewriter show app.ts "1.3"

# 3. Insert the new method
gnawtreewriter insert app.ts "1.3" 2 "newMethod(): void { console.log('hello'); }"
```

## AI Agent Integration

GnawTreeWriter is designed from the ground up for AI-native development workflows.

### Revolutionary Capabilities for AI Agents
- **Temporal Project Management**: Restore entire projects to specific timestamps
- **Session-based Workflows**: Track and undo complete AI agent sessions
- **Safe Experimentation**: Preview all changes before applying
- **Tree-based Editing**: No more bracket-matching or indentation errors
- **Comprehensive Help System**: Interactive learning and troubleshooting

### Perfect for AI Development Workflows
```bash
# Start AI session
gnawtreewriter session-start

# Make multiple changes safely
gnawtreewriter edit file.py "0.1" 'new code' --preview
gnawtreewriter add-component ui.qml "0" Button

# If something goes wrong, undo entire session
gnawtreewriter restore-session "session_id"
```

### Built-in Testing Framework
See [AI_AGENT_TEST_SCENARIOS.md](AI_AGENT_TEST_SCENARIOS.md) for comprehensive testing scenarios designed specifically for AI agents.

### Multi-Agent Development Proven
This tool was built using multi-agent collaboration (Claude, Gemini, GLM-4.7, Raptor Mini), proving that human-AI collaborative development is not just possible, but superior.

For complete integration guide, see [docs/LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md).

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Development Workflow

1. Make changes to parser or core logic
2. Test with example files in `examples/` directory
3. Run `cargo check` for compilation errors
4. Commit with descriptive message following conventional commits
5. Update CHANGELOG.md with user-facing changes

### Adding New Languages

1. Create new parser file in `src/parser/{language}.rs`
2. Implement `ParserEngine` trait
3. Add to `src/parser/mod.rs` in `get_parser()`
4. Update Cargo.toml with TreeSitter dependency
5. Add example file in `examples/`
6. Update README and documentation

## Contributing

We welcome contributions! Areas of interest:

- **More languages**: Add parsers for JavaScript, Go, Java, C++, etc.
- **Better QML parsing**: Improve nested component handling
- **Diff preview**: Show what will change before applying edits
- **Undo/redo**: Track and revert changes
- **LSP integration**: Provide language server protocol support
- **VSCode extension**: Create editor plugin
- **Testing**: Add test suite with edge cases

### For Language Models

If you're testing GnawTreeWriter with an LLM:

1. Start with the example files in `examples/`
2. Try basic edits (property changes, simple insertions)
3. Move to complex edits (nested structures, multiple changes)
4. Report issues or confusing behavior
5. Suggest improvements to the edit intents or API

Your feedback is crucial for making this tool truly LLM-friendly!

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## Roadmap

### v0.2.0
- [ ] JavaScript parser (using existing TypeScript parser)
- [ ] Go language support
- [ ] Improved QML parser with better nesting
- [ ] Diff generation and preview

### v0.3.0
- [ ] Batch undo/redo
- [ ] Context-aware suggestions
- [ ] VSCode extension
- [ ] Python API/SDK

### v0.5.0 (AI Integration)
- [x] CLI structure for local AI (ModernBERT)
- [x] Local inference engine using Candle (Rust-native)
- [ ] Semantic search and node discovery
- [ ] Intent-based edit suggestions

### Future
- [ ] More languages (Java, C++, C#, etc.)
- [ ] LSP server
- [ ] Web interface
- [ ] AI-powered refactoring suggestions

## Design Principles

### **Safety by Design**
- **Directory handling**: Requires explicit `--recursive` flag for directories to prevent accidental large scans
- **Preview first**: All destructive operations support `--preview` to show changes before applying
- **Automatic backups**: Every edit creates timestamped backups before making changes
- **Syntax validation**: Code changes are validated before saving to prevent corruption

### **Clear Intent**
- **Explicit flags**: Directory operations require `--recursive` to make intent clear
- **Helpful errors**: Error messages provide specific guidance and examples
- **No surprises**: Commands do exactly what they say, nothing hidden or automatic

## Known Limitations

- **Directory analysis**: Intentionally requires `--recursive` flag for directories (safety feature)
- **QML instantiation**: Parse success doesn't guarantee runtime QML instantiation success  
- **Large projects**: Very large projects may require patience for full analysis
- **Hash matching**: Occasional backup hash mismatches resolved with timestamp fallback

## Version History & Feature Availability

- **v0.2.1**: Complete time restoration system, interactive help system (`examples`, `wizard`), AI testing framework, `lint` command, `--version` flag
- **v0.2.0**: Multi-file support, transaction logging, session management (`restore-project`, `restore-session`)  
- **v0.1.x**: Basic tree editing and QML support (`analyze`, `edit`, `add-property`)

**Current version check**: `gnawtreewriter --version`  
**Feature verification**: All examples in this README are tested with v0.2.1+

## License

This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

See [LICENSE](LICENSE) file for details.

### Why MPL 2.0?

The Mozilla Public License 2.0 ensures that:
- ✅ **Core improvements are shared** - Modifications to GnawTreeWriter files must be contributed back
- ✅ **Commercial use allowed** - You can build commercial products with GnawTreeWriter
- ✅ **Integration friendly** - You can combine it with proprietary code in separate files
- ✅ **Patent protection** - Explicit patent grant protects both contributors and users

This means if you improve the parser or core functionality, those improvements benefit everyone. But you can still build proprietary integrations and commercial products around it.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be under the terms of the Mozilla Public License 2.0, without any additional terms or conditions.

## Getting Help

- **Interactive Help**: `gnawtreewriter wizard` for guided assistance
- **Examples**: `gnawtreewriter examples` for practical workflows  
- **Command Help**: `gnawtreewriter <command> --help` for detailed usage
- **AI Agent Testing**: See [AI_AGENT_TEST_SCENARIOS.md](AI_AGENT_TEST_SCENARIOS.md)
- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: Complete handbook in [GnawTreeWriter_docs/](GnawTreeWriter_docs/)

## Acknowledgments

- TreeSitter for excellent parser grammar framework
- Rust community for the amazing tooling
- All contributors and testers
- Special thanks to LLM models like Claude Sonnet 4.5, GLM-4.7, Gemini 3: Flash and Raptor mini for making this project possible.

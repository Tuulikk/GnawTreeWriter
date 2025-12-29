# GnawTreeWriter

Tree-based code editor for LLM-assisted editing. Edit code files based on tree structure levels, avoiding bracket issues and structural problems from LLM code generation.

## Features

- **Multi-language support**: Python, Rust, TypeScript/TSX, PHP, HTML, QML, Go, CSS, YAML, TOML, JSON
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

## Why Use GnawTreeWriter?

### Problems with Traditional LLM Code Editing
- LLMs often struggle with matching brackets
- Indentation errors are common
- Structural changes can break code
- Hard to make precise, targeted edits

### How GnawTreeWriter Solves This
- **No bracket management**: AST handles structure automatically
- **No indentation worries**: Formatting is preserved with smart indentation
- **Syntax Validation**: proposed edits are checked against the parser before saving
- **Precise targeting**: Edit specific nodes at specific paths
- **Deterministic results**: Same input always produces same output
- **Context-aware**: LLM can understand surrounding code structure

---

## Installation

### From Source

```bash
git clone https://github.com/Tuulikk/GnawTreeWriter.git
cd GnawTreeWriter
cargo build --release
```

The binary will be at `target/release/gnawtreewriter`.

### Using cargo install (Recommended)

```bash
cargo install --git https://github.com/Tuulikk/GnawTreeWriter.git
```

### From Binary Release (Future)

Once releases are published:
```bash
# Download binary for your platform
chmod +x gnawtreewriter
sudo mv gnawtreewriter /usr/local/bin/
```

## Quick Start

### First Time? Get Interactive Help!

```bash
# Get comprehensive help and examples
gnawtreewriter --help
gnawtreewriter examples
gnawtreewriter wizard --task first-time
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
| TypeScript | `.ts`, `.tsx` | TreeSitter | ✅ Stable |
| JavaScript | `.js`, `.jsx` | TreeSitter | ✅ Stable |
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

GnawTreeWriter använder två huvudsakliga parsing-strategier beroende på format:

- TreeSitter-baserade grammatikparsers — ger en komplett och detaljerad AST, bra för programmeringsspråk (t.ex. Python, Rust, TypeScript) när en stabil TreeSitter-grammar finns. Ger precisa mutationer på nod-nivå, men kräver att grammar-crates använder kompatibla `tree-sitter`-versioner (annars kan det uppstå länk- och kompabilitetsproblem).

- Biblioteksbaserade parsers — exempelvis `xmltree` för XML, `serde_json` för JSON, `toml` för TOML och `serde_yaml` för YAML. Dessa är ofta mer robusta för konfigurations- och dokumentformat, undviker FFI-beroenden och är pålitliga när TreeSitter inte är lämpligt.

I det här projektet:
- Vi använder TreeSitter där det ger fördel (språk/syntax där grammatik är bra).
- För format där TreeSitter ger problem eller inte är nödvändigt (t.ex. XML) använder vi stabila bibliotek (`xmltree`) och mappar resultatet till samma `TreeNode`-modell. Det ger stabil parsing och korrekta radnummer för `list`/`show`/`edit`.

Kort guide:
- Vill du ha maximal AST-precision och nodes: satsa på TreeSitter om grammatiken finns.
- Vill du ha robust dokument-/konfigparser utan FFI-komplexitet: använd bibliotek (som vi gjort för XML).

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

Praktiska exempel som visar vanliga arbetsflöden. Använd `--preview` för att se diff innan du applicerar ändringen, och använd `--source-file` för att undvika shell-citatproblem vid större snippets.

### Batch Operations

Koordinera ändringar över flera filer med atomiska batch-operationer:

```bash
# Skapa batch-specifikation
cat > update.json << 'EOF'
{
  "description": "UI tema och API uppdatering",
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

**Se [BATCH_USAGE.md](BATCH_USAGE.md) för komplett dokumentation och fler exempel.**

### Snabbkommandon
```bash
# Analysera en fil (skriv ut AST i JSON)
gnawtreewriter analyze note.xml

# Lista noder och deras dot-paths
gnawtreewriter list note.xml

# Visa innehåll i en nod
gnawtreewriter show note.xml element_4

# Förhandsgranska en ändring (läs ny kod från fil för att slippa quoting)
gnawtreewriter edit note.xml element_4 --source-file replacement.xml --preview
```

### Named references (tags)
```bash
# Tilldela en namnreferens till en node-path i en fil (t.ex. 'my_function' -> '0.1.2')
gnawtreewriter tag add main.rs "0.1.2" "my_function"

# Lista alla taggar för en fil
gnawtreewriter tag list main.rs

# Ta bort en tagg från en fil
gnawtreewriter tag remove main.rs "my_function"

# Byt namn på en tagg
gnawtreewriter tag rename main.rs "my_function" "main_function"
# (lägg till --force för att skriva över befintlig tag)

# Redigera via inline-tag-syntax (använd 'tag:<name>' som node-path)
gnawtreewriter edit main.rs tag:my_function 'def updated():\n    print("Updated")' --preview

# Alternativt kan du även använda --tag som flagga:
gnawtreewriter edit --tag my_function main.rs 'def updated():\n    print("Updated")'
```

### XML-exempel
```bash
# Steg 1: analysera och hitta mål-nod
gnawtreewriter analyze note.xml
gnawtreewriter list note.xml

# Steg 2: visa noden och bestäm vad du vill ändra
gnawtreewriter show note.xml element_4

# Steg 3: ändra noden med fil (säkert mot shell-escaping) och förhandsgranska
echo '<note><to>Ann</to></note>' > new_note.xml
gnawtreewriter edit note.xml element_4 --source-file new_note.xml --preview

# Om diffen ser bra ut, kör utan --preview för att applicera
gnawtreewriter edit note.xml element_4 --source-file new_note.xml
```

Tips: I scripts och CI är det säkrast att använda `--source-file` eller `-` (stdin) när du skickar kod till `edit` för att undvika problem med citattecken och shell-escaping.

## CI / Hook-exempel

Här är ett minimalt GitHub Actions-exempel som testar och kör en snabb kontroll av parsing på push/pull requests:

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

### Pre-commit hook (lokalt)
Ett enkelt pre-commit-hook som kontrollerar att nya ändrade XML/Markdown/YAML-filer kan parses:
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

- Läs `CONTRIBUTING.md` för bidragsriktlinjer.
- Se `AGENTS.md` och `AI_AGENT_TEST_SCENARIOS.md` för exempel på hur man använder verktyget tillsammans med LLMs och automatisering.


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
Find nodes matching criteria across files.

```bash
# Find by node type
gnawtreewriter find <file_path> --node-type Property

# Find by content
gnawtreewriter find <file_path> --content "mainToolbar"

# Find in directory
gnawtreewriter find app/ui/qml/ --content "width:"
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

MIT License - see LICENSE file for details

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

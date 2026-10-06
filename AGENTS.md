# Contributing to GnawTreeWriter as an AI Agent

**Guide for AI agents contributing to the GnawTreeWriter project through dogfooding**

Version: 2.1 | Last Updated: 2026-10-06

---

## ⚡ Quick Rule for AI Agents: Use GnawTreeWriter to edit this repo

**Default to GnawTreeWriter for code edits. Text-replace tools (`sed`, `replace`, `write_file`) are fallback only.**

This is the GnawTreeWriter repository. The tool is available to you — use it to
edit the codebase, the way it is meant to be used.

### Decision rule
| Situation | Use |
|---|---|
| Editing a function, struct, class, or named code block | **GnawTreeWriter** (AST edit) |
| Inserting a new node at a structural position | **GnawTreeWriter** (insert) |
| Renaming / refactoring within a file | **GnawTreeWriter** |
| Reading a specific block without dumping the file | **GnawTreeWriter** (read_node / skeleton) |
| Multi-file coordinated change | **GnawTreeWriter** (batch) |
| Editing prose, README, CHANGELOG, comments-only | text tools are fine |
| File type not in the supported list | text tools are fine |
| AST edit failed twice with corrected input | fall back to text tools |

### How to reach the tool
- **In VSCode / Copilot Chat**: MCP tools prefixed `mcp_gnawtreewrite_*`
  (`edit_node`, `semantic_edit`, `insert_node`, `batch`, `preview_edit`, `compress`, `pack`, `curate`, ...).
- **In a terminal / CLI agent**: the `gnawtreewriter` binary
  (`gnawtreewriter edit <file> <path> -`, `gnawtreewriter batch <spec.json>`, ...).

### 🧠 Lektioner från agent-sessioner (läs detta — det är dyr info)

Dessa är icke-uppenbara fakta som orsakat riktiga problem under
sessioner. De gäller oavsett vilket verktyg du använder.

1. **Verifiera ALLTID GTW-skrivningar efteråt** (grep/omläs noden).
   `quick-replace` har en gång rapporterat "✓ applied" utan att ändra
   ett enda byte (fynd #14 — gardet finns nu i binären, men vanan är
   policy: en opverifierad "framgång" ledde till en hel session felaktigt
   diagnoserade buggar).
2. **`quick-replace` ersätter ALLA förekomster per anrop** — loopa
   ALDRIG ett ersättningssats där den nya texten innehåller den gamla
   (blir exponentiell dubblett-explosion: 336 rader på minuter). Och
   argument som börjar med `-` kräver `--`-avgränsare
   (`quick-replace FIL -- '-sök' 'ersätt'`).
3. **Källkod ≠ installerad binär.** Efter kodändringar är PATH:ens
   `gnawtreewriter` ofta gammal (nytt flagga → "unrecognized subcommand",
   fixar saknas). Bygg om: `scripts/build-gpu.sh` (GPU, podman) eller
   `cargo install --path . --features modernbert,mcp`. Kontrollera med
   `gnawtreewriter --version` mot `Cargo.toml`. Debug-bin:
   `$CARGO_TARGET_DIR/debug/gnawtreewriter`.
4. **MCP-daemoner lever kvar** i andras sessioner med binären de startade
   med. Symptom: "gamla beteenden trots ny version". Kontroll:
   `readlink /proc/<pid>/exe` → `(deleted)` = stela. `kill <pid>` — värden
   spawnar nytt vid nästa anrop (inte omedelbart).
5. **Feature-matrisen:** default OCH `mamba` båda har `modernbert` på —
   cfg-hål utanför modernbert syns BARA i
   `cargo check --no-default-features --all-targets` (CI kör det;
   kör det lokalt om du rört `#[cfg]`/kommandon). LLM-vägar kräver
   `--features mamba`.
6. **Integrationstester MÅSTE vara hermetiska.** Aldrig operera på
   repo-root-state (transaktionslogg/git-rötter): CI-checkoutar har
   gitignorerade loggar, och lokalt korrumperade test ens eget repo (det
   som såg ut som en "resurrection-bugg" var bara test som spelade
   transaktioner). Recept: `tempfile::tempdir()` + `.git`-markör, eller
   `serve_with_shutdown_root(listener, token, root, shutdown)`. Tunga
   modelltester är `#[ignore]` — kör med
   `cargo test -- --ignored --nocapture`.
7. **Om "gammalt innehåll återkommer" efter GTW-editer** — misstänk EJ
   GTW-cache (den misstänktes felaktigt en gång): kontrollera först
   uppspelning av transaktionsloggar (punkt 6) och andra processer som
   skriver filen. Och verifiera bytes — `git diff` är sanningen.
8. **Redigeringsordning (policy):** GTW → OpenCode edit-verktyget →
   *aldrig* python/`sed -i`/`dd`/`tee` på projektfiler (de kringgår både
   GTW-validering och shadow-checkpoints; reglerna blockerar dem också).
   GTW-fel är FYND: logga i `GTW_MCP_ISSUE_LOG.md` (verktyg + exakta
   parametrar + svar + omväg), avboka sedan en kategori — varje stängd
   post blev en riktig fix. Eskalering: ett fel → EN omväg, gå vidare,
   logga sedan. Aldrig `git checkout` av GTW-edits (använd `undo` /
   `restore-project --preview`).
9. **CI innan du litar på grön status:** `validate.yml` (test + clippy
   `-D warnings` + no-default-features) och `mcp-examples.yml`
   (integration + Node/Python/Rust-klienter, **matris = två OS-ben ≈
   8–10 min**) — en "in_progress"-fläta i flera minuter är normal.
10. **Release:** följ "Release Process"-sektionen nedan; lib-yteändringar
    (enum-/struct-fält, nya obligatoriska parametrar) kräver en
    `### BREAKING (lib)`-sektion i CHANGELOG **innan** taggen — Motor2
    och andra path-dep:ar litar på att brytningar ropas ut.
11. **GPU för utvecklare:** `gnawtreewriter.yaml` → `indexing.device: auto`
    (enda konfigen; 20 %-VRAM-gate gäller alltid) + bygg med
    `scripts/build-gpu.sh` (podman, ingen CUDA-toolkit krävs på värden).
    GPU är ALDRIG default — CPU är produkten, GPU är opt-in.
12. **Diagnostik-filer att läsa innan du gissar:** `.gnawtreewriter_search_log.jsonl`
    (misstänkta/nollträff-sökningar med prior_failures),
    `.gnawtreewriter_metrics.json` (duplexräknare),
    `gnawtreewriter doctor` (hälsa), `history` (vad GTW ändrade).

### GPU indexing (opt-in; devs recommended)

GPU use is **off by default** — GnawTreeWriter was designed CPU-only and
stays conservative for users. The recommendation for developer machines is
to change the flag in the file: set

```yaml
# gnawtreewriter.yaml (project root)
indexing:
  device: auto
```

That single flag is the whole configuration. `auto` enables the GPU for
`ai index` only when the binary was built with `--features cuda` **and**
≥20% of VRAM is free (never squeeze the OS or other GPU users); everything
else always stays on CPU. One-off alternative: `gnawtreewriter ai index --gpu`.

Building the cuda binary (no host CUDA toolkit needed — podman + NVIDIA
driver is enough): `scripts/build-gpu.sh`. Without that build the flag
harmlessly resolves to CPU with an explanatory log line.

### För agenter som ANVÄNDAR GTW (icke-repotsspecifikt)

**Diagnostisk kedja** (använd i den här ordningen — gissna aldrig radnummer):

1. `analyze`/`get_skeleton` → filens struktur och nod-sökvägar
2. `sense`/`search_nodes` → hitta exakt nod ("var är X?" → sense; exakt text → search_nodes)
3. `read_node` → verifiera innehållet du tänker ändra
4. `preview_edit` → diffen, skriver ingenting
5. `edit_node`/`semantic_edit`/`insert_node` → skriv (syntax valideras FÖRE skrivning)
6. Verifiera (`cargo check`/test) → vid fel: `undo` (transaktionslogg), aldrig `git checkout`

**Noteringstaxonomi** (något strular → logga i `GTW_MCP_ISSUE_LOG.md`, inte bara
åsidosätt): ange verktyg + exakta parametrar + vad som svarade + vad du använde
i stället. Avbocka sedan en av: *användarfel / bugg / saknad funktion /
sub-funktion saknas / svår att nå / inte hittad vid behov / under förmåga*.
Varje stängd post i den loggen motsvarar en riktig fix (se hela listan — stubbar,
tysta tomma svar och felaktiga "applied"-rapporter kom alla därifrån).

**Eskalationsregel:** ett timeout/fel → EN omväg (grep/Read/CLI), gå vidare med
uppgiften, logga sedan — aldrig upprepa samma anrop blint, aldrig tysta
feladrättelser (t.ex. `git checkout` av GTW-edits). Misstänker du en
"framgång" utan byte? Verifiera bytes (`git diff` / läs om noden) och logga —
GTW:s fel ska vara högljudda, inte tysta (fynd #14).

### Why this matters
GnawTreeWriter validates syntax **before** writing and targets the smallest
possible node. Text-replace can silently corrupt syntax on multi-line edits and
match unintended locations. When you skip the AST, you lose the safety net this
project exists to provide.

See `.github/copilot-instructions.md` and
`.github/instructions/gnawtreewriter-edit.instructions.md` for the full policy.

---

## 🎯 Purpose

This document explains how AI agents can contribute to the GnawTreeWriter project by using the tool to develop itself—a practice known as "dogfooding." By using GnawTreeWriter to edit its own codebase, AI agents help validate functionality, discover edge cases, and improve the tool's design.

---

## 🏗️ Project Overview

GnawTreeWriter is a tree-based code editor written in Rust that works at the AST (Abstract Syntax Tree) level. It uses TreeSitter parsers to support multiple programming languages.

### Tech StackJ
- **Language**: Rust (Edition 2021)
- **Parsers**: TreeSitter with language-specific grammars
- **CLI**: Clap 4.5
- **Serialization**: serde, serde_json, serde_yaml, toml
- **Async Runtime**: Tokio

### Project Structure
```
GnawTreeWriter/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli.rs               # Command-line interface definitions
│   ├── core/                # Core functionality
│   ├── parser/              # Parser engine implementations
│   └── llm/                 # LLM integration utilities
├── docs/                    # Detailed documentation
├── examples/                # Example files for testing
├── tests/                   # Test files
└── Cargo.toml              # Dependencies and metadata
```

### Supported Languages
- Python (`.py`)
- Rust (`.rs`)
- C (`.c`, `.h`)
- C++ (`.cpp`, `.hpp`, `.cc`, `.cxx`, `.hxx`, `.h++`)
- Java (`.java`)
- Zig (`.zig`)
- TypeScript (`.ts`, `.tsx`)
- JavaScript (`.js`, `.jsx`)
- Bash (`.sh`, `.bash`)
- PHP (`.php`)
- HTML (`.html`)
- QML (`.qml`)
- Go (`.go`)
- CSS (`.css`)
- YAML (`.yaml`, `.yml`)
- TOML (`.toml`)
- XML (`.xml`)
- Markdown (`.md`, `.markdown`)

---

## 🐶 Dogfooding: Using GnawTreeWriter to Edit GnawTreeWriter

The best way to contribute to GnawTreeWriter is to use it! This practice—eating your own dog food—helps identify issues and validate the tool's capabilities.

### Getting Started

1. **Install GnawTreeWriter** (from source if contributing):
   ```bash
   cd GnawTreeWriter
   cargo install --path .
   ```

2. **Start a development session**:
   ```bash
   gnawtreewriter session-start
   ```

3. **Analyze the codebase**:
   ```bash
   # Analyze the CLI module
   gnawtreewriter analyze src/cli.rs
   
   # List all functions in main.rs
   gnawtreewriter list src/main.rs --filter-type function_definition
   ```

### Example Workflows

#### Adding a New CLI Command

**Scenario**: Lägg till ett nytt delkommando. Verkligt exempel:
`validate` — **finns nu!** (`gnawtreewriter validate <file>`, både CLI och
MCP `validate {file_path}`). Stegen nedan visar mönstret:

```bash
# Step 1: Analyze the CLI structure
gnawtreewriter analyze src/cli.rs

# Step 2: Find the Commands enum
gnawtreewriter list src/cli.rs --filter-type enum_item

# Step 3: Add the variant — anchor an existing variant, prepend the new one (preview first)
gnawtreewriter quick-replace src/cli.rs '    Status {' '    Validate {
        /// File to validate (strict parse; nonzero exit on syntax error)
        file: PathBuf,
    },
    Status {' --preview

# Step 4: Apply the same replacement WITHOUT --preview (preview wrote nothing —
# the anchor still exists, so one apply = one insertion, never a duplicate)
gnawtreewriter quick-replace src/cli.rs '    Status {' '    Validate {
        /// File to validate (strict parse; nonzero exit on syntax error)
        file: PathBuf,
    },
    Status {'

# Step 5: Add the dispatch arm the same way (anchor = an existing arm)
gnawtreewriter list src/main.rs --filter-type match_arm
gnawtreewriter quick-replace src/main.rs '            Commands::Status => {' '            Commands::Validate { file } => {
                Self::handle_validate(&file)?;
            }
            Commands::Status => {' --preview
```

#### Implementing a New Parser

**Scenario**: Add support for a new language (e.g., Java)

```bash
# Step 1: Create new parser file
cat > src/parser/java.rs << 'EOF'
use crate::parser::ParserEngine;
use anyhow::Result;
use tree_sitter::Parser;

pub struct JavaParser;

impl ParserEngine for JavaParser {
    fn parse(&self, code: &str) -> Result<crate::core::TreeNode> {
        let mut parser = Parser::new();
        let language = unsafe {
            std::mem::transmute::<tree_sitter_language::LanguageFn, fn() -> tree_sitter::Language>(
                tree_sitter_java::LANGUAGE,
            )()
        };
        parser.set_language(&language)?;
        // Implementation details...
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["java"]
    }
}
EOF

# Step 2: Update Cargo.toml to add dependency
gnawtreewriter quick-replace Cargo.toml 'tree-sitter-bash = "0.25.1"' 'tree-sitter-java = "0.23"' --preview

# Step 3: Update parser/mod.rs to register the parser
gnawtreewriter quick-replace src/parser/mod.rs '"go" => Ok' '"java" => Ok(Box::new(java::JavaParser::new())),' --preview
```

#### Adding Tests

**Scenario**: Add a unit test for a function

```bash
# Step 1: Find the function in src/core/
gnawtreewriter list src/core/mod.rs --filter-type function_definition

# Step 2: Add test module
gnawtreewriter quick-replace src/core/mod.rs "#[cfg(test)]" '
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_functionality() {
        // Test implementation
        assert!(true);
    }
}' --preview
```

#### Updating Documentation

**Scenario**: Document a new feature in README.md

```bash
# Step 1: Analyze README structure
gnawtreewriter list README.md --filter-type heading

# Step 2: Find the CLI Commands section
gnawtreewriter search README.md "## CLI Commands"

# Step 3: Insert documentation for the new command AFTER the heading
# (quick-insert adds after matching lines; --unique inserts only at the first match)
gnawtreewriter quick-insert README.md --after "## CLI Commands" '
### validate
Check file syntax without making changes.

```bash
gnawtreewriter validate <file_path>
```' --unique --preview
```

#### Batch Multi-File Operations

**Scenario**: Coordinated changes across multiple files using atomic batch operations

```bash
# Step 1: Create batch specification for coordinated changes
cat > batch_update.json << 'EOF'
{
  "description": "Update UI theme and backend API",
  "operations": [
    {
      "type": "edit",
      "file": "src/main.qml",
      "path": "1.1.3.2.0.1",
      "content": "darkblue"
    },
    {
      "type": "insert",
      "file": "src/main.qml",
      "parent_path": "1.1",
      "position": 2,
      "content": "radius: 8"
    },
    {
      "type": "edit",
      "file": "src/api.py",
      "path": "1.2.1",
      "content": "def get_theme(): return 'darkblue'"
    }
  ]
}
EOF

# Step 2: Preview batch changes (recommended first)
gnawtreewriter batch batch_update.json --preview

# Step 3: Apply batch atomically
gnawtreewriter batch batch_update.json

# Step 4: Verify changes with history
gnawtreewriter history

# Step 5: Rollback if needed
gnawtreewriter undo --steps 3
```

**Key Benefits for AI Agents:**
- ✅ All operations validated in-memory before any writes
- ✅ Automatic rollback if any operation fails
- ✅ Single transaction history for coordinated changes
- ✅ Perfect for multi-agent workflows and refactoring

#### Quick Command (Fast, Single-File Edits)

**Scenario**: Make a quick edit to a single file with minimal overhead

```bash
# Node-edit mode (AST-based): edit node at path — preview first, then apply
gnawtreewriter edit app.py "0.1.0" 'def new_function():' --preview
gnawtreewriter edit app.py "0.1.0" 'def new_function():'

# Find/replace mode (text-based): replaces ALL occurrences — preview first, then apply
gnawtreewriter quick-replace app.py 'old_function' 'new_function' --preview
gnawtreewriter quick-replace app.py 'old_function' 'new_function'
```

**Key Benefits for AI Agents:**
- ✅ Lower overhead than full batch operations
- ✅ Perfect for single-line or simple edits
- ✅ Preview mode for safe exploration
- ✅ Automatic backup and transaction logging
- ✅ Parser validation for supported file types

#### Diff-to-Batch (AI Agent Integration)

**Scenario**: Convert a unified diff (from git or AI agent) to a safe batch operation

```bash
# Step 1: Generate or receive a diff
git diff > changes.patch
# OR AI agent produces diff as output

# Step 2: Preview the diff
gnawtreewriter diff-to-batch changes.patch

# Step 3: Convert to batch JSON
gnawtreewriter diff-to-batch changes.patch --output batch.json

# Step 4: Review batch preview
gnawtreewriter batch batch.json --preview

# Step 5: Apply with safety
gnawtreewriter batch batch.json
```

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

**Key Benefits for AI Agents:**
- ✅ AI agents can output standard unified diffs
- ✅ Automatic conversion to safe batch operations
- ✅ Full validation before application
- ✅ Atomic rollback if diff fails to apply
- ✅ Transaction logging for undo/redo
- ✅ Preview mode to review before applying

---

## 🤝 Contribution Areas

AI agents can contribute to several areas of the project:

### 1. Parser Development
- Add support for new languages (Java, Kotlin, Swift, etc.)
- Improve existing parsers (better AST representation, edge cases)
- Optimize parser performance

### 2. Core Functionality
- Implement new edit operations (move, rename, refactor, clone)
- Improve tree navigation and path resolution
- Enhance validation and error reporting
- Batch operations for coordinated multi-file edits

### 3. CLI Features
- Add new commands (lint, format, refactor)
- Improve existing command ergonomics
- Add better help text and examples

### 4. Testing & Quality
- Add unit tests for existing functionality
- Create integration tests for end-to-end workflows
- Improve test coverage in untested areas

### 5. Documentation
- Update README with new features
- Improve inline code documentation
- Create examples in `examples/` directory
- Write technical guides in `docs/`

### 6. Bug Fixes
- Identify and fix parsing issues
- Resolve edge cases in edit operations
- Improve error messages and handling

### 7. Performance Optimization
- Optimize tree traversal algorithms
- Improve parser memory usage
- Speed up file operations

---

## 📋 Project-Specific Conventions for AI Agents

### Code Style
- **Rust**: Follow standard `cargo fmt` formatting
- Use `cargo clippy` for linting
- Prefer `Result<T>` for error handling over `panic!`
- Document public functions with `///` doc comments

### Git Workflow
- Create feature branches: `feature/add-language-X`
- Write descriptive commit messages
- Reference issues in commit messages: `Fixes #123`
- Keep commits atomic (one logical change per commit)

### Testing Requirements
- All new features must have tests
- Run `cargo test` before submitting
- Ensure all existing tests pass
- Test with multiple file types when applicable

### Documentation Requirements
- Update README.md for user-facing changes
- Add inline documentation for new APIs
- Update CHANGELOG.md for version changes
- Document breaking changes clearly

### Error Handling Patterns
```rust
// Preferred: Return Result
fn parse_file(path: &Path) -> Result<TreeNode> {
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
    // ...
}

// Avoid: panic! unless absolutely necessary
fn parse_file(path: &Path) -> TreeNode {
    let content = fs::read_to_string(path).unwrap(); // ❌ Don't do this
    // ...
}
```

---

## 🔧 Practical Contribution Examples

### Example 1: Adding Fuzzy Search Feature

**Goal**: Add a fuzzy search command to find nodes by approximate name

```bash
# Step 1: Analyze the find command structure
gnawtreewriter analyze src/cli.rs
gnawtreewriter search src/cli.rs "Commands::Find"

# Step 2: Add fuzzy command to enum
gnawtreewriter quick-replace src/cli.rs "Find" 'Fuzzy { file: PathBuf, query: String }' --preview

# Step 3: Implement fuzzy matching in core
# Create src/core/fuzzy.rs with fuzzy search logic
# Then add handler in main.rs
```

### Example 2: Improving Error Messages

**Goal**: Make error messages more actionable

```bash
# Step 1: Find error handling code
gnawtreewriter list src/core/mod.rs --filter-type function_definition
gnawtreewriter search src/core/mod.rs "map_err"

# Step 2: Improve error message
gnawtreewriter quick-replace src/core/mod.rs "map_err" '.map_err(|e| {
    anyhow::anyhow!(
        "Failed to parse '{}': {}. Tip: Check file syntax with `gnawtreewriter validate`",
        path.display(),
        e
    )
})' --preview
```

### Example 3: Adding Session Management

**Goal**: Track editing history across sessions

```bash
# Step 1: Design session structure in core
gnawtreewriter insert src/core/mod.rs "pub struct TreeNode;" 0 '
pub struct Session {
    pub id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub operations: Vec<Operation>,
}

pub struct Operation {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub file: PathBuf,
    pub action: String,
}' --preview

# Step 2: Add CLI commands for session management
# Step 3: Implement persistence logic
```

---

## 🧪 Testing Your Contributions

Before submitting contributions, ensure:

```bash
# Format code
cargo fmt

# Check for linting issues
cargo clippy -- -D warnings

# Run all tests
cargo test

# Run specific test
cargo test test_parser

# Build release version to ensure it compiles
cargo build --release
```

---

## 📝 Creating Pull Requests

When submitting changes:

1. **Fork the repository** on GitHub
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make changes using GnawTreeWriter** (dogfooding!)
4. **Commit changes**: `git commit -m "Add amazing feature"`
5. **Push to branch**: `git push origin feature/amazing-feature`
6. **Open Pull Request** with:
   - Clear title describing the change
   - Description of what was changed and why
   - Link to relevant issues
   - Screenshots if UI-related

### PR Template
```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Added tests for new functionality
- [ ] All existing tests pass
- [ ] Tested with `cargo test`

## Dogfooding
I used GnawTreeWriter to make these changes using these commands:
- `gnawtreewriter analyze ...`
- `gnawtreewriter edit ...`
```

---

## 🚀 Advanced Contribution Topics

### Adding TreeSitter Grammars

To add a new language with TreeSitter:

1. Add dependency to `Cargo.toml`
2. Create parser module in `src/parser/`
3. Implement `ParserEngine` trait
4. Register parser in `main.rs`
5. Add tests in `tests/parser_*.rs`

### Custom Parsers

For languages without good TreeSitter support:

1. Use alternative parsing libraries (xmltree, serde_json, etc.)
2. Implement custom parsing logic
3. Ensure consistent `TreeNode` output format
4. Document parsing limitations

### CLI Plugin Architecture

For extensibility:

1. Design plugin interface in `src/plugins/`
2. Implement command registration system
3. Add plugin discovery mechanism
4. Document plugin development

### Add-ons (LSP & MCP)

Add-ons are opt-in extensions that augment GnawTreeWriter without bloating the core tool. They let users attach additional capabilities (semantic analysis, live monitoring, agent orchestration) when and where they want them.

- LSP add-ons (local / OSS): provide semantic features—hover/definition/diagnostics/completion—by connecting to a language server. These are optional, local add-ons that users can enable for richer editor feedback.
- MCP add-ons / Daemon (local OSS + optional cloud/premium): an active local daemon for monitoring projects, coordinating agent workloads, and exposing integration endpoints. A separate premium cloud offering could be provided under a different product name for users needing hosted/managed capabilities.

Key ideas:
- Keep GnawTreeWriter core 100% free and focused on AST-based editing and temporal control.
- Make advanced capabilities available as opt-in add-ons so each user can choose the level of integration they want.
- Document add-on APIs so third parties can implement safe, well-behaved integrations.

See ROADMAP.md for the planned timeline and details about add-ons and MCP: `docs/ROADMAP.md` (search for \"Add-ons & LSP\").

---

## 🤖 AI Agent Best Practices

### 1. Always Analyze First
Before making changes, understand the code structure:
```bash
gnawtreewriter analyze <file>
gnawtreewriter list <file> --filter-type <type>
```

### 2. Use Preview Mode
Never apply changes without previewing:
```bash
gnawtreewriter edit <file> <path> <content> --preview
```

### 3. Start Sessions
Track your work with sessions:
```bash
gnawtreewriter session-start
# Make changes
gnawtreewriter history
```

### 4. Test Incrementally
Test each small change:
```bash
cargo test
gnawtreewriter validate <test_file>
```

### 5. Undo Often
If something goes wrong:
```bash
gnawtreewriter undo
# or
gnawtreewriter restore-session <session_id>
```

### 6. Document Your Work
Leave clear comments and docstrings:
```rust
/// Parses a file using the appropriate parser based on extension.
/// 
/// # Arguments
/// * `path` - Path to the file to parse
/// 
/// # Returns
/// * `Result<TreeNode>` - Parsed tree structure or error
```

---

## 🚀 Release Process for Contributors

When contributing changes to GnawTreeWriter, follow these steps to ensure proper version management and releases.

### For Regular Commits (No Version Change)

Use this checklist for bug fixes, documentation updates, or minor changes that don't warrant a version bump:

```bash
# 1. Make your changes using GnawTreeWriter
gnawtreewriter edit src/file.rs "path" 'content'

# 2. Run tests
cargo test

# 3. Check for warnings
cargo clippy

# 4. Build release version
cargo build --release

# 5. Stage and commit
git add .
git commit -m "type(scope): description"

# 6. Push to GitHub
git push origin master
```

**Commit message format**: Follow [Conventional Commits](https://www.conventionalcommits.org/)
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only
- `chore:` - Maintenance tasks
- `test:` - Adding tests
- `refactor:` - Code refactoring

### For Version Releases (Patch, Minor, Major)

Use this checklist when you're ready to create a new release:

#### Step 1: Determine Version Number

Follow [Semantic Versioning](https://semver.org/):
- **Patch** (0.3.2 → 0.3.3): Bug fixes, documentation improvements
- **Minor** (0.3.3 → 0.4.0): New features, backward compatible
- **Major** (0.4.0 → 1.0.0): Breaking changes

#### Step 2: Update Version Files

```bash
# 1. Update Cargo.toml
gnawtreewriter edit Cargo.toml "version" '"0.3.4"'

# 2. Update CHANGELOG.md (add new section at top)
gnawtreewriter insert CHANGELOG.md "0" '
## [0.3.4] - 2025-01-02

### Added
- Feature description

### Changed
- Change description

### Fixed
- Fix description
'

# 3. Mark previous "Unreleased" as released (if any)
# Use gnawtreewriter edit to remove "(Unreleased)" tags
```

#### Step 3: Build and Test

```bash
# Clean build
cargo clean
cargo build --release

# Run all tests
cargo test

# Verify version
./target/release/gnawtreewriter --version
# Should show: gnawtreewriter 0.3.4
```

#### Step 4: Create Release Notes

```bash
# Create release notes file
cat > RELEASE_NOTES_v0.3.4.md << 'EOF'
# GnawTreeWriter — Release Notes (v0.3.4)

**Date:** 2025-01-02
**Type:** Patch/Minor/Major Release

## Summary
Brief description of what this release includes.

## Changes
- List key changes
- New features
- Bug fixes

## Upgrade Instructions
How to upgrade from previous version.

EOF
```

#### Step 5: Commit Version Changes

```bash
# Stage all version-related files
git add Cargo.toml Cargo.lock CHANGELOG.md RELEASE_NOTES_v0.3.4.md

# Commit with version bump message
git commit -m "chore(release): bump version to 0.3.4"

# Push to GitHub
git push origin master
```

#### Step 6: Create Git Tag

```bash
# Create annotated tag
git tag -a v0.3.4 -m "Release v0.3.4: Brief description"

# Push tag to GitHub
git push origin v0.3.4
```

#### Step 7: Create GitHub Release

```bash
# Using GitHub CLI (recommended)
gh release create v0.3.4 \
  --title "v0.3.4 - Release Title" \
  --notes-file RELEASE_NOTES_v0.3.4.md

# Verify release was created
gh release list
# Should show v0.3.4 as Latest
```

**Manual alternative**: Go to https://github.com/Tuulikk/GnawTreeWriter/releases/new

### Quick Release Checklist

Use this as a quick reference:

- [ ] Determine version number (semver)
- [ ] Update `Cargo.toml` version
- [ ] Update `CHANGELOG.md` (lib-yteändringar i enum/struct-fält, nya obligatoriska parametrar = `### BREAKING (lib)`-sektion INNAN release — Motor2 m.fl. path-dep:ar och litar på att brytningar ropas ut)
- [ ] Create `RELEASE_NOTES_vX.Y.Z.md`
- [ ] Run `cargo clean && cargo build --release`
- [ ] Run `cargo test` (all pass)
- [ ] Verify `--version` output
- [ ] Commit: `chore(release): bump version to X.Y.Z`
- [ ] Push to master
- [ ] Create git tag: `git tag -a vX.Y.Z -m "message"`
- [ ] Push tag: `git push origin vX.Y.Z`
- [ ] Create GitHub Release with notes
- [ ] Verify GitHub shows new version as "Latest"

### Common Pitfalls

**❌ Don't:**
- Skip version bumps in Cargo.toml
- Forget to update CHANGELOG.md
- Create releases without tags
- Push code without running tests
- Use manual version strings in code (use Cargo.toml)

**✅ Do:**
- Always test before releasing
- Keep CHANGELOG.md up to date
- Use semantic versioning
- Create annotated tags (`-a` flag)
- Write clear release notes
- Verify GitHub Release shows as "Latest"

### Version Synchronization

Ensure these are always in sync:
1. `Cargo.toml` → `version = "X.Y.Z"`
2. `CHANGELOG.md` → `## [X.Y.Z] - DATE`
3. Git tag → `vX.Y.Z`
4. GitHub Release → `vX.Y.Z` marked as Latest
5. CLI output → `gnawtreewriter --version` shows X.Y.Z

---

## 📚 Additional Resources

- **[README.md](README.md)** - Project overview and usage
- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Technical architecture details
- **[MULTI_AGENT_DEVELOPMENT.md](docs/MULTI_AGENT_DEVELOPMENT.md)** - AI agent collaboration patterns
- **[LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md)** - How LLMs integrate with GnawTreeWriter
- **[TESTING.md](docs/TESTING.md)** - Testing strategies and examples

---

## 💡 Success Stories

### Real Contributions from AI Agents

- **Gemini**: Designed the session management architecture
- **Claude**: Improved error handling and added comprehensive tests
- **GLM-4.7**: Implemented multiple parser engines and CLI commands
- **Raptor Mini**: Provided critical UX feedback that improved the quick-replace workflow

These contributions demonstrate that AI agents, when used appropriately and following best practices, can make meaningful contributions to complex software projects.

---

## 🧠 The Gnaw Mental Model: Avoiding Common AI Pitfalls

As an AI agent, your standard "mental model" for editing files often involves overwriting the whole file or using `sed`-like string replacement. In the GnawTreeWriter ecosystem, this can lead to disastrous (and expensive) results. 

### 🛡️ The TCARV Safety Net
To avoid panic and project corruption, always follow the **TCARV 1.0 (Text-Centric Architecture & Recursive Verification)** methodology:

1.  **Text First (Hypothesis):** Never edit code without a clear logic description in text first.
2.  **Surgical Edits:** Use `gnawtreewriter` to change ONLY the relevant nodes.
3.  **Anti-Panic Protocol:**
    *   **Got an error?** STOP. Do not "shotgun debug".
    *   **Check the path:** Use `gnawtreewriter list` to verify the node structure hasn't changed.
    *   **Undo is your friend:** If the build fails, run `gnawtreewriter undo` immediately and rethink the logic.
    *   **Agency with Responsibility:** You have the mandate to fix things, but you must backup complex code before removing it.

### 1. The Double-Brace Trap (Rust & JSON Macros)
Many AI agents (and their tool-calling layers) automatically "escape" curly braces when generating shell commands.
- **Problem**: You try to inject `json!({"a": 1})`, but the shell call becomes `gnawtreewriter edit ... 'json!({{ "a": 1 }})'`.
- **Result**: Broken Rust code that won't compile.
- **Solution**: **NEVER** pass complex code snippets as direct command-line arguments. Use **STDIN** or **Source Files**.
  ```bash
  # INCORRECT (Risk of double-braces via shell)
  gnawtreewriter edit app.rs "0.1" 'json!({"a":1})' 
  
  # CORRECT (Safe from shell escaping)
  echo 'json!({"a":1})' | gnawtreewriter edit app.rs "0.1" -
  ```

### 2. Don't Fight the Shell (Use `-` for STDIN)
Standard `run_shell_command` utilities often use `bash -c`. Bash will try to interpret your quotes, dollars, and braces. 
- **Rule**: If your code snippet is longer than 20 characters or contains any special symbols, pipe it into `gnawtreewriter` using the `-` (STDIN) argument. This bypasses Bash's parsing logic entirely.

### 3. Operating in Sandboxes (Flatpak/Docker)
If you find yourself running inside a sandboxed editor (like Zed via Flatpak), you might lose access to the host's `gnawtreewriter` binary.
- **Problem**: `gnawtreewriter: command not found` even though it's installed.
- **Solution**: Use `flatpak-spawn --host`.
  ```bash
  flatpak-spawn --host gnawtreewriter analyze main.rs
  ```
- **Context**: `flatpak-spawn` is a bridge that allows a sandboxed application to execute commands on the host system.

### 4. Think in Nodes, Not Lines
Stop looking for "line 45". Lines change constantly. 
- **Best Practice**: Use `gnawtreewriter list` or `gnawtreewriter search_nodes` to find the **Path** (e.g., `1.3.12`) of the function you want to change. Targeting a Node Path is 100% stable regardless of how many lines are added above it.

### 5. Validation is Mandatory
Don't assume your edit is correct. GnawTreeWriter is a surgical tool; a single missing brace in a node edit can break the whole file's AST.
- **Rule**: Always run `cargo check` or `gnawtreewriter analyze <file>` after an edit to ensure the file is still valid. If it fails, use `gnawtreewriter undo` immediately.

---

## 🎓 Getting Help

- **GitHub Issues**: https://github.com/Tuulikk/GnawTreeWriter/issues
- **Discussions**: GitHub Discussions tab
- **Documentation**: See docs/ directory
- **Examples**: See examples/ directory

---

**Remember**: The best contributions come from real usage. Use GnawTreeWriter to build GnawTreeWriter—this dogfooding practice makes the tool better for everyone! 🐶✨

---

## AI-Friendly Features

For complete documentation on AI-optimized features, see [docs/AI_FRIENDLY.md](docs/AI_FRIENDLY.md).

### Quick Commands
```bash
# Token-aware analysis
gnawtreewriter analyze src/main.rs --format summary

# Code compression (~70% reduction)
gnawtreewriter compress src/main.rs --stats

# Project packing for AI context
gnawtreewriter pack . --compress --format markdown

# Intelligent file curation
gnawtreewriter curate "authentication" --max-tokens 5000
```

### MCP Tools
All features available as MCP tools: `compress`, `pack`, `curate`

---

*Version 2.0 - Rewritten to focus on contributing and dogfooding rather than end-user usage*

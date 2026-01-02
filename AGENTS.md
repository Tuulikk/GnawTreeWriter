Ja# Contributing to GnawTreeWriter as an AI Agent

**Guide for AI agents contributing to the GnawTreeWriter project through dogfooding**

Version: 2.0 | Last Updated: 2025-12-27

---

## 🎯 Purpose

This document explains how AI agents can contribute to the GnawTreeWriter project by using the tool to develop itself—a practice known as "dogfooding." By using GnawTreeWriter to edit its own codebase, AI agents help validate functionality, discover edge cases, and improve the tool's design.

---

## 🏗️ Project Overview

GnawTreeWriter is a tree-based code editor written in Rust that works at the AST (Abstract Syntax Tree) level. It uses TreeSitter parsers to support multiple programming languages.

### Tech Stack
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

**Scenario**: Add a `validate` command that checks file syntax

```bash
# Step 1: Analyze the CLI structure
gnawtreewriter analyze src/cli.rs

# Step 2: Find the Commands enum
gnawtreewriter list src/cli.rs --filter-type enum_item

# Step 3: Add new command variant (preview first)
gnawtreewriter fuzzy-edit src/cli.rs "enum Commands" 'Validate { file: PathBuf }' --preview

# Step 4: Apply the edit
gnawtreewriter fuzzy-edit src/cli.rs "enum Commands" 'Validate { file: PathBuf }'

# Step 5: Add command handler in main.rs
gnawtreewriter list src/main.rs --filter-type match_arm
gnawtreewriter fuzzy-edit src/main.rs "Commands::Edit" 'Commands::Validate { file } => {
    println!("Validating: {:?}", file);
},' --preview
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
gnawtreewriter fuzzy-edit Cargo.toml "tree-sitter-bash" 'tree-sitter-java = "0.23"' --preview

# Step 3: Update parser/mod.rs to register the parser
gnawtreewriter fuzzy-edit src/parser/mod.rs '"go" => Ok' '"java" => Ok(Box::new(java::JavaParser::new())),' --preview
```

#### Adding Tests

**Scenario**: Add a unit test for a function

```bash
# Step 1: Find the function in src/core/
gnawtreewriter list src/core/mod.rs --filter-type function_definition

# Step 2: Add test module
gnawtreewriter fuzzy-edit src/core/mod.rs "#[cfg(test)]" '
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
gnawtreewriter find README.md --content "## CLI Commands"

# Step 3: Add documentation for the new command
gnawtreewriter fuzzy-edit README.md "## CLI Commands" '
### validate
Check file syntax without making changes.

```bash
gnawtreewriter validate <file_path>
```' --preview
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
# Node-edit mode (AST-based)
gnawtreewriter quick app.py --node "0.1.0" --content "def new_function():" --preview
gnawtreewriter quick app.py --node "0.1.0" --content "def new_function():"

# Find/replace mode (text-based)
gnawtreewriter quick app.py --find "old_function" --replace "new_function" --preview
gnawtreewriter quick app.py --find "old_function" --replace "new_function"
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
gnawtreewriter find src/cli.rs --content "Commands::Find"

# Step 2: Add fuzzy command to enum
gnawtreewriter fuzzy-edit src/cli.rs "Find" 'Fuzzy { file: PathBuf, query: String }' --preview

# Step 3: Implement fuzzy matching in core
# Create src/core/fuzzy.rs with fuzzy search logic
# Then add handler in main.rs
```

### Example 2: Improving Error Messages

**Goal**: Make error messages more actionable

```bash
# Step 1: Find error handling code
gnawtreewriter list src/core/mod.rs --filter-type function_definition
gnawtreewriter find src/core/mod.rs --content "map_err"

# Step 2: Improve error message
gnawtreewriter fuzzy-edit src/core/mod.rs "map_err" '.map_err(|e| {
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
- [ ] Update `CHANGELOG.md`
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
- **Raptor Mini**: Provided critical UX feedback that improved the fuzzy-edit workflow

These contributions demonstrate that AI agents, when used appropriately and following best practices, can make meaningful contributions to complex software projects.

---

## 🎓 Getting Help

- **GitHub Issues**: https://github.com/Tuulikk/GnawTreeWriter/issues
- **Discussions**: GitHub Discussions tab
- **Documentation**: See docs/ directory
- **Examples**: See examples/ directory

---

**Remember**: The best contributions come from real usage. Use GnawTreeWriter to build GnawTreeWriter—this dogfooding practice makes the tool better for everyone! 🐶✨

---

*Version 2.0 - Rewritten to focus on contributing and dogfooding rather than end-user usage*

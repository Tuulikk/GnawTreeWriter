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
- TypeScript (`.ts`, `.tsx`)
- JavaScript (`.js`, `.jsx`)
- PHP (`.php`)
- HTML (`.html`)
- QML (`.qml`)
- Go (`.go`)
- CSS (`.css`)
- YAML (`.yaml`, `.yml`)
- TOML (`.toml`)
- XML (`.xml`)

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

**Scenario**: Add support for a new language (e.g., C++)

```bash
# Step 1: Create new parser file
cat > src/parser/cpp.rs << 'EOF'
use crate::parser::ParserEngine;
use anyhow::Result;
use tree_sitter::{Parser, Language};

pub struct CppParser;

impl ParserEngine for CppParser {
    fn parse(&self, code: &str) -> Result<crate::core::TreeNode> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_cpp::language())?;
        // Implementation details...
    }
    
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["cpp", "cc", "cxx", "h", "hpp"]
    }
}
EOF

# Step 2: Update Cargo.toml to add dependency
gnawtreewriter fuzzy-edit Cargo.toml "tree-sitter" 'tree-sitter-cpp = "0.22"' --preview

# Step 3: Update main.rs to register the parser
gnawtreewriter list src/main.rs --filter-type function_definition
gnawtreewriter fuzzy-edit src/main.rs "register_parsers" 'parsers.push(Box::new(parser::cpp::CppParser));' --preview
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

---

## 🤝 Contribution Areas

AI agents can contribute to several areas of the project:

### 1. Parser Development
- Add support for new languages (C++, Java, Kotlin, etc.)
- Improve existing parsers (better AST representation, edge cases)
- Optimize parser performance

### 2. Core Functionality
- Implement new edit operations (move, rename, refactor)
- Improve tree navigation and path resolution
- Enhance validation and error reporting

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

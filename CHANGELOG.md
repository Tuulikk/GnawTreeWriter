# Changelog

All notable changes to GnawTreeWriter.

## Unreleased

### Added
- **MCP Documentation**: Added comprehensive MCP server documentation in `docs/MCP.md`
  - Detailed protocol documentation, supported methods, authentication, and examples
  - Guide for adding new tools and testing the server
- **MCP Example Clients**: Added Python and Rust example clients for interacting with MCP server
  - `examples/python_mcp_client.py`: Full-featured Python CLI with `init`, `list`, `analyze`, and `call` commands
  - `examples/mcp_client.rs`: Rust example using `reqwest` and `tokio`
  - Both clients demonstrate JSON-RPC 2.0 request/response handling and error checking
- **MCP Integration Tests**: Added comprehensive integration tests for MCP server
  - Test for `tools/list` to verify tool discovery
  - Test for `tools/call` with missing arguments (error handling)
  - Test for authentication flow (unauthorized vs authorized)
  - Test for `tools/call` analyze with temporary file
- **MCP CI Testing**: Added GitHub Actions workflow to test MCP examples on CI
  - `.github/workflows/mcp-examples.yml`: Builds with MCP feature and runs example clients
  - Starts MCP server in background, tests both Python and Rust clients
- **MCP CLI Help**: Improved CLI help text for MCP commands
  - Enhanced documentation for `gnawtreewriter mcp serve` with examples and token usage

### Fixed
- **MCP Test Compilation**: Fixed MCP test compilation errors
  - Corrected imports (`axum::http::Request`, `tower::util::ServiceExt`)
  - Added `tower = "0.5"` to dev-dependencies
  - Fixed unused variable warnings and dead code warnings
- **MCP Clippy Errors**: Resolved clippy warnings in MCP and related code
  - Made `find_symbols_in_tree` and `find_symbols_in_directory` static methods to avoid recursion warnings
  - Fixed needless borrows in `TransactionLog::load` calls (use `.clone()` where needed)
  - Removed unused imports in test files (conditionally imported with `#[cfg(feature)]`)
  - Added `#[allow(dead_code)]` for unused `jsonrpc` field in `JsonRpcRequest`

### Changed
- **MCP README**: Updated `README.md` with MCP server documentation
  - Added usage examples and link to detailed `docs/MCP.md`
  - Documented token usage and environment variable (`MCP_TOKEN`)
- **Cleanup**: Removed temporary and untracked files
  - Removed `.gnawtreewriter_session.json` from version control (gitignored)
  - Removed temporary files: `src/cli.rs.tmp`, `src/llm/ai_manager_minimal.rs`

## [0.6.0] - 2025-01-05

### Fixed
- **CI/CD Robustness**: Fixed GitHub Actions build failures for ModernBERT AI features
  - Added `#[allow(unused_variables, unreachable_code)]` attributes to handle cfg-conditional compilation
  - Fixed parameter naming consistency in AI functions (`force`, `file_path`, `node_path`, etc.)
  - All builds now pass with `-D warnings` flag (warnings treated as errors)
- **Conditional Compilation**: Improved handling of `modernbert` feature flag
  - Functions now compile cleanly with or without the feature enabled
  - Proper `#[allow(unused_mut)]` for `get_status()` function

### Changed
- Improved code quality in `src/llm/ai_manager.rs`
- All changes made using GnawTreeWriter dogfooding (eating our own dog food!)

---

## [0.5.0] - 2025-01-06

### Added
- **ModernBERT AI Integration**: Local, privacy-focused AI features using Candle
  - **Semantic Search**: Find code by meaning, not just text, with `gnawtreewriter find --semantic`
  - **AI Refactoring**: Identify complex code and get refactoring suggestions with `gnawtreewriter ai refactor`
  - **Context-Aware Completion**: AST-based code completion using `gnawtreewriter ai complete`
  - **Local Inference**: Runs entirely on your machine using Rust-native Candle (CPU/CUDA/Metal)
- **Clone Operation**: Duplicate code nodes/structures within or between files
  - Clone functions, classes, or any AST node with `gnawtreewriter clone`
  - Preview mode shows diff before applying
  - Supports same-file or cross-file cloning
  - Perfect for creating similar components or duplicating boilerplate
- **Zig Language Support**: Full TreeSitter-based parser for Zig (`.zig`)
  - Support for functions, structs, tests, and all Zig constructs
  - Added example file: `examples/hello.zig`

### Examples
```bash
# Semantic search for login logic
gnawtreewriter find --semantic "authentication and login"

# Get AI refactoring suggestions for a file
gnawtreewriter ai refactor src/main.rs

# Clone a function within same file
gnawtreewriter clone app.py "0.1" app.py "0.2" --preview

# Clone between files
gnawtreewriter clone src.rs "1.0" dest.rs "2.0"

# Analyze Zig code
gnawtreewriter list examples/hello.zig --filter-type function_declaration
```

### Changed
- Updated README with Zig language support
- Updated AGENTS.md with Clone operation examples

## [0.4.0] - 2025-01-03

### Added
- **Java Language Support**: Full TreeSitter-based parser for Java (`.java`)
  - AST parsing for classes, methods, interfaces, and all Java constructs
  - Added example file: `examples/HelloWorld.java`
- **Refactor/Rename Command**: AST-aware symbol renaming across files
  - Rename functions, variables, classes with confidence
  - Distinguishes declarations from usages
  - Perfect for large refactorings where search-and-replace would be dangerous
  - Recursively search directories with `--recursive` flag
  - Preview mode with `--preview` flag
  - Validates new symbol names against language-specific reserved keywords
- **Dry-run Support**: Global `--dry-run` flag for all write operations
  - Preview what would happen without making changes
  - Increases safety across all commands

### Changed
- Updated README with Java and Refactor/Rename documentation
- Updated AGENTS.md with new refactoring examples

### Examples
```bash
# Rename a function in a single file
gnawtreewriter rename myFunction newFunction app.py

# Preview renaming a method across a Java file
gnawtreewriter rename greet sayHello examples/HelloWorld.java --preview

# Rename a class recursively across entire project
gnawtreewriter rename MyClass NewClass src/ --recursive
```

### Fixed
- Fixed QuickReplace test parameter handling to include all 5 parameters

## [0.3.4] - 2025-01-03

### Added
- **C Language Support**: Full TreeSitter-based parser for C (`.c`, `.h`)
- **C++ Language Support**: Full TreeSitter-based parser for C++ (`.cpp`, `.hpp`, `.cc`, `.cxx`, `.hxx`, `.h++`)
- **Bash Language Support**: Full TreeSitter-based parser for Bash (`.sh`, `.bash`)
- **Java Language Support**: Full TreeSitter-based parser for Java (`.java`)
- **Example Files**: Added example files for C, C++, and Bash in `examples/` directory

### Changed
- **TreeSitter Upgrade**: Upgraded core `tree-sitter` from 0.24 to 0.26.3 (latest)
- **Parser Updates**: Updated all language parser crates to latest compatible versions:
  - `tree-sitter-python`: 0.25.0
  - `tree-sitter-rust`: 0.24.0
  - `tree-sitter-go`: 0.25.0
  - `tree-sitter-php`: 0.24.2
  - `tree-sitter-bash`: 0.25.1
  - `tree-sitter-c`: 0.24.1
  - `tree-sitter-cpp`: 0.23.4
  - `tree-sitter-html`: 0.23.2
  - `tree-sitter-typescript`: 0.23.2
  - `tree-sitter-javascript`: 0.25.0
- **Parser API**: Updated all parser implementations to use tree-sitter 0.26 API with LanguageFn
- **Documentation**: Updated README with new language support (C, C++, Bash)

### Fixed
- **Clippy Warnings**: Fixed all 4 remaining clippy warnings:
  - Removed duplicated `#[cfg(test)]` attribute in undo_redo.rs
  - Removed needless borrows in cli.rs TransactionLog::load calls
  - Replaced `find().is_some()` with `any()` in xml.rs for cleaner iterator usage

## [0.3.3] - 2025-01-02

### Changed
- **Documentation**: Added Installation section at top of README for better GitHub visibility

## [0.3.2] - 2025-12-31

### Added
- **Diff-to-Batch**: New `gnawtreewriter diff-to-batch` command that converts unified diffs (git diff format) to batch operation specifications
- **Test Coverage**: Added 5 new tests for diff parser functionality

### Changed
- **Code Quality**: Reduced clippy warnings from 46 to 28 through systematic cleanup
- **Parser Improvements**: Fixed string slicing in multiple parsers (go, html, php, python, rust, typescript) using `source.get()` instead of `as_bytes()` + `from_utf8()`
- **Module Structure**: Renamed `src/llm/llm.rs` to `src/llm/llm_integration.rs` to resolve module inception warning
- **Backup Directory Renaming**: Renamed unused `project_root` field to `_project_root` in `RestorationEngine`
- **Batch Module**: Added Serialize trait support to Batch and BatchOp for JSON output
- **BatchEdit Export**: Made BatchEdit enum publicly available in core module for diff-to-batch integration

### Fixed
- **Clippy Warnings**: 
  - Removed unused imports (Context) from parser module
  - Removed dead code (get_backup_dir, unused add function)
  - Fixed manual_strip warnings using strip_prefix
  - Fixed wildcard_in_or_patterns warnings
  - Fixed print_literal warnings in CLI output
  - Removed useless conversions (.into())
  - Fixed if_same_then_else warnings
  - Fixed collapsible_if warnings
  - Replaced map_or(false, ...) with is_some_and(...)
  - Removed empty line after doc comment
  - Fixed unnecessary map of identity function
  - Fixed needless_borrow warnings in LLM module
  - Removed unnecessary Ok(?) wrapping in multiple parsers
  - Fixed manual suffix stripping using strip_suffix in QML parser
- **CLI Quick Command**: Fixed handle_quick_replace function placement and integration
- **Test Failures**: All 32 tests now passing after fixing test environment setup (27 core + 5 diff-parser)

## [0.3.1] - 2025-12-31

### Summary
This release includes comprehensive code quality improvements with clippy warnings reduced from 46 to 28.

## [0.3.0] - 2025-12-28

### Added
- **Implicit Sessions**: Sessions auto-start on first edit and session IDs persist across commands via `.gnawtreewriter_session_id`, improving UX for ad-hoc workflows and agent sessions.
- **Built-in Diff View**: `gnawtreewriter diff` implemented to show precise changes per transaction (by ID or `--last N`), using the backup system and content-hash matching.
- **Generic Node Support**: `GenericParser` added to treat unknown file types (README, Dockerfile, .config, etc.) as a single node so they can be analyzed, backed up, and edited as text blobs.
- **Named References (Tags)**: `tag add/list/remove` implemented; `edit/insert/delete` accept `tag:<name>` shorthand to reference named node paths (enables robust, readable scripting).
- **Batch Operations**: `gnawtreewriter batch` command for atomic multi-file edits from JSON specification. Features in-memory validation, unified diff preview, automatic rollback on failure, and transaction logging per file. Perfect for AI agent workflows and coordinated refactoring.

### Changed
- Documentation updates across README, AGENTS.md and ROADMAP.md to reflect the new features and add-on strategy.
- Roadmap current status updated to v0.3.0 (2025-12-28).

### Fixed
- Minor reliability and documentation fixes related to the new features.

## [0.2.2] - 2025-12-27

### Added
- **Safe Input Methods**: New `--source-file` and stdin support (`-`) for `edit` and `insert` commands to handle complex code without shell escaping issues.
- **Smart Timestamp Parsing**: Restoration commands now accept naive timestamps (e.g., "2025-12-27 20:30:00") and intelligently assume Local time, converting to UTC automatically.
- **Persistent Undo**: The `undo` stack is now restored from the transaction log between sessions, allowing undo across restarts.
- **Context Awareness**: Commands now automatically locate the project root (via `.git` or session file) regardless of the current working directory, fixing "No changes found" errors when running from subdirectories.
- **Unescape Flag**: New `--unescape-newlines` flag for manual string unescaping if needed.
- **CSS Support**: Full CSS parsing with custom parser for rules, selectors, declarations, at-rules (@media, @keyframes), and nested structures.
- **YAML Support**: Complete YAML parsing using `serde_yaml` library for mappings, sequences, scalars, and tagged values.
- **XML Support**: Added a robust XML parser implemented with the `xmltree` crate. Supports XML declaration, DOCTYPE, comments, CDATA, attributes, nested elements and converts the result into the project's `TreeNode` model. Includes unit tests for common cases.
- **Markdown Support**: Full CommonMark-compliant Markdown parsing with support for headings (h1-h6), paragraphs, lists (ordered/unordered), code blocks with language specification, block quotes, horizontal rules, and inline formatting (bold, italic, code, links).
- **Text Support**: Simple line-based parsing for plain text files (.txt) with line-by-line node structure for easy editing of individual lines.

### Changed
- Updated `restore-project` and `restore-files` to use the new smart timestamp parser.
- Improved CLI help text for restoration commands to clarify time format support.

---

## [0.2.1] - 2025-12-27

### Added
- **Revolutionary Help System**: Interactive `examples` and `wizard` commands for guided learning
- **AI Agent Testing Framework**: Comprehensive test scenarios (AI_AGENT_TEST_SCENARIOS.md) with structured evaluation
- **Multi-File Time Restoration**: Complete project-wide time travel with `restore-project`, `restore-files`, `restore-session`
- **Transaction Logging**: Full audit trail of all operations with session management
- **Version Flag**: `--version` command to check current version
- **Lint Command**: `lint` command for basic file validation and issue detection
- **Directory Analysis**: `--recursive` flag for analyzing entire directories
- **Go Support**: Full TreeSitter-based parsing support for Go (`.go`)
- **Enhanced Preview**: `--preview` now shows a proper unified diff (using `similar` crate) instead of just the whole file
- **QML add-component**: New command to safely inject child components into QML objects
- **Core API**: Added `get_source()` to `GnawTreeWriter` for easier integration

### Changed
- **Documentation Overhaul**: Complete README and ROADMAP updates reflecting current capabilities
- **Error Handling**: Better error messages for directory analysis and invalid paths
- Improved CLI `preview` flags across all edit/insert/delete operations

### Fixed
- **Directory Analysis Bug**: Fixed "Is a directory" error with proper `--recursive` flag requirement
- **Documentation Inconsistencies**: Aligned all documentation with actual CLI behavior
- **Missing Commands**: Added previously documented but missing `lint` and `--version` commands

---

## [0.2.0] - 2025-12-26

### Added
- **TreeSitter QML Parser**: Replaced custom regex parser with a robust TreeSitter-based parser for QML.
- **Syntax Validation**: Automatic in-memory syntax validation before saving any edits. Prevents file corruption.
- **Smart Indentation**: `insert` command now automatically detects and applies parent/sibling indentation to new content.
- **Dedicated QML Add-Property**: New `add-property` command specifically optimized for QML AST structure.
- **Automatic FFI Linking**: Resolved version mismatch issues with `tree-sitter-qmljs` using dynamic language loading.

### Changed
- Improved `insert` logic to handle container braces correctly (e.g., inserting after `{`).
- Standardized node paths across all supported languages.
- Updated documentation with new command examples and technical details.

### Fixed
- Fixed a bug where nested braces in macros (like `serde_json::json!`) could cause code corruption during edits.
- Improved CLI stability and error reporting for missing nodes.

---

## [0.1.1] - 2025-12-26

### Added
- **Multi-file operations**: analyze, lint, find now support wildcards and directories
- **Automatic backup system**: timestamped JSON backups before every edit
- **Human-friendly lint mode**: `file:line:col severity message` format with JSON output
- **Fuzzy-edit command**: LLM-friendly editing with multi-strategy matching
- **QML add-property command**: Safe property injection for QML components
- **Diff preview**: `--preview` flag shows unified diff for all edit operations
- **List command**: Show all nodes with paths in a file
- **Smart selectors**: Find nodes by type and content

### Changed
- Updated README with comprehensive command documentation
- Added links to RECIPES.md and QML_EXAMPLES.md
- Added .gitignore for .gnawtreewriter_backups/ directory

### Fixed
- CLI stability issues (--help now works reliably)
- Multi-file analyze support (respects wildcards)
- Exit codes for lint errors

---

## [0.1.0] - 2025-12-26

### Initial Release

- Basic tree-based code editor for LLM-assisted editing
- Multi-language support: Python, Rust, TypeScript, PHP, HTML, QML
- Foundation for tree-level code manipulation
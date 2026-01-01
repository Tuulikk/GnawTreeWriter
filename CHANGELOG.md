# Changelog

All notable changes to GnawTreeWriter.

## [0.3.1] - 2025-12-31 (Unreleased)

### Added
- **Test Robustness**: Added mutex-based CWD protection to CLI tests to prevent race conditions when changing directories
- **Test Environment**: Created `.git` directory mock in test temporary directories for proper project root detection

### Changed
- **Code Quality**: Reduced clippy warnings from 46 to 23 through systematic cleanup
- **Parser Improvements**: Fixed string slicing in multiple parsers (go, html, php, python, rust, typescript) using `source.get()` instead of `as_bytes()` + `from_utf8()`
- **Module Structure**: Renamed `src/llm/llm.rs` to `src/llm/llm_integration.rs` to resolve module inception warning
- **Backup Directory Renaming**: Renamed unused `project_root` field to `_project_root` in `RestorationEngine`

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
- **Test Failures**: All 27 tests now passing after fixing test environment setup

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
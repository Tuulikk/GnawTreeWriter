# Changelog

All notable changes to GnawTreeWriter.

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
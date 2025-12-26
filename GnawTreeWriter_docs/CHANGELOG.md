# Changelog

All notable changes to GnawTreeWriter.

## [0.1.1] - 2025-12-26

### Added
- **Multi-file operations**: analyze, lint, find now support wildcards and directories
- **Automatic backup system**: timestamped JSON backups before every edit
- **Human-friendly lint mode**: `file:line:col severity message` format with JSON output
- **Fuzzy-edit command**: LLM-friendly editing with multi-strategy matching
  - Content substring matching
  - Node type matching
  - Word matching
  - Prefix matching
  - Levenshtein similarity
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
- Multi-language support:
  - Python (TreeSitter)
  - Rust (TreeSitter)
  - TypeScript/TSX (TreeSitter)
  - PHP (TreeSitter)
  - HTML (TreeSitter)
  - QML (Custom parser)
- CLI commands:
  - analyze: Parse files and show AST structure
  - show: Display specific node content
  - edit: Replace node content
  - insert: Add new content
  - delete: Remove nodes
- Foundation for tree-level code manipulation

# Changelog

All notable changes to GnawTreeWriter.

## [0.9.1] - 2026-02-04

### Added
- **Surgical Inline Editing**: Character-level precision for code edits. You can now edit specific nodes (like parameters or variable names) within a single line without affecting surrounding code.
- **Pedagogical Syntax Tips**: The editor now provides language-specific advice when an edit fails syntax validation (Rust, QML, Python).
- **Column-Aware TreeNode**: Upgraded `TreeNode` structure and the Rust parser to track and utilize character offsets for enhanced precision.

### Changed
- **Enhanced Documentation**: Updated `README.md`, `examples`, and the interactive `wizard` to reflect the new surgical precision capabilities.
- **Version Bump**: Major refinement release marking the transition to v0.9.1 "The Surgical Update".

### Fixed
- **Precision Failures**: Resolved issues where inline edits would inadvertently delete parts of the line.
- **CLI Robustness**: Improved error reporting for JSON and cross-file operations.

## [0.9.0] - 2026-01-31

### Added
- **Slint Support**: Full AST-based editing and analysis for `.slint` files. Powered by `tree-sitter-slint`.
- **AI Default**: The `modernbert` (GnawSense) and `mcp` features are now enabled by default. No more `--features` flags needed for standard usage.
- **Enhanced Status**: The `status` command now proudly displays the state of **GnawSense**, **HRM2** (Hierarchical Reasoning), and **Undo/Redo** history.
- **GnawTree Architect Skill**: A specialized agent skill (`gnawtree-architect`) to guide AI agents in surgical code editing.

### Fixed
- **Safety Nets**: Implemented node count limits (500-1000 nodes) and depth limits in `list`, `skeleton`, and MCP tools to prevent agent context crashes.
- **Memory Optimization**: Refactored `list_nodes` to avoid cloning entire subtrees, significantly reducing memory usage on large files.
- **CLI Hygiene**: Removed duplicate `Status` command handlers and cleaned up unused imports in core modules.
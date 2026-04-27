## [0.9.3] - 2026-04-27

### Added
- **GnawSense Semantic Navigation** (AI-powered code search & insertion):
  - `sense` command: search code by meaning using local ModernBERT model
  - `sense-insert` command: insert code at semantically located anchors
  - All 4 intents supported: `after`, `before`, `inside`, `replace`
  - `--auto-index` flag: skip interactive prompt for AI agents/CI
  - `GNAW_JSON=1` environment variable for machine-readable output
  - Confidence threshold: filters < 0.2, warns < 0.5
  - Multi-language `extract_name_from_preview`: 15+ patterns (Rust, Python, Go, JS, Java, C, QML)
  - Standardized `err_modernbert_disabled()` helper with JSON support

### Fixed
- **quick-replace literal \n bug**: Auto-detects literal `\n`/`\t` in replacement text and converts to real newlines/tabs (prevented broken file writes from CLI escaping)
- **sense-insert position logic**: Fixed off-by-one in `get_next_index()` — was `idx+3+1`, now `idx+3`
- **Compiler warnings**: Eliminated all warnings (unused variable, dead code)

### Changed
- **Performance**: Model caching with `OnceLock<ModernBertModel>` — loads once, reuses across calls
- **Performance**: JIT file index cache with content-hash invalidation in `GnawSenseBroker`
- **SemanticIndex** now derives `Clone` for caching support

### Documentation
- GnawSense SKILL.md for AI agents at `~/.pi/agent/skills/gnaw-sense/`
- ROADMAP.md: detailed 5-tier GnawSense improvement plan with measured baselines
# Changelog

All notable changes to GnawTreeWriter.

## [0.9.2] - 2026-02-05

### Fixed
- **Critical Compilation Errors**: Fixed 56+ syntax errors in `src/cli.rs` caused by unescaped double quotes in help text examples. All println! statements containing nested quotes have been properly escaped.
- **Build System**: Restored compilation on Rust stable by fixing string literal syntax issues.

### Technical Details
- Problem: String literals like `println!("text "quote" more")` were interpreted as separate tokens
- Solution: Escaped all nested quotes as `println!("text \"quote\" more")`
- Affected: Help text in examples subcommand covering editing, search, restoration, and AI features
- Lines modified: ~80 println! statements across 30+ example categories

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
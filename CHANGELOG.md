# Changelog

All notable changes to GnawTreeWriter.

## [0.8.4] - 2026-01-27

### Added
- **Human-Readable Sessions**: Introduced session aliasing. Users can now name their sessions (e.g., `--name cleanup`) for much easier restoration and navigation.
- **Safe Content Injection (@file syntax)**: Added the ability to read edit/insert content from a local file path prefixed with `@`. This completely eliminates shell escaping and quoting issues for AI agents.
- **Improved Semantic Indexing**: Enhanced persistence and metadata sharing for the future **GnawMimir** integration.

### Fixed
- **CLI Robustness**: Standardized indentation and fixed minor logic bugs in cross-file command handling.
- **Updated Documentation**: Refreshed `KNOWN_ISSUES.md` to accurately reflect the v0.8.x series and addressed feedback from the GLM architectural analysis.

## [0.8.3] - 2026-01-27

### Added
- **Token Overload Protection**: Implemented mandatory limits on the `list` command to protect LLM context windows and prevent API krascher.
  - **Surgical Safety Valve**: Standard limit of 100 nodes per list operation.
  - **Pagination Support**: Added `--limit` and `--offset` flags to the `list` command for safe navigation of large files.
  - **Proactive Guidance**: Automatically suggests the correct `--offset` when data is truncated.

## [0.8.2] - 2026-01-27

### Added
- **The Helpful Guard**: Proactive help system that intercepts CLI errors and provides strategic advice and command tips.
- **Enhanced AI Examples**: Expanded `examples --topic ai` with practical workflows for ALF, GnawSense, and ReportEngine.
- **Auto-Indentation Fixes**: Cleaned up internal CLI code for better maintenance.

## [0.8.1] - 2026-01-27

### Added
- **ReportEngine (The Evidence Generator)**: New capability to generate structural engineering reports in Markdown format.
  - **Audit Integration**: Merges high-level intent from ALF with low-level structural changes from TransactionLog.
  - **Professional Reporting**: Designed to provide clear, evidence-based accounts of technical progress for GitHub or administrative reporting.
  - **New CLI Command**: `gnawtreewriter ai report` to generate and save structural evolution reports.

## [0.8.0] - 2026-01-23

### Added
- **The Guardian (Structural Integrity Protection)**: Revolutionary safety layer that protects your code from accidental destruction by AI agents.
  - **Integrity Auditing**: Automatically analyzes every edit for significant volume reduction or structural loss.
  - **Heuristic Complexity Check**: Detects if critical logic (if, loops, try-catch) is removed without valid replacement.
  - **Documentation Watchdog**: Warns if comments or documentation are stripped from a node.
  - **--force Override**: Allows human operators to bypass safety blocks when intentional major refactorings are performed.
- **Smart Project Indexing (v0.7.9 features included)**: Faster project-wide semantic search with hash-based skipping and large file support.

## [0.7.9] - 2026-01-23

### Added
- **Smart Re-indexing**: The project crawler now skips unchanged files by comparing content hashes, making subsequent indexing operations near-instant.
- **ModernBERT Chunking**: Automatic recursive chunking for large nodes, preventing sequence length errors and allowing indexing of massive source files.
- **Ecosystem Metadata**: Saved model info in `.gnawtreewriter_ai/index/model_info.json` to enable safe vector sharing with **GnawMimir**.

### Fixed
- **Path Canonicalization**: Resolved "prefix not found" errors during indexing by ensuring all paths are canonicalized before stripping project root.
- **Dependency Hygiene**: Standardized more tree-sitter dependency names for better build stability.

## [0.7.8] - 2026-01-23

### Added
- **Multi-Actor ALF**: Support for multiple tools collaborating on the same project.
  - **Actor Attribution**: ALF entries now track which tool performed the action (e.g., `@writer`, `@mimir`).
  - **Ecosystem Readiness**: GnawTreeWriter is now prepared to share its structural journal with the upcoming **GnawMimir** project.
  - **CLI Expansion**: New `--actor` flag for the `alf` command.

## [0.7.7] - 2026-01-23

### Added
- **Project-wide Semantic Search**: Revolutionary ability to search for code by meaning across the entire codebase.
  - **Project Crawler**: New `ai index` command that rekursivt genomsöker och indexerar projektet.
  - **Satelite View 2.0**: The `sense` command now performs global searches if no file is specified, return rankings from the entire project.
  - **Persistent Embedding Cache**: All embeddings are saved in `.gnawtreewriter_ai/index/` for instant project-wide intelligence.
- **Hygiene & Stability**: Achieve 100% clean build with no clippy warnings across all modules.

## [0.7.6] - 2026-01-23

### Added
- **Multi-Platform Expansion**: Native support for mobile development languages.
  - **Swift Support**: Full AST-based editing and semantic search for `.swift` files.
  - **Kotlin Support**: Full AST-based editing and semantic search for `.kt` and `.kts` files using `tree-sitter-kotlin-ng`.
- **Improved Language Hygiene**: Standardized dependency naming in `Cargo.toml` for better package discovery.

## [0.7.5] - 2026-01-23

### Added
- **ALF (Agentic Logging Framework)**: A structural journaling system for AI agents.
  - **Auto-Journaling**: Every `edit`, `insert`, and `scaffold` operation is now automatically logged to `.gnawtreewriter_ai/alf.json`.
  - **Transaction Linking**: ALF entries automatically link to their corresponding Transaction IDs for perfect traceability.
  - **Structural Intent**: New `alf` command to log high-level intents, assumptions, risks, and outcomes.
  - **Retrospective Tagging**: Ability to go back and tag or enrich previous journal entries.
- **Enhanced Agent Memory**: Agenter kan nu "minnas" varför de fattade vissa arkitektoniska beslut genom att läsa ALF-journalen.

## [0.7.4] - 2026-01-23

### Added
- **The Duplex Loop (Foundation)**: Built the core infrastructure for self-healing code edits.
  - **Structured Syntax Errors**: Introduced `SyntaxError` and `ParseResult` to provide detailed feedback (line, column, message) from parsers.
  - **Syntax Healer**: Initial implementation of the `Healer` module, capable of suggesting fixes for common errors like missing braces or colons.
  - **Parser Versioning**: Implemented `LegacyParserWrapper` to support a gradual migration of all 20+ supported languages to the new error reporting standard.
- **Dogfooding Milestone**: Successfully used GnawTreeWriter's own `edit` and `list` commands to perform a project-wide structural refactoring of the parser system.

### Fixed
- **System Stability**: Resolved critical borrow checker issues in recursive tree traversal and error detection logic.
- **Improved Parsers**: Enhanced Rust and Python parsers with explicit error node detection.
- **CLI Robustness**: Fixed type mismatches and variable scoping in `handle_quick_replace`.

## [0.7.3] - 2026-01-23

### Added
- **HRM 2.0 (Hierarchical Reasoning Model)**: Introduces relational awareness to GnawSense.
  - **Relational Indexing**: Automatically maps function calls and definitions across files.
  - **Impact Alerts**: GnawSense now warns you when a semantic search result is used by other parts of the project.
  - **Just-In-Time (JIT) Analysis**: Automatically indexes surrounding files during semantic search to provide context-aware impact reports.
- **Improved MCP Integration**: Semantic search tools in MCP now include impact analysis, allowing AI agents to foresee side-effects of their changes.

## [0.7.2] - 2026-01-22

### Added
- **Recursive Scaffolding**: Enhanced `scaffold` command to support deeply nested structures using `children:[...]` syntax.
- **HRM 2.0 Vision**: Documented the future path for Hierarchical Reasoning Models in `docs/HRM_VISION.md`.

### Fixed
- **CI Stability**: Resolved all remaining Clippy warnings and GitHub Actions connection timeouts.
- **Parser Robustness**: Improved the schema parser to handle various input formats for scaffolding.

## [0.7.1] - 2026-01-22

### Added
- **Structural Scaffolding**: Create new files with a predefined AST structure using the `scaffold` command.
  - Usage: `gnawtreewriter scaffold <file_path> --schema "lang:structure"`
  - Supports Rust and Python templates.
- **Arbitrary AST Indexing**: Upgraded core `insert` logic to support arbitrary child indices, enabling precise semantic insertions beyond the basic "start/end" positions.

### Fixed
- **Sense-Insert Stability**: Improved index calculation in GnawSense to ensure generated proposals are always valid for the core engine.
- **Code Cleanup**: Removed unused imports and silenced warnings in AI-related modules.

## [0.7.0] - 2026-01-22

### Added
- **GnawSense: Semantic Search & Action**: Revolutionary AI-powered navigation and editing using **ModernBERT**.
  - `gnawtreewriter sense "<query>"`: Search for code by meaning, not just text (Satelite & Zoom views).
  - `gnawtreewriter sense-insert <file> "<anchor>" "<content>"`: Insert code near a semantic landmark without needing exact paths.
  - MCP Support: Fully exposed to AI agents via `sense` and `semantic_insert` tools.
- **TCARV Methodology**: Formalized development process for AI-assisted engineering.
  - `TCARV_1_0.md`: Core methodology (Hypothesis -> Blocks -> Verification -> Shell).
  - `TCARV_ADDON_TAC.md`: Tool Architecture & Core - focus on system robustness and agnosticism.
  - `TCARV_ADDON_AUTO.md`: Autonomous Iteration rules for agent-driven development.
- **Anchor Detection System**: Ported from Comparative-Writer to support partial code injection.
  - Detects `// ...`, `# ...`, and other "existing code" markers in AI output.
  - New `src/core/anchor.rs` module for structural anchor detection.
- **Agent Intelligence & Safety**:
  - `extensions/gemini/GEMINI.md`: Direct instructions for Gemini CLI to use GnawTreeWriter proaktivt.
  - Anti-Lobotomy Policy: Strict rules against destructive code simplification.
  - Git Surgery Policy: Bans "nuclear" rollbacks; mandates surgical recovery from history.

### Fixed
- **Extension Manifest**: Restored and fixed `gemini-extension.json` with portable relative paths.
- **Ignore Rules**: Updated `.gitignore` to correctly handle extension manifests and AI models.
- **Semantic Reporting**: Added missing `category` field to quality findings and fixed `LabelManager` imports.
- **Stability**: Cleaned up deprecated ModernBERT tests and fixed syntax errors in ported logic.

### Changed
- **Agent Guidelines**: Rewrote `AGENTS.md` and `AI_AGENT_TEST_SCENARIOS.md` to align with TCARV (from "Survival" to "Architect" mindset).
- **Version Bump**: Major upgrade to v0.7.0 reflecting significant architectural and AI improvements.

---

## [0.6.0] - 2025-01-05
... (Resten av changeloggen fortsätter nedan)

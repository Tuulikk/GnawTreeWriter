# Changelog

All notable changes to GnawTreeWriter.

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

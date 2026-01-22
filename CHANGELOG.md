# Changelog

All notable changes to GnawTreeWriter.

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

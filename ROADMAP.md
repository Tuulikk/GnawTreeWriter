# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.7.9 (Released 2026-01-23)

### ✅ Completed Features (The Ecosystem Efficiency Update)

- **Smart Re-indexing**: Blixtsnabb uppdatering av projekt-indexet genom hash-koll.
- **ModernBERT Chunking**: Stöd för gigantiska filer utan krascher.
- **Ecosystem Metadata**: Delad intelligens redo för GnawMimir.
- **Multi-Actor ALF**: Support för cross-tool journaling.

---

# 🌍 Open Source Roadmap

All features in this section are and will remain **free and open source** under the project license.

---

## Phase 1: Reliability & Safety ✅ COMPLETE
**Status: DONE**

- [x] **Transaction Log System**: JSON-based log tracking all operations with timestamps.
- [x] **Multi-File Time Restoration**: Project-wide and session-based rollback.
- [x] **Undo & Redo Commands**: Navigation without Git dependency.
- [x] **Interactive Help System**: `examples` and `wizard` commands.
- [x] **Temporal Demo Project**: Step-by-step evolution guide with history snapshots.

---

## Phase 2: MCP Integration & Extensions ✅ COMPLETE
**Status: DONE**

- [x] **Stdio & HTTP Transports**: Native support for modern AI clients.
- [x] **Registry & Discovery**: Seamless tool listing for Gemini CLI and Zed.
- [x] **Surgical Edit Tools**: Precise node-based manipulation via MCP.
- [x] **Standardized Extensions**: Centralized `/extensions` directory for all integrations.

---

## Phase 3: GnawSense & Semantic Infrastructure 🔄 IN PROGRESS
**Target: Q1 2026**

### **Semantic Navigation (The Radar)**
- [x] **Zoom Mode**: Semantic search within a single file.
- [x] **Skeletal Mapping**: High-level definition overview for token efficiency.
- [x] **Node Discovery**: Search for nodes by name or content.
- [x] **Semantic Selection**: Target nodes using `@fn:name` shorthand.
- [x] **Project-wide Cache**: Background crawler that indexes the entire project into a local vector store (v0.7.7).
- [ ] **Lateral Navigation Graph**: Link nodes by usage/calls (Knowledge Graph) to allow agents to "follow the thread".
- [ ] **Context Truncation**: Smart summary generation for very large AST branches.

### **Actionable Intent (The Hand)**
- [x] **Semantic Anchors**: Basic `after` insertion based on semantic landmarks.
- [x] **Relative Placement Expansion**: Support for `INSIDE`, `BEFORE`, `BEGINNING`, and `END` using AST context.
- [x] **The Duplex Loop (Foundation)**: Self-correcting edits using structured syntax errors (v0.7.4).
- [ ] **Structural Style Transfer**: Analyze the user's specific coding style and normalize agent-generated code to match it.

---

## Phase 4: Language & Parser Expansion ✅ COMPLETE
**Status: DONE**

- [x] **New Languages**: Kotlin, Swift (v0.7.6).
- [ ] **Template Support**: Jinja2 / HTML mixed-mode parsing.
- [ ] **Multi-Parser Files**: Seamlessly switching parsers within a single file.
- [ ] **Structural Anomaly Detection**: AI-linter that warns about unsafe patterns or semantic duplication before edits.

---

## Phase 5: Intelligence & Autonomy 🔄 IN PROGRESS
**Target: Q3 2026**

- [x] **ALF (Agentic Logging Framework)**: Standardized temporal journaling for AI agents (v0.7.5).
- [x] **Structural Scaffolding**: Create new files by defining a tree schema (Moved from Phase 5 to v0.7.1).
- [ ] **HRM 2.0 Integration**: Implementation of Hierarchical Reasoning Models for side-effect prediction and structural style transfer. See [docs/HRM_VISION.md](docs/HRM_VISION.md).
- [🔄] **"Fix-my-Fix" Loop**: If an edit causes a parse error, use the AST to suggest or auto-apply the syntax fix (Initiated in v0.7.4).
- [ ] **Semantic Diffing**: Show changes as tree operations instead of line diffs.

---

## Phase 6: Universal Tree Platform 🔄 PLANNED
**Target: Q4 2026 / v1.0**

- [ ] **Gnaw Daemon**: Background process holding the project AST in memory for instant responses.
- [ ] **Cross-File Refactoring**: Symbol renaming with cross-file guarantees.
- [ ] **File Watcher**: Real-time updates to the AST when files are changed.
- [ ] **Infrastructure as Code**: Terraform, K8s YAML manipulation.

---

## Recent Progress

### v0.7.1 (2026-01-22) — THE SCAFFOLDING UPDATE 🏗️
- ✅ **Structural Scaffolding**: Command `scaffold` for creating AST-templated files.
- ✅ **Improved Indexing**: Support for arbitrary child positions in core `insert`.
- ✅ **Help System**: Added semantic search and scaffolding examples to `examples`.

### v0.7.0 (2026-01-22) — THE SEMANTIC RELEASE 🚀
- ✅ **GnawSense**: Semantic search (`sense`) and action (`sense-insert`) using ModernBERT.
- ✅ **TCARV 1.0**: Core methodology for AI-assisted engineering (+ TAC & AUTO modules).
- ✅ **Anchor System**: Ported from Comparative-Writer to handle `// ...` anchors.
- ✅ **Agent Intelligence**: Added `GEMINI.md` and updated `AGENTS.md` for safer AI collaboration.
- ✅ **Safety Policies**: Implemented Anti-Lobotomy and Git-Surgery (No-Nuke) rules.

### v0.6.11 (2026-01-12)
- ✅ **Help System Cleanup**: Fully updated `examples` and `wizard` commands to match current functionality (Contributed by OpenCode).
- ✅ **Command Documentation**: Added missing examples for `search` and `skeleton`.
- ✅ **Quick-Replace Fix**: Corrected outdated references to the `quick` command.

### v0.6.10 (2026-01-12)
- ✅ **Full CLI Parity**: Added `search`, `skeleton`, and `semantic-report` commands to match MCP capabilities.
- ✅ **Docs Cleanup**: Fixed `examples --topic ai` to accurately reflect available commands.
- ✅ **Linting**: Silenced unused field warnings in `AiManager`.

### v0.6.9 (2026-01-12)
- ✅ **Semantic Selection**: Target nodes using `@fn:name`, `@struct:name`, etc., instead of numeric paths.
- ✅ **Enhanced CLI**: Added `read` command and improved `list` output with node names.
- ✅ **Clean Core**: Moved name-extraction logic to `TreeNode` for universal use.

### v0.6.8 (2026-01-11)
- ✅ **Agent Safety Guide**: Added "The Gnaw Mental Model" to AGENTS.md to prevent AI mistakes.
- ✅ **Zed Flatpak Support**: Added dedicated documentation and `flatpak-spawn` instructions for Zed users.
- ✅ **Robust Extensions**: Improved Zed extension source code for better reliability.

### v0.6.7 (2026-01-11)
- ✅ **Contextual Usage Hints**: Added a "Just-in-Time" learning system that prints helpful tips to stderr.
- ✅ **Double-Brace Shield**: Hardened CLI and MCP outputs against common shell escaping issues.

### v0.6.6 (2026-01-11)
- ✅ **Colored Diff Preview**: Added ANSI color support for CLI previews.
- ✅ **MCP Diff Feedback**: Edit and Insert tools now return context-aware unified diffs.
- ✅ **Preview Tool**: Added `preview_edit` to MCP for "dry run" capabilities.

### v0.6.5 (2026-01-11)
- ✅ **Intelligence Loop**: Integrated LabelManager and Semantic Reporting.
- ✅ **Robust MCP**: Fixed JSON-RPC syntax and added stdio/http stability.
- ✅ **Clean Imports**: Optimized dependency usage in core modules.

### v0.6.4 (2026-01-11)
- ✅ **Skeletal Mapping**: Added `get_skeleton` for high-level definition overviews.
- ✅ **Smart Search**: Added `search_nodes` to find targets by name/text.
- ✅ **Token Efficiency**: Depth-limited listing and punctuation filtering.

### v0.6.2 (2026-01-10)
- ✅ **Full MCP Stdio Support**: Integration with Gemini CLI and Zed.
- ✅ **License Guardian**: Added `scripts/check-license.sh` to ensure MPL-2.0 purity.
- ✅ **Temporal Demo**: Added `examples/temporal-demo` micro-project.

### v0.6.0 (2025-01-05)
- ✅ Fixed GitHub Actions CI/CD for ModernBERT.
- ✅ Extensive dogfooding - fixes made using GnawTreeWriter!

---

*This roadmap is a living document. Inspired by Comparative-Thinker and Comparative-Writer.*

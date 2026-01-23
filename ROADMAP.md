# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.7.1 (Released 2026-01-22)

### ✅ Completed Features (The Structural Scaffolding Update)

- **Structural Scaffolding**: Create new files with predefined AST structures via the `scaffold` command.
- **Arbitrary AST Indexing**: Upgraded core logic to support precise child positioning beyond basic top/bottom.
- **GnawSense Engine**: Revolutionary AI-powered semantic search and action driven by **ModernBERT**.
- **Semantic Insertion**: `sense-insert` command allowing code injection near landmarks without paths.
- **TCARV Methodology**: Formalized AI-native development process (1.0 + TAC + AUTO addons).
- **Anchor Detection**: Ported from Comparative-Writer to support partial AI code snippets (`// ...`).
- **Agent Intelligence**: `GEMINI.md` and updated `AGENTS.md` for proaktive agent collaboration.
- **Robust MCP Server**: Fully exposed GnawSense tools to AI agents.
- **Safety Policies**: Anti-Lobotomy and Git-Surgery (No-Nuke) rules enforced for agents.
- **Temporal Recovery**: Restore project, files, or sessions to any point in time.
- **Multi-language support**: 17+ programming languages.

---

# 🌍 Open Source Roadmap

All features in this section are and will remain **free and open source** under the project license.

---

## Phase 1: Reliability & Safety ✅ COMPLETE
**Status: DONE**

- [x] **Transaction Log System**: JSON-based log tracking all operations.
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
- [ ] **Project-wide Cache**: Background crawler that indexes the entire project into a local vector store (Inspired by Comparative-Thinker).
- [ ] **Lateral Navigation Graph**: Link nodes by usage/calls (Knowledge Graph) to allow agents to "follow the thread".
- [ ] **Context Truncation**: Smart summary generation for very large AST branches.

### **Actionable Intent (The Hand)**
- [x] **Semantic Anchors**: Basic `after` insertion based on semantic landmarks.
- [x] **Relative Placement Expansion**: Support for `INSIDE`, `BEFORE`, `BEGINNING`, and `END` using AST context.
- [ ] **Structural Style Transfer**: Analyze the user's specific coding style and normalize agent-generated code to match it.
- [ ] **The Duplex Loop**: Self-correcting edits where GnawSense validates its own proposal against the AST before presenting it.

---

## Phase 4: Language & Parser Expansion 🔄 PLANNED
**Target: Q2 2026**

- [ ] **New Languages**: Kotlin, Swift, Scala, Ruby, Lua.
- [ ] **Template Support**: Jinja2 / HTML mixed-mode parsing.
- [ ] **Multi-Parser Files**: Seamlessly switching parsers within a single file.
- [ ] **Structural Anomaly Detection**: AI-linter that warns about unsafe patterns or semantic duplication before edits.

---

## Phase 5: Intelligence & Autonomy 🔄 PLANNED
**Target: Q3 2026**

- [ ] **ALF (Agentic Logging Framework)**: Standardized temporal journaling for AI agents (`ALF.md`).
- [x] **Structural Scaffolding**: Create new files by defining a tree schema (Moved from Phase 5 to v0.7.1).
- [ ] **"Fix-my-Fix" Loop**: If an edit causes a parse error, use the AST to suggest or auto-apply the syntax fix.
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
- ✅ **Help System Cleanup**: Updated examples and wizard commands.

---

*This roadmap is a living document. Inspired by Comparative-Thinker and Comparative-Writer.*
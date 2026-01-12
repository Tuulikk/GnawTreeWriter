# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.6.5 (Released 2026-01-11)

### ✅ Completed Features

- **Intelligence Loop**: Integrated `LabelManager` and `SemanticReport` for persistent AI-driven code analysis.
- **Robust MCP Server**: Fixed JSON-RPC compliance, added support for both Stdio and HTTP, and ensured stability for agents.
- **Scalable MCP Tools**: Skeletal mapping (`get_skeleton`) and pattern-based discovery (`search_nodes`) for large files.
- **Context Management**: Depth-limited listing and automatic noise reduction in AST views.
- **Native Extensions**: Official support for Gemini CLI and Zed extensions.
- **Multi-language support**: Python, Rust, TypeScript, JavaScript, PHP, HTML, QML, Go, Java, Zig, C, C++, Bash, XML, YAML, TOML, CSS, Markdown.
- **TreeSitter Foundation**: Robust parsing for all core languages.
- **Smart Indentation**: Automatic preservation of code style during insertions.
- **Syntax Validation**: In-memory re-parsing before saving changes.
- **Automatic Backups**: JSON snapshots before every edit.
- **Temporal Recovery**: Restore project, files, or sessions to any point in time.

---

# 🌍 Open Source Roadmap

All features in this section are and will remain **free and open source** under the project license.

---

## Phase 1: Reliability & Safety ✅ COMPLETE
**Status: DONE**

### ✅ Core Safety & Recovery System
- [x] **Transaction Log System**: JSON-based log tracking all operations with timestamps.
- [x] **Multi-File Time Restoration**: Project-wide and session-based rollback.
- [x] **Undo & Redo Commands**: Navigation without Git dependency.
- [x] **Interactive Help System**: `examples` and `wizard` commands.
- [x] **Temporal Demo Project**: Step-by-step evolution guide with history snapshots.

---

## Phase 2: MCP Integration & Extensions ✅ COMPLETE
**Status: DONE**

### ✅ MCP Server & IDE Support
- [x] **Stdio & HTTP Transports**: Native support for modern AI clients.
- [x] **Registry & Discovery**: Seamless tool listing for Gemini CLI and Zed.
- [x] **Surgical Edit Tools**: Precise node-based manipulation via MCP.
- [x] **Standardized Extensions**: Centralized `/extensions` directory for all integrations.

---

## Phase 3: AI-Enhanced Editing ✅ IN PROGRESS
**Status: Early Phase 3 Started**

### **Smart Semantic Targeting**
- [x] **Skeletal Mapping**: High-level definition overview for token efficiency.
- [x] **Node Discovery**: Search for nodes by name or content without counting indexes.
- [x] **Contextual Usage Hints**: Just-in-Time learning tips in CLI stderr to guide users and agents (e.g., suggesting 'undo' after edits).
- [ ] **Semantic Selection**: `--function "name"` targeting instead of raw paths.
- [ ] **Context Truncation**: Smart summary generation for very large AST branches.

### **Local AI Features** (ModernBERT)
- [x] **Semantic Search**: Find code by meaning with `--semantic` flag.
- [x] **AI Refactoring Suggestions**: Identify complex code patterns.
- [x] **Context-Aware Completion**: AST-based code completion.
- [ ] **Structural Anomaly Detection**: AI-linter that warns about unsafe patterns or semantic duplication before edits.

---

## Phase 4: Language & Parser Expansion 🔄 PLANNED
**Target: Q2 2026**

- [ ] **New Languages**: Kotlin, Swift, Scala, Ruby, Lua.
- [ ] **Template Support**: Jinja2 / HTML mixed-mode parsing (handling embedded languages).
- [ ] **Multi-Parser Files**: Seamlessly switching parsers within a single file (e.g., JS inside HTML).

---

## Phase 5: Intelligence & Autonomy 🔄 PLANNED
**Target: Q3 2026**

- [ ] **Structural Scaffolding**: Create new files by defining a tree schema (e.g., "mod:MyFeature(struct:Config)") rather than raw text, ensuring valid syntax from start.
- [ ] **"Fix-my-Fix" Loop**: If an edit causes a parse error, use the AST to suggest or auto-apply the syntax fix (e.g., closing missing braces).
- [ ] **Semantic Diffing**: Show changes as tree operations ("Renamed function X") instead of line diffs.

---

## Phase 6: Universal Tree Platform 🔄 PLANNED
**Target: Q4 2026 / v1.0**

- [ ] **Gnaw Daemon**: Background process holding the project AST in memory for instant edits and query responses.
- [ ] **Cross-File Refactoring**: Rename symbols or move code across files with guaranteed safety.
- [ ] **File Watcher**: Real-time updates to the AST when files are changed by other editors.
- [ ] **Infrastructure as Code**: Terraform, K8s YAML manipulation.

---

## Recent Progress

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

*This roadmap is a living document. Priorities may shift based on community feedback and market needs.*

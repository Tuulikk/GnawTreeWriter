# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.6.2 (Released 2026-01-10)

### ✅ Completed Features

- **Full MCP Support**: Stdio and HTTP transport layers for AI agent integration.
- **Native Extensions**: Official support for Gemini CLI and Zed extensions.
- **Multi-language support**: Python, Rust, TypeScript, JavaScript, PHP, HTML, QML, Go, Java, Zig, C, C++, Bash, XML, YAML, TOML, CSS, Markdown.
- **TreeSitter Foundation**: Robust parsing for all core languages.
- **Smart Indentation**: Automatic preservation of code style during insertions.
- **Syntax Validation**: In-memory re-parsing before saving changes.
- **Diff Preview**: Visual unified diff display using `similar` library.
- **Automatic Backups**: Non-git safety net creating JSON snapshots before every edit.
- **Batch Operations**: Atomic multi-file editing via JSON specification with rollback.
- **Quick Command**: Fast, low-overhead edits supporting AST and text-based modes.
- **Diff-to-Batch**: Converts unified diffs to batch operation specifications.
- **ModernBERT AI Integration**: Local, privacy-focused AI features (optional).
- **Clone Operation**: Duplicate code nodes within or between files.
- **Refactor/Rename**: AST-aware symbol renaming across files.

---

# 🌍 Open Source Roadmap

All features in this section are and will remain **free and open source** under the project license.

---

## Phase 1: Reliability & Safety ✅ COMPLETE
**Status: DONE**

The foundation that makes GnawTreeWriter bulletproof and independent of Git for session-level recovery.

### ✅ Core Safety & Recovery System

- [x] **Transaction Log System**: JSON-based log tracking all operations with timestamps.
- [x] **Multi-File Time Restoration**: `restore-project`, `restore-files`, `restore-session`.
- [x] **Undo & Redo Commands**: Navigate backup history without Git dependency.
- [x] **Enhanced Restore System**: Point-in-time recovery with preview.
- [x] **Interactive Help System**: `examples` and `wizard` commands for guided learning.
- [x] **Temporal Demo Project**: A complete micro-project showing the evolution of a tool with history snapshots.
- [x] **AI Agent Testing Framework**: Comprehensive test scenarios for AI agents.

---

## Phase 2: MCP Integration & Daemon ✅ IN PROGRESS
**Status: v0.6.2 - MCP Core DONE**

Core MCP and daemon features that make GnawTreeWriter usable by AI agents and IDEs.

### **MCP Server** (Optional Feature) ✅ COMPLETE

- [x] **Basic MCP Tool Server**:
  - `gnawtreewriter mcp stdio` - Direct pipe communication (preferred).
  - `gnawtreewriter mcp serve` - Run as HTTP server.
  - Expose all core operations as MCP tools (analyze, edit, list, etc.).
  - Native support for Gemini CLI and Zed.

- [x] **MCP Tool Definitions**:
  - `analyze` - Parse and return AST structure.
  - `list_nodes` - Flat list of edit targets with paths.
  - `read_node` - Get source of specific node.
  - `edit_node` - Surgical node-based editing.
  - `insert_node` - Indentation-aware code insertion.

### **Local File Watcher Daemon** (Optional Feature) 🔄 PLANNED

- [ ] **Project Monitoring Daemon**:
  - `gnawtreewriter daemon start [--project <path>]` - Start background watcher.
  - Real-time file change detection (even from external editors).
  - Automatic backup on every save.
  - Conflict detection: "File changed outside GnawTreeWriter".

---

## Phase 3: AI-Enhanced Editing
**Target: v0.8.0 - Q3 2025**

### **Smart Semantic Targeting**

- [ ] **Semantic Node Selection**:
  - `--function "main"` instead of raw paths.
  - `--class "UserController" --method "create"`.
  - Fuzzy matching with confidence scores.

- [ ] **LLM-Optimized Output**:
  - Token-compressed JSON formats for large ASTs.
  - Hierarchical detail levels: summary → detailed → full AST.

### **Local AI Features** (ModernBERT)

- [x] **Semantic Search**: Find code by meaning with `--semantic` flag.
- [x] **AI Refactoring Suggestions**: Identify complex code patterns.
- [x] **Context-Aware Completion**: AST-based code completion.
- [ ] **Pattern Detection**: Identify anti-patterns and suggest improvements.

---

## Phase 4: Language & Parser Expansion
**Target: v0.9.0 - Q4 2025**

- [ ] **New Languages**: Kotlin, Swift, Scala, Ruby, Lua.
- [ ] **Multi-Parser Files**: Handle embedded languages (JS in HTML, etc.).

---

## Phase 5: Universal Tree Platform
**Target: v1.0.0 - 2026**

- [ ] **Infrastructure as Code**: Terraform, CloudFormation, Kubernetes YAML.
- [ ] **Configuration Management**: Docker Compose, CI/CD Pipelines.

---

# 💎 Premium/Enterprise Roadmap

---

## Phase 1: Multi-Project & Team Collaboration
**Target: Q2 2025**

- [ ] **Project Manager System**: ARCHIVED session switching.
- [ ] **GnawTreeWriter Server**: Self-hosted coordination for teams.
- [ ] **Multi-Agent Coordination**: Conflict prevention for concurrent AI agents.

---

## Recent Progress

### v0.6.2 (2026-01-10)
- ✅ **Full MCP Stdio Support**: Seamless integration with Gemini CLI and Zed.
- ✅ **Robust Handshake**: Handles LSP-style headers and notification-only flows.
- ✅ **Refactored Extensions**: Moved to root `/extensions` directory.
- ✅ **Unified Documentation**: New MCP integration guides.

### v0.6.0 (2025-01-05)
- ✅ Fixed GitHub Actions CI/CD for ModernBERT.
- ✅ Extensive dogfooding - fixes made using GnawTreeWriter!

### v0.5.0 (2025-01-06)
- ✅ ModernBERT AI Integration (semantic search, refactoring, completion).
- ✅ Clone operation for code duplication.
- ✅ Zig language support.

---

*This roadmap is a living document. Priorities may shift based on community feedback and market needs.*
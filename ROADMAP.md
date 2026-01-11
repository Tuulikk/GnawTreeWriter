# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.6.4 (Released 2026-01-11)

### ✅ Completed Features

- **Scalable MCP Tools**: Skeletal mapping (`get_skeleton`) and pattern-based discovery (`search_nodes`) for large files.
- **Context Management**: Depth-limited listing and automatic noise reduction in AST views.
- **Full MCP Support**: Stdio and HTTP transport layers for AI agent integration.
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
- [ ] **Semantic Selection**: `--function "name"` targeting instead of raw paths.
- [ ] **Context Truncation**: Smart summary generation for very large AST branches.

### **Local AI Features** (ModernBERT)
- [x] **Semantic Search**: Find code by meaning with `--semantic` flag.
- [x] **AI Refactoring Suggestions**: Identify complex code patterns.
- [x] **Context-Aware Completion**: AST-based code completion.
- [ ] **Structural Anomaly Detection**: Flag inconsistent code patterns using ModernBERT.

---

## Phase 4: Language & Parser Expansion 🔄 PLANNED
**Target: Q4 2025**

- [ ] **New Languages**: Kotlin, Swift, Scala, Ruby, Lua.
- [ ] **Template Support**: Jinja2 / HTML mixed-mode parsing.
- [ ] **Multi-Parser Files**: Handle embedded languages.

---

## Phase 5: Universal Tree Platform 🔄 PLANNED
**Target: v1.0.0 - 2026**

- [ ] **Infrastructure as Code**: Terraform, K8s YAML manipulation.
- [ ] **Local Daemon**: Background file watcher and conflict detection.

---

## Recent Progress

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

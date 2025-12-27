# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

## Current Status: v0.2.1 (Released 2025-12-26)

### ✅ Completed Features

- **Multi-language support**: Python, Rust, TypeScript, PHP, HTML, QML, **Go**.
- **TreeSitter Foundation**: Robust parsing for all core languages.
- **Smart Indentation**: Automatic preservation of code style during insertions.
- **Syntax Validation**: In-memory re-parsing before saving changes.
- **QML Intents**: Dedicated commands for `add-property` and `add-component`.
- **Diff Preview**: Visual unified diff display using the `similar` library.
- **Automatic Backups**: Non-git safety net creating JSON snapshots before every edit.

---

## Phase 1: Reliability & Safety (The Non-Git Safety Net)
**Target: v0.3.0 - Q1 2026**

Focus on making the tool bulletproof and independent of Git for session-level recovery.

- [ ] **`undo` & `redo` Commands**: Traverse the `.gnawtreewriter_backups` history without touching Git.
- [ ] **Transaction Log**: A human/agent-readable log of all structural changes made during a session.
- [ ] **`restore <timestamp|id>`**: Browse and restore specific node states from the backup repository.
- [ ] **Content-based Node IDs**: Move from index-based paths (`0.1`) to more stable hash-based IDs where possible to handle concurrent changes.

---

## Phase 2: Connectivity & Agent Integration
**Target: v0.4.0 - Q2 2026**

Making it easier for AI Agents (like Claude, GPT, Raptor) to use the tool effectively.

- [ ] **MCP Server Implementation**: Native Model Context Protocol support to let agents call GnawTreeWriter as a built-in tool.
- [ ] **Smart Path Targeting**: Allow edits by name/type instead of just raw path (e.g., `edit --function "main" --content "..."`).
- [ ] **Token Optimization**: Compressed JSON output formats designed specifically for LLM context windows.
- [ ] **"Intent Extrapolation"**: High-level commands that perform complex AST transformations from minimal input.

---

## Phase 3: Intelligence & Background Automation
**Target: v0.5.0 - Q3 2026**

Moving towards an "always-on" assistant that maintains the tree structure.

- [ ] **Analysis Toggles**: Background "watch mode" that keeps the tree updated and alerts agents of structural changes.
- [ ] **Structural Linting**: Go beyond syntax errors to detect logical AST issues (e.g., "This Rectangle has no size properties").
- [ ] **Visual AST Explorer (Optional UI)**: A TUI or Web-view to visualize the tree, helping developers and agents debug complex paths.
- [ ] **Batch Scripting**: A simple DSL (Domain Specific Language) to chain multiple tree operations into one atomic unit.

---

## Future Vision

- **LSP Integration**: Serve as a backend for Language Server Protocol, providing structural editing to any IDE.
- **Refactoring Engine**: Automated structural refactoring (extract function, move component) that is 100% safe.
- **AI-Native IDE Bridge**: Deep integration with Cursor, VS Code Copilot, and other AI-centric editors.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Technical design
- [FUTURE_CONCEPTS.md](docs/FUTURE_CONCEPTS.md) - Deep dive into planned features
- [LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) - Guide for AI agents
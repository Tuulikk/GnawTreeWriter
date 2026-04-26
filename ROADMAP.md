# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.8.5 (Released 2026-01-27)

### ✅ Completed Features (The Handbook Update)

- **The Architect Handbook**: Consolidated quickstart guide for high-level engineering.
- **Human-Readable Sessions**: Support for naming sessions (aliases) for easier restoration.
- **Safe Content Injection**: Added `@file` syntax to the `edit` command to bypass shell escaping issues.
- **The Helpful Guard**: Proactive CLI error handling with strategic advice and tips.

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
**Target: Q1–Q3 2026**

### Architecture

```
src/llm/
├── ai_manager.rs          — Model management (load ModernBERT, device selection)
├── gnaw_sense.rs          — Broker: Satelite/Zoom/ProposeEdit
├── semantic_index.rs      — Vector storage + cosine similarity search
├── project_indexer.rs     — Project crawler → per-node embeddings
└── relational_index.rs    — Call graph (Call/Definition/Reference relations)
```

**Two modes**: Satelite (project-wide search) and Zoom (file-level search with impact analysis).
**Model**: ModernBERT-base (149M params, 768-dim, 571 MB) via Candle — fully local, no external calls.

### Performance Baseline (Measured 2026-04-26)

| Step | Time | Bottleneck? |
|------|------|-------------|
| Load ModernBERT (571 MB) | ~2–3 sec | 🔴 YES — 90% of total time |
| Generate 1 embedding | ~50–200 ms | 🟡 Medium |
| Brute-force cosine sim (468 nodes) | ~0.15 ms | 🟢 Negligible |
| Load 23 JSON index files | ~5–10 ms | 🟢 Negligible |

**Key insight**: Vector DB is NOT needed at current scale. The bottleneck is model loading, not search.

---

### Tier 1: Quick Fixes (Hours)

- [x] **Confidence threshold**: Only return results with score > 0.3. Warn if best result is > 0.5 (low confidence). Prevents false-positive "matches" that agents trust.
- [x] **JSON output for sense**: `--json` flag for machine-readable results. Agents currently parse ANSI/emoji stdout.
- [x] **Extract name — all languages**: Add patterns for `def `, `class `, `func `, `pub fn`, `async fn`, `impl`, `trait`, methods, etc. Currently only handles `fn ` and `struct `.
- [x] **Better error when model not loaded**: `not(feature = "modernbert")` should return JSON-formatted error for agents.

### Tier 2: Agent-Friendly (1–2 Days)

- [ ] **Auto-index without prompt**: `--auto-index` flag that indexes without interactive y/N. Current prompt blocks agents.
- [ ] **Sense-insert with all intents**: Support `before`, `inside`, `replace` — not just `after`. Currently returns "Unsupported intent" for everything else.
- [ ] **Fix propose_edit position logic**: `get_next_index()` uses `idx + 3 + 1` hack. Replace with proper AST sibling lookup via GnawTreeWriter.

### Tier 3: Performance (2–3 Days) — 🔥 CURRENT PRIORITY

- [ ] **Model caching**: Load ModernBERT once into `Arc<Mutex<Option<ModernBertModel>>>`. Reuse across calls. Eliminates 2–3 sec per invocation. 10–20x speedup.
- [ ] **JIT index cache**: Cache Zoom-mode embeddings per file+content-hash. Avoid re-embedding unchanged files.
- [ ] **Incremental project indexing**: Only re-index changed files at `ai index`. Currently re-embeds everything (smart hash check exists but is per-file, not global).

### Tier 4: Intelligence (1–2 Weeks)

- [ ] **Query expansion**: Expand vague queries ("fixa git-grejen") into multiple embedding searches for better recall.
- [ ] **Hierarchical context in embeddings**: Include parent node type/name when generating embeddings. A `login` function in `tests/` differs from one in `auth/`.
- [ ] **Fix RelationalIndex integration**: `index_directory()` is only called inside `sense()` JIT, never during `ai index`. Call graph is never properly built at index time.
- [ ] **Feedback loop**: Log failed searches (no results, low confidence) and adjust scoring. Spec calls this "AUTO-koppling".

### Tier 5: Scale & Vision (Future)

- [ ] **HNSW index**: Pure Rust `hnsw` crate for approximate nearest neighbor. Only needed at 10k+ nodes. Current brute-force takes 0.15 ms at 468 nodes.
- [ ] **Smaller/code-specialized model**: Evaluate swapping ModernBERT-base (571 MB) for a smaller code-tuned model to reduce memory footprint.
- [ ] **Side-effect prediction**: Use relational graph to warn about downstream impact before edits (HRM Vision).
- [ ] **Structural style transfer**: Learn user's coding style and normalize agent-generated code.
- [ ] **Semantic diffing**: Show changes as tree operations instead of line diffs.

---

### Completed (Foundation)

- [x] **Zoom Mode**: Semantic search within a single file.
- [x] **Skeletal Mapping**: High-level definition overview for token efficiency.
- [x] **Node Discovery**: Search for nodes by name or content.
- [x] **Semantic Selection**: Target nodes using `@fn:name` shorthand.
- [x] **Project-wide Cache**: Background crawler that indexes the entire project into a local vector store (v0.7.7).
- [x] **Semantic Anchors**: Basic `after` insertion based on semantic landmarks.
- [x] **Relative Placement Expansion**: Support for `INSIDE`, `BEFORE`, `BEGINNING`, and `END` using AST context.
- [x] **The Duplex Loop (Foundation)**: Self-correcting edits using structured syntax errors (v0.7.4).

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

### v0.9.2 (2026-04-25) — THE AGENT TOOLBELT UPDATE 🔧
- ✅ **Quick Insert**: Bulk insert content after regex-matched lines with `--filter`, `--unique`, preview support.
- ✅ **5 New Languages**: JavaScript, C#, Dart, Svelte, SQL → 26 languages total.
- ✅ **Global `--dry-run`**: All edit commands respect global dry-run flag for safer agent workflows.
- ✅ **Diagnostics Module**: `doctor` command, `GNAW_JSON=1`, `GNAW_VERBOSE=1`, post-edit AST diff.
- ✅ **Better Error Context**: Offending code line, language name, nearby nodes on parse failure.
- ✅ **`commands` endpoint**: `gtw commands --json` lists all tools with metadata for dynamic integration.

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

---

## Phase 7: Debuggability & Agent Diagnostics ✅ COMPLETE
**Target: v0.10.0 | Q2 2026**

*Investigation date: 2026-04-24 — comprehensive audit of existing debugging infrastructure.*

### Existing Infrastructure (Already Complete)
- [x] **Transaction Log System** — JSON-based operation log with timestamps, hashes, session IDs
- [x] **Backup System** — Per-edit JSON backups with content hashes
- [x] **Undo/Redo** — Stack-based navigation through transaction history
- [x] **Restoration Engine** — Project-wide and session-based rollback
- [x] **Guardian Engine** — Edit impact analysis (volume, complexity, comment preservation)
- [x] **Healer (Duplex Loop)** — Auto-heal for simple syntax errors (missing braces/colons)
- [x] **Post-edit Validation** — Reparse code after edit, block invalid syntax
- [x] **TreeVisualizer** — Visual diff with focus-nodes and sparklines
- [x] **Health Check (`status`)** — Environment, Git, AI, MCP, undo-state
- [x] **ALF (Agentic Logging)** — Intent/risk/outcome journaling
- [x] **Report Engine** — Markdown structural evolution reports
- [x] **SyntaxError** — Parsing errors with line/col/message
- [x] **Lint Command** — Find issues in files

### Prio 1 — Agent-Critical (Making GTW Easy to Debug for AI Agents)
- [x] **`--dry-run` on edit/insert/delete** — Return what *would* happen without writing to disk (global flag working on all commands)
- [x] **Structured JSON errors** — `GNAW_JSON=1` env var gives machine-readable errors with error_type, suggestion, context
- [x] **`--verbose` flag** — `GNAW_VERBOSE=1` visar parser-val, node-uppslagning, guardian-score, AST-validering, structural changes

### Prio 2 — Robustness
- [x] **`doctor` command** — Test all parsers, check backup integrity, validate transaction log
- [x] **Better error context** — Include offending code line at parse errors, language name, nearby nodes

### Prio 3 — Advanced
- [x] **Post-edit AST diff** — Compare tree before/after and warn if structure breaks (e.g. function_declaration becomes something else)
- [ ] **Reference analysis** — Warn if deletion affects referenced code (future: cross-file)

### Identified Gaps (Why These Features Matter)
1. **No post-edit AST verification** — Validation checks if code *parses*, not that tree structure is consistent
2. **No `--dry-run` on single edits** — Only batch and quick-replace have preview. Agents can't test-drive edits
3. **Limited error messages** — `SyntaxError` has line/col but lacks code context, fix suggestions, and language info
4. **No structured error format** — All errors formatted for humans (emojis, ANSI). Agents need JSON
5. **No debug/verbose flag** — No way to see what GTW does step-by-step (parser choice, node lookup, etc.)
6. **No `doctor` command** — `Status` shows env but doesn't validate parsers, backups, or log consistency
7. **No consequence analysis** — Guardian checks volume/complexity but not reference integrity or scope
8. **No diff command** — Can't see before/after detail of an edit (only visualizer with focus)

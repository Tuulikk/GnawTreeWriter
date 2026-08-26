## [0.9.7] - 2026-08-25

### Added
- **Rules engine (`lint --rules`)**: semgrep-like pattern matching against ASTs (spec: docs/RULES_ENGINE_SPEC.md).
  - YAML rules with `$X` placeholders, compiled to structural AST matching in Rust
  - Builtin rules (rust_unwrap, rust_self_assignment, py_bare_except, py_eval, js_console_log)
  - Project rules auto-loaded from `gnawtreewriter.rules.yaml`
  - `--rules <file>`, `--severity`, `--rule <id>` filters, JSON output
  - `lint` is now real lint (structural rules), not just a parse check
- **Rules guardian on edit (Duplex Loop 2.0)**: after validation, builtin rules run on the new code — error-severity findings block the edit (unless `--force`), warnings are printed but allowed. From "does it parse?" to "is it good code?".
- **Rule annotations in `edit --ask` prompts**: known rule violations in the file are injected into the LLM prompt so the model can avoid introducing/worsening them (rule-injected expertise). `edit --ask` switched to a line-based proposal format (`{"line": N, "new": "..."}`) — far more reliable for a small model than JSON-escaping whole code snippets.
- **Local LLM command extension (LFM2.5-1.2B, Q4)** behind the `mamba` feature:
  - `explain <file> [--node <path>]` — plain-language explanation of a code node
  - `summarize <dir>` — hierarchical AST-skeleton map-reduce summary
  - `investigate "question"` — query expansion → index search → ranked answer with file references
  - All three available as MCP tools (`explain`, `summarize`, `investigate`)
  - `ai calibrate` — measures this machine's inference speed and saves a timing profile
- **`--resolution` flag** (fast / balanced / thorough) on explain/summarize/investigate: trades speed vs detail via chunk size and output budgets.
- **Token & time transparency**: every command reports a budget (`expected/actual tokens`, `calls`, `estimated/actual seconds`, truncation warning). No silent cut-offs.
- **`edit --ask "request"`** (+ MCP `edit_ask`): the local model proposes a minimal old→new change; GTW finds the containing AST node, applies the replacement inside it, and validates through the Duplex Loop before apply. The AST places, the model states intent — a small model stays viable because it never needs mechanical precision.
- **ROADMAP.md**: updated to current status (v0.9.7) with a new Phase 8 (Local LLM Command Extension) section and the planned LLM missions.

### Performance notes
- LFM2.5 prefill in candle is ~O(seq²), so pipeline steps chunk to small sizes and feed compact AST skeletons instead of raw source (~98% smaller) — summarize went from >3 min/file to ~15s/file.

## [0.9.6] - 2026-08-24

### Added
- **`explore` command**: Map-like navigation with 4 zoom levels (overview / directory / file / full), each node carrying token counts and drill-down hints.
- **Session-level parse cache** (`parse_cache.rs`): Files parsed once are reused across tool calls in the same session — no redundant AST parses.
- **`pack --compress-threshold N`**: Compress only files larger than N tokens; 0 (default) compresses everything.
- **`scripts/benchmark_ai.sh`**: Reproducible benchmark for explore/pack/index/state operations.

### Changed
- **Parallel processing (rayon)**: pack, explore (directory level), and MCP index batch handlers now run across all CPU cores. Output order is preserved — results remain byte-identical to the sequential versions.

### Performance (measured on own codebase, 79 files / 169k tokens)
| Operation | Before | After |
|---|---|---|
| pack --compress | 676 ms | 335 ms (-50%) |
| explore directory (level 1) | 318 ms | 130 ms (-59%) |
| index_entities (20 files) | 177 ms | 107 ms (-40%) |
| index_relations (20 files) | 84 ms | 40 ms (-52%) |

## [0.9.5] - 2026-08-23

### Added
- **AI-Friendly Context Tools** (inspired by Repomix analysis):
  - `compress` command: Replace function bodies with `⋮----` placeholders (~70% token reduction)
  - `pack` command: Package entire project into AI-optimized format (markdown/json/plain)
  - `curate` command: Intelligent file selection based on task description
  - Token counting in `analyze` output (estimated_tokens field)
  - Secret detection and redaction (18 patterns: AWS, GitHub, GitLab, Stripe, JWT, etc.)
  - Git-aware file walking (respects .gitignore, replaces hardcoded skip-lists)

- **New MCP tools**: `compress`, `pack`, `curate`

- **New modules**:
  - `file_walker.rs`: Git-aware traversal using `ignore` crate
  - `token_count.rs`: Heuristic token estimation for LLM context planning
  - `compress.rs`: AST-based code compression
  - `pack.rs`: Project packaging with token budgets
  - `secrets.rs`: Credential detection and redaction
  - `curator.rs`: Multi-strategy context curation (relevance, git changes, dependencies)

### Changed
- Replaced hardcoded skip-lists in `gnaw_find`, `blast`, `inspect`, `gnaw_refactor`, `project_indexer`, `relational_index` with unified git-aware walker

## [0.9.4] - 2026-04-30

### Added
- **`multi-replace` command**: Multiple search/replace pairs in one pass
  - Supports STDIN (`--pairs -`), inline JSON, or file path
  - Single file read, single backup, single transaction
  - Atomic: all-or-nothing validation
  - Auto-unescape of `\n`/`\t` in replacement text
  - Example: `echo '[{"search":"a","replace":"b"}]' | gtw multi-replace file.rs --pairs -`

- **Batch STDIN support**: `batch -` reads JSON from STDIN
  - No temp file needed for pipelining
  - Example: `cat ops.json | gtw batch - --preview`

### Documentation
- **SKILL.md**: Added Quick STDIN Reference table at top
- **SKILL.md**: Added "Agent Workflows" section with 10 patterns
- **GTW_AGENT_COOKBOOK.md**: New 287-line cookbook with 10 recipes for AI agents

## [0.9.3] - 2026-04-27

### Added
- **GnawSense Semantic Navigation** (AI-powered code search & insertion):
  - `sense` command: search code by meaning using local ModernBERT model
  - `sense-insert` command: insert code at semantically located anchors
  - All 4 intents supported: `after`, `before`, `inside`, `replace`
  - `--auto-index` flag: skip interactive prompt for AI agents/CI
  - `GNAW_JSON=1` environment variable for machine-readable output
  - Confidence threshold: filters < 0.2, warns < 0.5
  - Multi-language `extract_name_from_preview`: 15+ patterns (Rust, Python, Go, JS, Java, C, QML)
  - Standardized `err_modernbert_disabled()` helper with JSON support

### Fixed
- **quick-replace literal \n bug**: Auto-detects literal `\n`/`\t` in replacement text and converts to real newlines/tabs (prevented broken file writes from CLI escaping)
- **sense-insert position logic**: Fixed off-by-one in `get_next_index()` — was `idx+3+1`, now `idx+3`
- **Compiler warnings**: Eliminated all warnings (unused variable, dead code)

### Changed
- **Performance**: Model caching with `OnceLock<ModernBertModel>` — loads once, reuses across calls
- **Performance**: JIT file index cache with content-hash invalidation in `GnawSenseBroker`
- **SemanticIndex** now derives `Clone` for caching support

### Documentation
- GnawSense SKILL.md for AI agents at `~/.pi/agent/skills/gnaw-sense/`
- ROADMAP.md: detailed 5-tier GnawSense improvement plan with measured baselines
# Changelog

All notable changes to GnawTreeWriter.

## [0.9.2] - 2026-02-05

### Fixed
- **Critical Compilation Errors**: Fixed 56+ syntax errors in `src/cli.rs` caused by unescaped double quotes in help text examples. All println! statements containing nested quotes have been properly escaped.
- **Build System**: Restored compilation on Rust stable by fixing string literal syntax issues.

### Technical Details
- Problem: String literals like `println!("text "quote" more")` were interpreted as separate tokens
- Solution: Escaped all nested quotes as `println!("text \"quote\" more")`
- Affected: Help text in examples subcommand covering editing, search, restoration, and AI features
- Lines modified: ~80 println! statements across 30+ example categories

## [0.9.1] - 2026-02-04

### Added
- **Surgical Inline Editing**: Character-level precision for code edits. You can now edit specific nodes (like parameters or variable names) within a single line without affecting surrounding code.
- **Pedagogical Syntax Tips**: The editor now provides language-specific advice when an edit fails syntax validation (Rust, QML, Python).
- **Column-Aware TreeNode**: Upgraded `TreeNode` structure and the Rust parser to track and utilize character offsets for enhanced precision.

### Changed
- **Enhanced Documentation**: Updated `README.md`, `examples`, and the interactive `wizard` to reflect the new surgical precision capabilities.
- **Version Bump**: Major refinement release marking the transition to v0.9.1 "The Surgical Update".

### Fixed
- **Precision Failures**: Resolved issues where inline edits would inadvertently delete parts of the line.
- **CLI Robustness**: Improved error reporting for JSON and cross-file operations.

## [0.9.0] - 2026-01-31

### Added
- **Slint Support**: Full AST-based editing and analysis for `.slint` files. Powered by `tree-sitter-slint`.
- **AI Default**: The `modernbert` (GnawSense) and `mcp` features are now enabled by default. No more `--features` flags needed for standard usage.
- **Enhanced Status**: The `status` command now proudly displays the state of **GnawSense**, **HRM2** (Hierarchical Reasoning), and **Undo/Redo** history.
- **GnawTree Architect Skill**: A specialized agent skill (`gnawtree-architect`) to guide AI agents in surgical code editing.

### Fixed
- **Safety Nets**: Implemented node count limits (500-1000 nodes) and depth limits in `list`, `skeleton`, and MCP tools to prevent agent context crashes.
- **Memory Optimization**: Refactored `list_nodes` to avoid cloning entire subtrees, significantly reducing memory usage on large files.
- **CLI Hygiene**: Removed duplicate `Status` command handlers and cleaned up unused imports in core modules.
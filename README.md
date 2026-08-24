# GnawTreeWriter

**AI-native tree-based code editor** - Edit code files based on AST structure levels with surgical precision, avoiding the common pitfalls of LLM-generated code.

## 🚀 Version 0.9.6: Explore, Parallel Performance, Smarter Packs

The AI-facing tooling gets a major upgrade in this release — faster, more explorable, and more flexible:

- **`explore` command**: Map-like navigation with zoom levels. `overview (0)` → project tree with token counts, `directory (1)` → per-file summaries, `file (2)` → structural signatures, `full (3)` → complete source. Drill through a codebase like a map, not a file dump.
- **Parallel processing (rayon)**: The heavy AST workflows now run across all CPU cores:
  - `pack --compress` is **50% faster** (676ms → 335ms)
  - `explore --level 1` is **59% faster** (318ms → 130ms)
  - `index_entities` batch is **40% faster** on 20 files (177ms → 107ms)
  - `index_relations` batch is **52% faster** on 20 files (84ms → 40ms)
  - Output order is preserved — results stay byte-identical to the sequential versions.
- **`pack --compress-threshold N`**: Compress only files larger than N tokens. Small files stay readable, big files get the ⋮---- treatment. Useful for huge repos where compression of small files costs more than it saves.
- **Session-level parse cache**: Files parsed once are reused across tool calls in the same session — no redundant AST parses when multiple tools touch the same file.

## 🚀 Version 0.9.5: Move, MacroDispatcher, Candle 0.9

This release brings three major improvements:

- **`move` command**: Atomically delete a node and reinsert it at a new location. Works cross-file. `gnawtreewriter move <src_file> <src_path> [tgt_file] <tgt_path>`. MCP: `move_node`.
- **MacroDispatcher**: Parses `json!()` macro bodies with a JSON parser, injecting 2600+ virtual AST nodes. Lays foundation for other macro-aware parsing (SQL, TOML, etc.).
- **Candle 0.9.2**: Fixes ModernBERT rope dimension mismatch. `sense`, `semantic_edit`, `semantic_insert` now work reliably.

## 🚀 Version 0.9.1: The Surgical Update

We've just released v0.9.1, a major refinement that brings **surgical inline precision** to your editing workflow. This update bridges the gap between high-level structural editing and the need for microscopic changes within a single line.

### 🎯 Surgical Inline Editing
No more replacing entire lines just to change one variable! GnawTreeWriter v0.9.1 introduces column-aware editing (starting with Rust), allowing you to:
- **Edit a single parameter** in a dense function call without disturbing surrounding code.
- **Rename a variable** while preserving trailing comments on the same line.
- **Update types or values** with character-level accuracy within the AST.

### 💡 Pedagogical Validation
The **Duplex Loop** is now more than just a gatekeeper; it's a teacher. If an edit fails syntax validation, you get **language-specific tips** to help you or your agent fix the issue:
- **Rust**: Detects missing semicolons `;` or unbalanced braces `{}`.
- **QML**: Ensures properties have colons `:` and objects are correctly closed.
- **Python**: Checks indentation and colon placement.

---

## 🧠 GnawSense: AI-Powered Navigation & Action
Powered by **ModernBERT**, GnawSense transforms GnawTreeWriter from a precision editor into a **semantically-aware cognitive infrastructure**.

- **Semantic Search (`sense`)**: Search for logic by description (e.g., "how is backup handled?"). Includes *Satelite View* for project-wide discovery and *Zoom View* for file-specific focus.
- **Semantic Insertion (`sense-insert`)**: Insert code near a landmark without knowing its path. Just describe the anchor point (e.g., "after the login function") and let GnawSense find the correct AST position.
- **MCP Native**: All GnawSense features are exposed via the MCP server, enabling AI agents to navigate and edit your project autonomously and safely.

### 🏗️ Structural Scaffolding
Stop starting with empty files. Use the `scaffold` command to create new files with a predefined AST structure.
```bash
# Create a new Rust module with a struct and start function
gnawtreewriter scaffold src/network.rs --schema "rust:mod(name:server, struct:Config, fn:start)"
```
This ensures your files are syntaktically correct from the very first byte.

### 🛡️ TCARV Methodology Integration
GnawTreeWriter officially supports and enforces the **TCARV 1.0 (Text-Centric Architecture & Recursive Verification)** methodology.
- **Logic-First**: Define logic in text before writing code.
- **Anti-Lobotomy Policy**: Prevents agents from deleting complex code to fix build errors.
- **Git Surgery**: Bans "nuclear" rollbacks; encourages precise recovery from history.

---

### 📦 Installation Options

```bash
# Core only
cargo install --path .

# Full power (Recommended: includes GnawSense and MCP)
cargo install --path . --features modernbert,mcp
```

---

## 🛡️ The Structural Guardian

GnawTreeWriter isn't just a text editor; it's a **Structural Guardian** for your codebase. It monitors the "entropy" of your code during every edit, ensuring that AI agents (or human operators) don't accidentally perform a "lobotomy" on your logic.

- **Integrity Auditing**: Every edit is scored for structural loss. If a massive amount of logic or documentation is removed, The Guardian blocks the change.
- **The Duplex Loop**: GnawTreeWriter validates proposed changes against the AST *before* they touch your disk. If it's not valid syntax, it won't be applied.

## 📓 ALF: Agentic Logging Framework

To solve the problem of "Agent Amnesia," we built **ALF**. It's a structural journal that links high-level intent with low-level code changes.

- **Traceable Intent**: Why was this function changed? ALF knows.
- **Transaction Linking**: Every journal entry is tied to a specific `TransactionID` in the history.
- **Ecosystem Ready**: Designed to share knowledge with other tools like **GnawMimir**, creating a unified cognitive workspace.

## 📊 Engineering Case Studies

### Case 1: Preventing "Agent Lobotomy" (The Guardian)
*Scenario: An AI agent tries to "fix" a bug by deleting 40 lines of error handling logic.*
| Layer | Details |
| :--- | :--- |
| **Old State** | Complex function with nested `match` and `Result` handling. |
| **Agent Proposal** | Replacing the logic with a simple `unwrap()`. |
| **Guardian Action** | 🛑 **BLOCK**: Structural integrity check failed. |
| **Reasoning** | Significant complexity loss detected. Logic markers dropped from 12 to 1. |

### Case 2: Surgical Precision (v0.9.1 Update)
*Scenario: Changing a single parameter in a complex Rust function call.*
| Layer | Details |
| :--- | :--- |
| **Old Line** | `let res = process_data(config, true, timeout, "standard");` |
| **New Precision** | `gnawtreewriter edit file.rs "1.5.2" 'false'` |
| **Result** | `let res = process_data(config, false, timeout, "standard");` |
| **Benefit** | The rest of the line (config, timeout, etc.) remains untouched. |

---

## Key Features

- **AST-Level Precision**: Work at tree level, never worry about brackets again.
- **GnawSense**: Semantic navigation and editing via local AI.
- **Time Travel**: Project-wide restoration to any timestamp.
- **Atomic Multi-File Operations**: Coordinated edits with automatic rollback.
- **Multi-Language Support**: 26 programming languages (Python, Rust, TypeScript, JavaScript, C#, Dart, Svelte, SQL, Go, Java, C/C++, Kotlin, Swift, PHP, QML, HTML, CSS, YAML, TOML, XML, JSON, Markdown, Bash, Zig, and more).
- **Doctor Command**: `gnawtreewriter doctor` validates all parsers, backups, and transaction logs.
- **Verbose Mode**: `GNAW_VERBOSE=1` shows parser selection, node resolution, guardian scoring, and AST structural changes.
- **Structured JSON Errors**: `GNAW_JSON=1` gives machine-readable error output for AI agents.
- **Post-Edit AST Diff**: Automatic structural analysis after every edit — warns if important nodes are removed or changed.
- **Enhanced Error Context**: Parse errors now show the offending code line, language name, and actionable tips.
- **`explore`**: Zoomable, map-like codebase navigation — from project overview down to full file contents, with token counts at every level.
- **`pack`**: Package a whole project into a single AI-optimized context blob (markdown/json/plain/xml) with optional AST compression and secret redaction.
- **`curate`**: Pick only the files relevant to a task description instead of dumping the whole repo into context.
- **`compress`**: Shrink function bodies to placeholders (~70% token reduction) while keeping signatures intact.
- **`stats`**: Project-level token and file statistics for context-window planning.
- **`diff-to-batch`**: Convert any unified diff into a validated, rollback-safe batch operation.

## ⚡ Performance

GnawTreeWriter's AI-facing tools are fast — measured on its own 79-file codebase (169k tokens):

| Operation | Time |
|---|---|
| explore overview (level 0) | ~190 ms |
| explore directory (level 1) | ~130 ms |
| pack (no compress) | ~280 ms |
| pack (with compress) | ~335 ms |
| compress single file | ~100 ms |
| stats summary | ~180 ms |
| index_entities (20 files) | ~107 ms |
| index_relations (20 files) | ~40 ms |
| save_state / diff_since | 15–25 ms |

All operations complete in well under a second. Heavy AST workflows (pack, explore, indexing)
are **parallelized across CPU cores** with rayon while preserving deterministic output order.
Run the benchmark yourself: `bash scripts/benchmark_ai.sh`.

## 🧠 AI Context Tools

Built for the way LLMs actually consume code — pack, explore, curate, and compress turn
a whole repository into exactly the context an agent needs, no more, no less.

### Explore: map-like navigation

Four zoom levels take you from the big picture to the exact line:

```bash
# Level 0: project overview — directory tree with token counts
gnawtreewriter explore . --level 0

# Level 1: directory view — every file with its function/struct summaries
gnawtreewriter explore src/core --level 1

# Level 2: file structure — signatures of all top-level items
gnawtreewriter explore src/core/pack.rs --level 2

# Level 3: full source
gnawtreewriter explore src/core/pack.rs --level 3
```

Each node carries its token count and a drill-down hint, so agents (or humans)
can navigate a codebase without dumping it all into context.

### Pack: project → context blob

```bash
# Default markdown output with structure tree, summary table, and file contents
gnawtreewriter pack src --no-redact

# Compress large files to save context window (~70% per file)
gnawtreewriter pack src --compress

# Compress only files above 5000 tokens — small files stay fully readable
gnawtreewriter pack src --compress --compress-threshold 5000

# JSON, plain, or Repomix-compatible XML output
gnawtreewriter pack src --format json
gnawtreewriter pack src --format xml
```

Secrets are auto-redacted by default (AWS keys, GitHub tokens, JWT, and 15 more patterns).

### Curate: only what matters

```bash
# Let the task description pick the files
gnawtreewriter curate "implement user authentication" --max-tokens 4000

# Or pick by recency / git changes instead of relevance
gnawtreewriter curate "database refactor" --strategy recent
```

### Compress: shrink without losing structure

```bash
gnawtreewriter compress src/core/pack.rs --stats
# Replaces function bodies with ⋮---- placeholders — signatures stay intact
```

### Diff-to-batch: diffs become safe operations

```bash
git diff > changes.patch
gnawtreewriter diff-to-batch changes.patch --output batch.json   # preview
gnawtreewriter batch batch.json                                   # apply atomically
```

All of these are also available as MCP tools (`explore`, `pack`, `curate`, `compress`,
`diff_to_batch`, `index_entities`, `index_relations`) for agents working through
Claude Desktop, Zed, or VSCode.

## 🤖 AI Agent Integration (VS Code / Copilot)

GnawTreeWriter ships with built-in agent guidance so AI assistants **actively
choose AST edits over text-replace** — not just as one option among many.

**Best setup — VSCode Extension (works in all projects):**

Install the extension once, and GnawTreeWriter MCP is available in every
VSCode window automatically. Uses the same `mcpServerDefinitionProviders` API as
Gnaw Checkpoint and Gnaw Dokubase.

```bash
# Sideload from the repo:
cp -r extensions/vscode ~/.vscode/extensions/gnaw-software.gnawtreewriter-mcp
# Then Reload Window (Ctrl+Shift+P)
```

| File | Role |
|---|---|
| `extensions/vscode/package.json` | Extension manifest with `mcpServerDefinitionProviders` |
| `extensions/vscode/src/extension.js` | Registers `gnawtreewriter mcp stdio` globally |
| `.github/copilot-instructions.md` | Always-on policy: AST-first, text-replace as fallback |
| `.github/instructions/gnawtreewriter-edit.instructions.md` | File-scoped policy (triggers on `.py .rs .ts .go .cpp ...`) |
| `AGENTS.md` | Same rule for CLI agents (Opencode, Claude Code, Gemini CLI) |

**Prerequisite:** `gnawtreewriter` in PATH (`cargo install --path .`).

See [docs/EDITOR_INTEGRATION.md](docs/EDITOR_INTEGRATION.md) for full walkthrough.

The policy in plain English: *for code in AST-supported files, use
`mcp_gnawtreewrite_edit_node` / `semantic_edit` first; fall back to
`replace_string_in_file` only when the file type is unsupported or an AST edit
has failed twice.*

---

## Documentation

- **[TCARV Methodology](TCARV_1_0.md)** - The core process for AI development.
- **[AGENTS.md](AGENTS.md)** - Guidelines for AI agents.
- **[Editor Integration](docs/EDITOR_INTEGRATION.md)** - MCP setup + agent guidance for VS Code, Zed, Gemini CLI.
- **[MCP.md](docs/MCP.md)** - Detailed Model Context Protocol documentation.
- **[AI-Friendly Features](docs/AI_FRIENDLY.md)** - Token-aware analysis, compression, packing, curation.
- **[Performance Benchmark](scripts/benchmark_ai.sh)** - Measure explore/pack/index latency on your own codebase.
- **[ROADMAP.md](ROADMAP.md)** - Our journey towards v1.0.

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE) for details.

---

*Built with ❤️ and multi-agent collaboration.*

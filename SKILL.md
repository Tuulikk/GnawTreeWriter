---
name: gnawtreewriter
description: Use when the agent needs to edit code (change a function or node, batch edits, rename, undo an edit), diagnose or explore file structure (AST skeleton, analyze, list nodes), find where something is implemented (semantic search via sense/search_semantic), read a specific node, lint/fix code, check system health (doctor), or produce an AST-aware diff — in any project where GnawTreeWriter is installed. Triggers: "edit code", "where is X implemented", "file structure", "batch edit", "rename", "undo edit", "GTW", "gnawtreewriter", "ändra en funktion", "var är X implementerat", "struktur på filen", "lint", "doctor". Prefer GTW tools over plain text editing when the target is code.
---

# Skill: GnawTreeWriter 🌳✨

You are an expert in using **GnawTreeWriter** for surgical, AST-based code editing. Always prefer GnawTreeWriter over generic text editing tools when the target is code.

## 🚀 Core Mandates

1. **Tool-First Policy**: ALWAYS use GTW (`edit_node`/`semantic_edit`/`batch` MCP tools, or CLI) instead of plain Edit/Write tools when editing code
2. **Surgical Precision**: Target the smallest possible node — don't replace entire lines to change one variable
3. **Preview First**: ALWAYS use `preview_edit` / `--preview` before applying
4. **Time Machine Safety**: `undo` exists per transaction; use `session-start` for multi-step work
5. **Verify, don't assume**: GTW failures are LOUD (no-match replace, unknown rule id, empty skeleton all error with guidance) — read the error, it tells you the next step
6. **Semantic search**: prefer `sense` over `grep` when the project is indexed — then confirm with `read_node` (one cheap read beats re-searching)

## 🧭 Situations → Tools (start here)

| Situation | Tool (MCP → CLI) | Example call |
|---|---|---|
| "Where is X implemented?" (don't know file) | `sense` → `gnawtreewriter sense "query"` | `sense {"query": "how is undo implemented"}` |
| "What's in this file?" (shape, not bodies) | `get_skeleton` → `skeleton <file>` | `get_skeleton {"file_path": "src/cli.rs", "max_depth": 2}` |
| "Give me node paths" | `analyze` / `list_nodes` → `analyze`/`list <file>` | `list_nodes {"file_path": "src/cli.rs", "filter": "function_item"}` |
| Read exactly one block | `read_node` → (CLI: `edit --preview` prints diffs) | `read_node {"file_path": "a.rs", "node_path": "35.2.105"}` |
| Change a function/struct you located | `preview_edit` → `edit_node` → `edit <file> <path> '<code>'` | `edit_node {"file_path": "a.rs", "node_path": "1.2", "content": "fn x() {}"}` |
| "Change X" without knowing the node | `semantic_edit` / `semantic_insert` | `semantic_edit {"file_path": "a.rs", "query": "the backup init", "content": "..."}` |
| Add code at a structural spot | `insert_node` → `insert <file> <parent> <pos> '<code>'` | `insert_node {"file_path": "a.rs", "parent_path": "35.2", "position": 1, "content": "..."}` |
| Move/rename across locations | `move_node` → `move <src_file> <src_path> [tgt] <tgt_path>` | `move_node {"source_file": "a.rs", "source_path": "1.2", "target_path": "0"}` |
| Multi-file coordinated change | `batch` (ONE transaction) → `batch <file.json>` | spec below — `{file, path, content}` format! |
| Oops / bad edit | `undo` → `gnawtreewriter undo --steps N` | `undo {"steps": 1}` |
| Lint / find anti-patterns | `lint` → `gnawtreewriter lint <path> --recursive` | `lint {"paths": ["src"], "recursive": true, "severity_filter": "warning"}` |
| Auto-apply rule fixes | `lint {fix: true}` (+ `preview: true` first!) | `lint {"paths": ["src"], "fix": true, "preview": true}` |
| Write a rule | `add_rule` (pattern + optional `fix`) | `add_rule {"id": "proj_no_todo", "language": "rust", "pattern": "todo!()", "severity": "warning", "message": "..."}` |
| Is GTW alive/sane here? | `doctor` → `gnawtreewriter doctor` | `doctor {}` → `{healthy, passed, failed, checks[]}` |
| Build/refresh the search index satellite sense needs | `index_project` → `gnawtreewriter ai index` | `index_project {"action": "start"}` → poll `{"action": "status"}` (background, GPU gate applies) |
| "What did GTW change just now?" | `history` → `gnawtreewriter history` | `history {"limit": 5}` → transactions, newest first, stable ids for undo |
| "How big is this codebase?" | `stats` → `gnawtreewriter stats` | `stats {}` → files/lines/tokens/languages + largest files |
| Plain-language explanation | `explain` / `investigate` (mamba build) | `explain {"file_path": "src/core/batch.rs", "node": "1.2"}` |
| Find text inside one file | `search_nodes` → `search <file> "<pattern>"` | `search_nodes {"file_path": "cli.rs", "pattern": "handle_lint"}` |
| Snappy text replace (not AST) | (CLI) `quick-replace` — FAILS LOUD on no-match | `gnawtreewriter quick-replace f.rs 'old' 'new' --preview` |

**Diagnostic chain**: `analyze`/`get_skeleton` → `sense`/`search_nodes` → `read_node` → `preview_edit` → `edit_node`/`semantic_edit` → verify (`cargo check`/tests) → `undo` if wrong.

## 🆘 Fallback Rule (timeout or tool error)

1. **Don't retry blindly.** One timeout → fall back, note it, continue your task.
2. **Detour**: use `grep`/Read for the immediate need — GTW was supposed to save you a guess, not block you.
3. **LOG IT** in `GTW_MCP_ISSUE_LOG.md` (this project): what you called, exact params, what came back, what you used instead. The log is how GTW gets fixed — every past outage (timeout, bare header, lying undo) was found this way.
4. **Never cheat the policy**: no `git checkout`/`git restore` to undo GTW edits — use `undo`/`restore-project --preview`.
5. After a suspicious "success", verify bytes actually changed (`git diff`, re-read the node). Loud failure is normal; silent no-op is a bug worth logging.

## 📋 Standard Workflow (edit existing code)

```bash
# 1. structure + paths
gnawtreewriter analyze <file>            # or MCP analyze / list_nodes
gnawtreewriter search <file> "<pattern>" # exact node path

# 2. preview (ALWAYS)
gnawtreewriter edit <file> "<path>" '<new_code>' --preview   # or MCP preview_edit

# 3. apply
gnawtreewriter edit <file> "<path>" '<new_code>'             # or MCP edit_node

# 4. verify
cargo check    # or the project's own build/test
```

Add code: find parent via `list` → `insert <file> "<parent>" <pos> '<code>'`.
Multi-file: `session-start` → edits → `history` → `restore-session <id>` if wrong.

## 📦 Batch spec (correct format!)

`{type: "edit"}` uses **`file` + `path` + `content`** (node replacement), NOT `search`/`replace`:

```json
{
  "description": "Multi-file refactor",
  "operations": [
    {"type": "edit", "file": "src/file1.rs", "path": "1.2", "content": "fn new() {}"},
    {"type": "insert", "file": "src/file2.rs", "parent_path": "1.0", "position": 1, "content": "use x;"}
  ]
}
```

```bash
gnawtreewriter batch spec.json --preview   # diff only, writes nothing
gnawtreewriter batch spec.json             # atomic — all or none, one txn id
```

MCP: `batch {"file": "spec.json", "preview": true}`.

## 🧠 GnawSense setup (first time per project)

```bash
gnawtreewriter ai index                 # CPU default; --gpu / gnawtreewriter.yaml indexing.device=auto for GPU
gnawtreewriter sense "how is X done?"   # satellite (needs index)
gnawtreewriter sense "X" src/file.rs     # zoom (no index needed)
```

Empty satellite result = index missing → the error message says so; build it with `index_project {"action":"start"}` (MCP, background — poll with `"status"`) or `ai index` on the CLI.

**Reading satellite answers:** every response carries
`search_quality {prior_failures, suspect_reason, expansion}` —
`suspect_reason` (`no_lex_overlap`/`low_cosine`/`empty`) means the ranker
distrusts its own top hit, and `prior_failures` counts earlier failures of
the same query (log: `.gnawtreewriter_search_log.jsonl`). Do not trust a
flat score when these flag it — adjust the query or fall back.
`expand: true` (mamba builds) adds an LFM2.5 query-expansion channel:
`sense {"query": "...", "expand": true}`.

## 🛡️ Error handling (what GTW tells you)

| Error | What it means / do |
|---|---|
| `search text not found … nothing was written` | quick-replace no-match — re-read the file (it changed, e.g. fmt) and retry. Zero bytes touched. |
| `Node not found at path` | file changed — `analyze`/`list` again for fresh paths |
| `Validation failed` | your code has syntax errors; the message carries language-specific tips — fix, don't force |
| `unknown rule id …` | typo or rule not loaded — `rules list` / `gnawtreewriter rules list` |
| `get_skeleton … must never be read as a valid empty answer` | raise `max_depth`, or use `analyze`/`list_nodes` |
| `Zoom search: no nodes matched` | broaden the query, or `list_nodes` for raw structure |
| `Strict parse refused …` (`E_STRICT_PARSE`) | the EXISTING file has syntax errors — read paths (`analyze`/`get_skeleton`/`read_node`) still answer partially with `syntax_warning`; fix the file before editing |
| `doctor: N FAILED` | read `checks[]` — transaction/backup trouble → `restore-project --preview` |
| timeout / Not connected | **Fallback Rule above** — detour + log to `GTW_MCP_ISSUE_LOG.md` |

## 🔧 MCP tool reference (core)

Full schemas via `tools/list` — every description carries VAD/NÄR/RETURNERAR/EXEMPEL. Core set:

```
analyze {"file_path"}            list_nodes {"file_path", "filter"?, "max_depth"?}
get_skeleton {"file_path", "max_depth"?}       read_node {"file_path", "node_path"}
search_nodes {"file_path", "pattern"}          sense {"query", "file_path"?, "expand"?}
preview_edit {"file_path","node_path","content"}    edit_node {"file_path","node_path","content"}
insert_node {"file_path","parent_path","position","content"}
semantic_edit {"file_path","query","content"}   semantic_insert {"file_path","anchor_query","content"}
move_node {"source_file","source_path","target_path"?}
batch {"file","preview"?}        undo {"steps"?}
lint {"paths","recursive"?,"severity_filter"?,"pattern"?,"fix"?,"preview"?}
add_rule {"id","language","pattern","message"?, "fix"?}     doctor {}
history {"limit"?}                 stats {}
index_project {"action"?}          expand lives on sense (satellite only, mamba)
explain {"file_path","node"}     get_semantic_report {"file_path"}
```

CLI equivalents: `gnawtreewriter <analyze|list|skeleton|search|sense|edit|insert|delete|move|batch|undo|history|lint|rules|doctor|status|quick-replace|session-start> --help`.

## ✅ Status

Check health with **`gnawtreewriter doctor`** (or MCP `doctor {}`) — never trust a hardcoded version claim. Current release: see `CHANGELOG.md`.

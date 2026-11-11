---
name: gnawtreewriter
description: >-
  Load before the first code edit, including bug fixes, feature work and
  refactors. Use when the agent needs to edit code (change a function or node, batch
  edits, rename, undo an edit), diagnose or explore file structure (AST
  skeleton, analyze, list nodes), find where something is implemented
  (semantic search via sense/search_semantic), read a specific node,
  map a codebase or answer dependency questions (code_index: items, coverage,
  impact, cycles, --why), decide which files are worth reading (curate/pack),
  lint/fix code, check system health (doctor), or produce an AST-aware
  diff — in any project where GnawTreeWriter is installed. Triggers: "edit
  code", "where is X implemented", "file structure", "batch edit",
  "fix bug", "implement feature", "refactor", "rename", "undo edit", "GTW", "gnawtreewriter",
  "what breaks if I change", "who calls this", "dependency cycles", "code index",
  "which files matter", "blast radius",
  "ändra en funktion", "var är X implementerat", "struktur på filen", "vad påverkas om jag ändrar",
  "vem anropar", "beroenden", "lint", "doctor". Prefer
  GTW tools over plain text editing when the target is code.
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
7. **Structure before meaning**: `code_index` answers *where code sits and what depends on what* in ONE call; `sense` answers *what it means*; `grep` answers neither well. If you are about to grep to find a file or a dependency, use the index instead.
8. **Guard a reused path**: a numeric node path is a POSITION and shifts on every insert/delete. Pass `expect_name` (MCP) / `--expect-name <name>` (CLI) when the path came from an earlier call — GTW then REJECTS a stale path instead of silently editing the wrong node.

## 🧰 HOW TO CALL GTW (pick your host row — read this first!)

The tool **names are identical everywhere**; only the *calling convention* differs.
Find your host, use that row. If you are unsure: try the direct call first — if
the tool is in your catalog, it works (no wrapper needed).

| Your host | How you call it | Notes |
|---|---|---|
| **Cline** | `gnawtreewriter__<tool>` as a **native tool** — e.g. `gnawtreewriter__analyze`, `gnawtreewriter__edit_node` | Double underscore, no wrapper. Cline also loads this skill through its `skills` tool. |
| **VSCode / Copilot Chat** | `mcp_gnawtreewrite_<tool>` | The host truncates the server name; the suffix is the GTW tool name. |
| **OpenCode 2.x (Code Mode)** | `await tools.gnawtreewriter.<tool>({...})` **inside `execute`** | MCP tools are not direct tools there; discover with `search({ namespace: "gnawtreewriter" })`. Recipe below. |
| **Codex** | the `gnawtreewriter` MCP tools, if they appear in your catalog | Codex-specific variant: `.agents/skills/gnawtreewriter/SKILL.md`. A newly registered server may need a new session. |
| **Crush** | the GTW MCP tools if you registered the server in Crush's config; otherwise `gnawtreewriter <command>` | Crush reads `AGENTS.md` as project context — that path always works. The `gnawtreewriter-crush.skill` package is **generated** from this file (`scripts/pack-skill.sh`), never edit it. **UNTESTED 2026-11-11: `crush` is not installed on the dev machine, so no Crush session has ever run — see the status banner in `CRUSH_INTEGRATION_GUIDE.md`.** |
| **Terminal / no MCP at all** | `gnawtreewriter <command>` | Same names in CLI form. Shell escaping applies — pass content via `-` (STDIN) when it contains quotes/braces. |

**OpenCode 2.x only — the `execute` recipe** (skip this block on every other host):
calls below in this skill mean `await tools.gnawtreewriter.<name>({...})` inside
`execute`. Group discovery and preview, inspect the result, then write:

```js
// optional discovery: exact names + signatures for the 37 tools
const hits = search({ namespace: "gnawtreewriter", query: "edit" });

// discovery and reads can share one execute call:
const skel = await tools.gnawtreewriter.get_skeleton({ file_path: "src/cli.rs", max_depth: 2 });
const node = await tools.gnawtreewriter.read_node({ file_path: "a.rs", node_path: "35.2.105" });
const preview = await tools.gnawtreewriter.preview_edit({ file_path: "a.rs", node_path: "35.2.105", content: "fn x() {}" });
// Review the preview before a separate edit_node call with the same arguments.
```

- **No shell-escaping traps on this path** — content passes as native JS strings. The old
  double-brace/STDIN advice applies ONLY to the CLI fallback below.
- **GTW tools missing from your catalog?** Then — and only then — use the
  `gnawtreewriter` CLI for analyze → preview → edit. A host that HAS the MCP
  tools should never shell out: the CLI adds quoting traps for free. (`code-index`
  refresh is indexing work; it does not satisfy a code-edit request.)
- **`gnawtreewriter` namespace missing from the catalog?** The MCP server is down:
  `opencode api get /api/mcp` to confirm, then
  `opencode api post /api/experimental/mcp/gnawtreewriter/connect` to reconnect.
  Reconnected tools appear in the NEXT session — until then use the CLI binary
  (`gnawtreewriter <command>`, same tool names in CLI form), which bypasses Code Mode
  entirely and always works.
- **Char-boundary panic (å/ä/ö in the message)? Check the RUNNING binary's age first** —
  `gnawtreewriter --version` + `ls -l ~/.cargo/libexec/gnawtreewriter` vs the repo
  CHANGELOG. A stale installed binary can predate released UTF-8 fixes: on 2026-10-28
  a 0.18.0 daemon kept a bug alive for 3 weeks after it was fixed in source (the panic's
  file:line pointed INTO a comment that documented its own fix). Redeploy via
  `scripts/build-gpu.sh` (CUDA setup) or `cargo install --path .`, then reconnect the
  MCP server. Real panics in current source: fall back to the edit tool, log in
  `GTW_MCP_ISSUE_LOG.md`, move on.

## 🧭 Situations → Tools (start here)

Every MCP call = the execute recipe above: `tools.gnawtreewriter.<tool>({...})`. The CLI
column is the fallback when the MCP namespace is missing.

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
| Which tool fits this situation? (no skill loaded) | `guide` | `guide {"situation": "undo a bad edit"}` → {when, tool, example} |
| Is this file safe to edit? (syntax gate) | `validate` → `gnawtreewriter validate <file>` | `validate {"file_path": "src/main.rs"}` → {valid, nodes} eller E_STRICT_PARSE |
| Verify exactly what changed after a write | `diff` (samma kärna som `gnaw-diff`) | `diff {"old_file": "/tmp/before.rs", "new_file": "src/lib.rs"}` |
| Show what undo WOULD revert first | `undo {preview: true}` | `undo {"steps": 1, "preview": true}` → operations-lista, noll ändringar |
| **"Where does code sit?"** / what's in this module | `code_index` (query) → `code-index <root>` | `code_index {"mode": "query", "kind": "items", "path_prefix": "src/core"}` |
| **"What breaks if I change X?"** | `code_index kind=impact` → `code-index <root> --impact <name>` | `code_index {"mode": "query", "kind": "impact", "symbol": "parse_file", "depth": 3}` |
| Are there dependency cycles? | `code_index kind=cycles` → `code-index <root> --cycles` | `code_index {"mode": "query", "kind": "cycles", "depth": 20}` |
| Is this dependency edge REAL? (falsify it) | CLI `code-index <root> --why <name>` | per edge: `resolved`, `target_defs`, `verdict` (`unique_in_index` / `ambiguous_pick` / `unresolved`) |
| Can I trust this graph answer? | the `meta` block on every graph answer | `meta.index_stale: true` ⇒ run `code-index <root> --db` before planning |
| Which files should I read FIRST for this task? | `curate` → `gnawtreewriter curate "<task>"` | `curate {"task": "auth flow", "max_tokens": 5000}` |
| Give me the whole project condensed | `pack` → `gnawtreewriter pack . --compress` | `pack {"path": ".", "compress": true, "format": "markdown"}` |
| **Create a NEW file** (CLI + MCP) | `scaffold` → `gnawtreewriter scaffold <file>` | `gnawtreewriter scaffold src/new.rs --content - < body.rs` **or** `--from body.rs` **or** `--schema "rust:mod(name:auth)"`. Parse-validated before the write and logged as `Insert`. Note: `undo` **cannot** delete a created file yet. |
| **Repair a file GTW cannot parse** (CLI + MCP) | `heal` → `gnawtreewriter heal <file>` | `gnawtreewriter heal src/x.rs` = proposal; `--apply` = write, re-verified against the parser and logged. Handles unbalanced braces both ways; **never deletes code** (an extra `}` is reported with its line, not removed). |
| Who calls this function? (CLI impact answer) | (CLI) `gnaw-blast` — no MCP tool yet | `gnawtreewriter gnaw-blast <file> <node_path> --recursive` |

**Diagnostic chain** (editing one file): `analyze`/`get_skeleton` → `sense`/`search_nodes` → `read_node` → `preview_edit` → `edit_node`/`semantic_edit` → verify (`cargo check`/tests) → `undo` if wrong.

**Planning chain** (before you touch anything): `code_index query` (where) → `impact` / `cycles` (what depends) → check `meta.index_stale` → spot-check ONE edge with `--why` → then the diagnostic chain per file. A graph answer without `meta` or `--why` is a claim, not a fact.

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

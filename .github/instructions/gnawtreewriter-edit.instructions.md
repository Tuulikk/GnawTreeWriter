---
description: "Use when editing, refactoring, renaming, or inserting code in supported languages (Python, Rust, TypeScript, JS, Go, C/C++, Java, PHP, QML, YAML, etc.) — prefer GnawTreeWriter AST tools (mcp_gnawtreewrite_*) over text-replace for surgical, syntax-validated edits."
applyTo: "**/*.{py,rs,c,h,cpp,hpp,cc,cxx,hxx,h++,java,zig,ts,tsx,js,jsx,sh,bash,php,html,qml,go,css,yaml,yml,toml,xml}"
---

# GnawTreeWriter-first editing for code files

When you open or edit a code file matching this scope, treat GnawTreeWriter as
your primary editing tool. The MCP tools (`mcp_gnawtreewrite_*`) operate on the
AST, so changes are surgical and syntax-validated before the file is written.

## When to use GnawTreeWriter (default — do this)
- Editing a function, method, struct, class, enum, or any named code block
- Inserting a new node (function, field, statement, import) at a structural position
- Renaming or refactoring within a single file
- Reading a specific block without printing the whole file (token-efficient)
- Multi-step changes where you want preview + atomic application via `batch`

## When text-replace is acceptable (fallback only)
- The change is to prose, comments only, or non-code content
- The file type is **not** in the supported list (`.py .rs .ts .js .go .cpp .java .php .qml .yaml ...`)
- An AST operation has failed and you have retried once with corrected input

## Mandatory workflow for AST edits
1. **Verify the target** — if unsure of the node path, call `mcp_gnawtreewrite_list_nodes` or `mcp_gnawtreewrite_search_nodes` first.
2. **Target the smallest node** that contains the change. Replacing a whole function to change one line is the anti-pattern this tool exists to prevent.
3. **Preview non-trivial edits** with `mcp_gnawtreewrite_preview_edit` before applying.
4. **Pass code verbatim** to the MCP tool — no manual escaping of `{`, `}`, `$`, `"`. The MCP layer serializes correctly.

## Key tools at a glance
| Action | Tool |
|---|---|
| See structure (token-light) | `mcp_gnawtreewrite_get_skeleton` |
| List editable nodes | `mcp_gnawtreewrite_list_nodes` |
| Find a node by text/name | `mcp_gnawtreewrite_search_nodes` |
| Read a specific node | `mcp_gnawtreewrite_read_node` |
| Edit by AST path | `mcp_gnawtreewrite_edit_node` |
| Edit by description (fuzzy) | `mcp_gnawtreewrite_semantic_edit` |
| Insert new code | `mcp_gnawtreewrite_insert_node` |
| Move code across files | `mcp_gnawtreewrite_move_node` |
| Multi-file atomic change | `mcp_gnawtreewrite_batch` |
| Safe preview before apply | `mcp_gnawtreewrite_preview_edit` |

## Anti-pattern: silent corruption
`replace_string_in_file` on multi-line code can match the wrong occurrence or
break syntax invisibly (e.g., unbalanced braces after a partial replace).
`mcp_gnawtreewrite_edit_node` validates the AST before write, so a malformed
edit is rejected rather than committed.

## Don't loop on failure
If an AST edit fails:
1. Read the error (often a wrong node path or malformed code).
2. Verify the path with `list_nodes` or `search_nodes`.
3. Retry once with corrected input.
4. If it still fails, fall back to `replace_string_in_file` and note that you did.
Do **not** retry the same call more than twice.

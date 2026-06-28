# Project: GnawTreeWriter

This repository **is** GnawTreeWriter — a tree-based (AST) code editor. The tool
is available to you in this workspace as MCP tools prefixed `mcp_gnawtreewrite_*`.
Use it; do not ignore it.

## Editing policy: AST-first, text-replace as fallback

For code changes in this repository, **default to GnawTreeWriter** for editing.
Use text-based tools (`replace_string_in_file`, `insert_edit_into_file`) only as
fallback, when one of these is true:

- The file type is not AST-parseable (see list below)
- The change is purely prose, comments-only, or in a non-code file (README, CHANGELOG, docs)
- An AST operation has failed twice with corrected input and you must recover

### Why this rule exists
GnawTreeWriter validates syntax **before** writing, targets the smallest possible
node (no "shotgun" line replacements that accidentally hit the wrong location),
and provides preview + history. `replace_string_in_file` can silently corrupt
syntax on multi-line code edits and match unintended locations.

### Supported file types (AST-parseable)
`.py` `.rs` `.c` `.h` `.cpp` `.hpp` `.cc` `.cxx` `.hxx` `.h++` `.java` `.zig`
`.ts` `.tsx` `.js` `.jsx` `.sh` `.bash` `.php` `.html` `.qml` `.go` `.css`
`.yaml` `.yml` `.toml` `.xml` `.md` `.markdown`

For these types, prefer the AST tool. For anything else, text-replace is fine.

## Quick tool map (Copilot / MCP)

| Goal | Tool |
|---|---|
| See structure (low token) | `mcp_gnawtreewrite_get_skeleton` |
| List editable nodes | `mcp_gnawtreewrite_list_nodes` |
| Find a node by name/text | `mcp_gnawtreewrite_search_nodes` |
| Read a specific node | `mcp_gnawtreewrite_read_node` |
| Edit by AST path | `mcp_gnawtreewrite_edit_node` |
| Edit by natural-language query | `mcp_gnawtreewrite_semantic_edit` |
| Insert new code | `mcp_gnawtreewrite_insert_node` |
| Move code across files | `mcp_gnawtreewrite_move_node` |
| Multi-file atomic change | `mcp_gnawtreewrite_batch` |
| Safe preview before applying | `mcp_gnawtreewrite_preview_edit` |

## Non-negotiables
1. **Target the smallest node** that contains the change. Never replace an entire
   function to change one line.
2. **Preview non-trivial edits** with `mcp_gnawtreewrite_preview_edit` first.
3. **Verify the path exists** before editing: call `list_nodes` or `search_nodes`
   if you are not certain of the node path.
4. **Pass code verbatim** to MCP tools — do not pre-escape braces, dollars, or
   quotes. The MCP layer handles serialization.

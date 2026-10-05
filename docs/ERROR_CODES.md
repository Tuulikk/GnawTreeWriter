# GTW MCP error codes

Stable machine codes on `tool_error` responses (Motor2 plan #25).
Response shape: `{content: [{type: "text", text: <human guidance>}],
isError: true, code: <one of below>}`.

**Contract:** integrations aggregate on `code`; humans read the text.
Codes are append-only — never renamed or removed silently (the
`integration_error_strings_carry_guidance` test keeps docs and code in
sync: every `tool_error_code(..., "CODE")` must appear here).

| Code | Meaning | Typical next step |
|---|---|---|
| `E_FILE_NOT_FOUND` | File missing/unreadable at the given path | verify path (`explore`, `search_nodes`), retry |
| `E_STRICT_PARSE` | Editor refused to open the file: the EXISTING content has syntax errors | read paths (`analyze`/`get_skeleton`/`read_node`) still answer partially with the position — fix the file before editing |
| `E_NODE_NOT_FOUND` | node_path/source/target stale or not a parent | re-run `analyze`/`list_nodes` for fresh paths |
| `E_EDIT_REJECTED` | Validation/guardian rejected the write (syntax or error-severity rule) | fix the message's details, `preview_edit` first |
| `E_VALIDATION` | Proposed code failed syntax validation (model proposal) | adjust request and retry (`edit --ask` style flows) |
| `E_SEMANTIC_NO_MATCH` | semantic_edit/insert found nothing above the relevance floor | different anchor phrase, or `insert_node` with a `parent_path` from `list_nodes` |
| `E_MODEL_UNAVAILABLE` | Local model missing/disabled (feature gate or `ai setup` needed) | `gnawtreewriter ai setup`, rebuild with `modernbert` feature |
| `E_RULE_REJECTED` | Rule/fix template failed validation (nothing written) | fix pattern/`$NAME` bindings; `rules list` for ids |
| `E_BATCH_ROLLED_BACK` | Batch transaction failed and was fully reverted | fix operations in the spec; `preview: true` first |

Lines in this file are the registry: `tests/mcp_integration.rs` parses
`tool_error_code` call sites and asserts each code appears above.

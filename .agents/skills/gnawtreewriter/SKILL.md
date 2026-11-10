---
description: Use GnawTreeWriter for structural code edits and precise node reads in the GnawTreeWriter repository. Use text tools for prose or unsupported files.
name: gnawtreewriter
---
# GnawTreeWriter in Codex

Use the registered `gnawtreewriter` MCP server for edits to functions, types,
imports, and other code nodes. Find the target with `get_skeleton`, `analyze`,
or `search_nodes`; read it with `read_node`. Preview the replacement with
`preview_edit`, inspect the diff, then apply `edit_node` or `insert_node`.
Re-read the node or inspect `git diff` after writing. For coordinated changes,
preview and apply a `batch`.

If GTW tools are missing from this Codex session, use the installed
`gnawtreewriter` CLI for the same workflow. A newly registered MCP server may
require a new Codex session before its tools appear. Refreshing
`code-index.md` does not satisfy a requested code edit.

If an AST operation fails, inspect the error and correct the path or content.
After two corrected attempts, use the repository's documented text-tool
fallback and record the GTW failure in `GTW_MCP_ISSUE_LOG.md`. Verify bytes
after every reported successful write.

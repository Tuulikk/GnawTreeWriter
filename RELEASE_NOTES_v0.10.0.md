# GnawTreeWriter — Release Notes (v0.10.0)

**Date:** 2026-10-04
**Type:** Minor Release (new features, backward compatible)

## Summary

Phase 9 of the MCP adoption roadmap: GnawTreeWriter's MCP surface now
*teaches* agents how to use it, lints and fixes through MCP, and every
LLM answer carries verifiable source citations. Also root-caused and fixed
a test-infrastructure bug that silently reverted real edits on every test run.

## Changes

### New features
- **MCP `lint` tool** — full schema, ad-hoc `$X` pattern search, shared
  core with the CLI (report-only; never writes). CLI parity: `--pattern`,
  `--language`.
- **`fix:` rule support** — rules can carry rewrite templates that expand
  from pattern matches; `lint --fix` applies them as an atomic batch,
  `--fix --preview` shows the diff first. Validation before save
  (`validate_fix` with scaffold fallback). MCP: `add_rule {fix}` and
  `lint {fix, preview}`. `py_bare_except` ships with a builtin fix.
- **Adoption contract for all 30 MCP tools** — self-teaching descriptions
  (VAD/NÄR/RETURNERAR/EXEMPEL) + complete, dispatch-honest inputSchemas,
  pinned mechanically by `integration_mcp_tools_adoption_contract`.
- **`sources` provenance on LLM answers** — `get_semantic_report` cites
  `{file, node_path}` per finding; `investigate` cites the evidence files
  the answer was synthesized from. Verified by contract tests that run
  without a model.
- **`preview` on satellite sense matches** — score + content in one
  response; saves an extra `read_node` round-trip.

### Fixed
- **Every `cargo test` run silently reverted the latest real edit**
  (`integration_mcp_tools_call_undo` operated on the repo's actual
  transaction log). New `serve_with_shutdown_root` binds tests to an
  isolated project root; regression-proofed with consecutive green runs
  and unchanged checksums. See GTW_MCP_ISSUE_LOG.md findings 12–13.
- **Batch integration test** used a nonexistent spec format since its
  creation (`replace` → real format `{file, path, content}`).
- **`--features mamba` build** was broken (pre-9.2 `Rule` initializer).

### Docs
- `GTW_INSTRUCTIONS.md` regenerated (30 tools + diagnostic chain),
  README Rules Engine + MCP lists updated, ROADMAP Phase 9 (9.1–9.4)
  marked complete, issue log entries for findings 1–13.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
```

No breaking changes: existing rules files work as before; `fix:` is
optional. MCP clients reconnecting will see richer tool descriptions
and the two new `lint`/`fix` capabilities automatically.


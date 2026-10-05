# GnawTreeWriter — Release Notes (v0.15.0)

**Date:** 2026-10-05
**Type:** Minor Release

## Summary

Everything from the Motor2 integration menu: the missing MCP index tool,
history/stats surfaces, semantic-match transparency, stable error codes
agents can aggregate on, persisted duplex metrics with a one-round
repair retry — plus partial-grace parsing so a localized syntax error
never voids a file for read paths (editors stay strict).

## Changes

### New features
- **`index_project` MCP tool** — builds the vector index satellite
  `sense` searches (background start/status; the graph-extracting
  `index_entities` now says so explicitly). Fixes the "indexed via MCP
  but sense says no matches" trap.
- **`history` + `stats` MCP tools** — transaction log and ProjectStats
  over MCP, with empty/normal-state guidance.
- **`semantic_match` transparency** — confidence + top-5 candidates on
  semantic_edit hits, explicit zero on misses; structured anchor data
  on semantic_insert.
- **Stable error codes** — `code` field on tool errors
  (`E_STRICT_PARSE`, `E_EDIT_REJECTED`, `E_SEMANTIC_NO_MATCH`, …),
  registry in `docs/ERROR_CODES.md`, sync-tested.
- **Duplex metrics** — `.gnawtreewriter_metrics.json`
  (proposed/validated/rejected/applied) for cross-session rates.
- **edit_ask repair round** — AST validation errors are fed back to the
  model once; `retried: true` + `first_error` surface the outcome.
- **Partial-grace reads** — analyze/skeleton/list/search/read return
  partial trees + `syntax_warning` for broken files; strict editors
  refuse with `E_STRICT_PARSE` and write nothing.

### Fixed
- Syntax refusals miscoded as missing files (E_STRICT_PARSE split out).
- preview_edit failures no longer claim "sense failed".
- Relation-count sanity cap scales with file size.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
# or GPU-enabled:
scripts/build-gpu.sh
```

No breaking changes: new tools, new optional response fields
(`code`, `semantic_match`, `syntax_warning`, `retried`), new metrics
file.


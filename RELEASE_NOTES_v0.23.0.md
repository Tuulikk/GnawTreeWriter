# GnawTreeWriter — Release Notes (v0.23.0)

**Date:** 2026-11-04
**Type:** Minor Release (edit/search reliability — Motor2 rapport B1–B5 handled)

## Summary

This release closes the B1–B5 bug classes from `GTW_RAPPORT_2026-11-01.md` —
the reliability gaps that bit real agent sessions (including the
2026-11-03 incident, which was B1's cost in practice):

1. **B1 path freshness** — `edit_node` gains `expect_name` / `expect_kind`;
   a stale positional path now rejects with `E_PATH_STALE` instead of
   writing the wrong node, and success receipts carry `new_node_path` +
   `node_content_hash` for the next edit.
2. **B2 partial-replacement guard** — an edit that would leave the OLD
   node's content below the new one (duplicate definitions downstream,
   tool said "success") is rejected loudly; nothing is written.
3. **B3 skeleton outline** — `get_skeleton` / `skeleton` default to a
   named-structure outline; comments, uses and attributes collapse into
   a count. `--raw` / `named_only: false` restores the full dump.
4. **B4 search alternation** — `search_nodes` pattern `"alpha|beta"` is
   OR: results merge deduped with `matched_pattern` naming the hit;
   empty alternations error loudly instead of a silent 0-hit answer.
5. **B5/T4** — capability ladder surfaced in `doctor` and
   sense/search_semantic responses (`level`); NO-OP edits are
   `Ok(changed: false)` instead of an error.

## Changes

### Fixed

- **B4**: `search_nodes` (MCP + CLI) splits the pattern on `|`; each
  alternative is a substring match, results merge deduped by node path
  and every match names `matched_pattern`. An alternation with no
  non-empty alternatives errors loudly ("Empty search pattern") instead
  of the silent 0-hit answer B4 reported. Grammar documented in the
  tool schema + GTW_INSTRUCTIONS.
- **B2**: remnant detector (`detect_replacement_remnant`) runs after
  every `edit_node`; on detection the edit is rejected loudly, nothing
  written, B2 class named. Lines already living OUTSIDE the node's span
  in the original are other code, never remnants (the false-positive
  class the first detector version had — caught by the guardian
  reduction test). Dogfood: the full funnel test-suite (300+ edits)
  runs through the guard without a single false positive; the crafted
  B2 signature fires.

### Added

- **B1a**: `EditReceipt` gains `new_node_path` + `node_content_hash` —
  the addressed node re-resolved in the fresh tree after write, with
  its content hash (the freshness key for consumers replaying stored
  paths).
- **B1b**: `edit_node` gains `expect_name` / `expect_kind` — declared
  identity checked BEFORE any write; mismatch rejects with
  `E_PATH_STALE` and the guidance "re-run search_nodes". Works with
  numeric dot-paths and `@kind:name` anchors (anchors are
  self-validating; the guard targets positional paths — the replay
  case). `GnawTreeWriter::expect_node` exposes the same guard to
  library callers.
- **B3**: shared outline builder (`core::skeleton_outline`) — CLI and
  MCP can no longer diverge. MCP payload gains `named_only` +
  `collapsed`.

### Tests

- `tests/mcp_integration.rs`: B4 alternation (finds both symbols with
  dedup + `matched_pattern`), single patterns unchanged, empty
  alternation errors.
- `tests/b1_path_validation.rs` + MCP integration test: receipt fields,
  mismatch rejects before writing, the 2026-11-03 incident shape is
  caught by an expect declaration.

## Upgrade Instructions

No breaking changes for library consumers of existing fields — the
receipt gains two additive fields and `edit_node` two optional
parameters. Re-run `cargo install --path .` to get the new binary:

```bash
cargo install --path .
gnawtreewriter --version   # gnawtreewriter 0.23.0
```

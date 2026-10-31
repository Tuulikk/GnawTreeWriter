# GnawTreeWriter — Release Notes (v0.21.0)

**Date:** 2026-10-31
**Type:** Minor Release (features + guardian fix)

## Summary

Three agent-facing features land, together with the fix for a guardian
false-positive that blocked valid edits inside large body-blocks
(Motor2 relay, issue-log (IV)):

1. **`undo --preview` in the CLI** — see before restore, same
   peek_undo core as the MCP tool; zero bytes, consumes nothing from
   the stack.
2. **Semantic channels degrade instead of dying** — `sense` /
   `search_semantic` (MCP) and `sense` (CLI) now return lexical text
   matches with an honest "DEGRADED" label when the bge-base model or
   index is missing; `doctor` reports the channel state (pass/warn).
3. **Guardian correctness + MCP force** — additions inside body-blocks
   are no longer misread as removals (set-membership pairing replaces
   positional pairing in `edit_delta::diff_walk`), and MCP `edit_node`
   gains an optional `force` parameter so the CLI and the agent path
   share the same deliberate last resort.

## Changes

### Fixed

- **Guardian false-positive on body-block additions**: positional child
  pairing shifted on top-insertions, so the token fallback reported
  "Statement node removed" and blocked valid edits — while the same
  content appended at the block END passed (pure-ASCII additions were
  blocked too: a pairing bug, not an encoding one). Children now match
  by (node_type, trimmed content) first; leftovers pair by order.
  Genuine removals still classify — the regression suite includes a
  negative control deleting a real condition statement.
- **MCP `edit_node` gains `force`** (default false): rejection guidance
  now names both routes: CLI `--force`, MCP `force: true`.

### Changed

- **Semantic degradation (T5)**: with model files or index missing,
  `sense`/`search_semantic` answer "Semantic channel unavailable
  (reason) — DEGRADED to lexical text match (NO semantic ranking)"
  with the matched nodes and the fix path; `GNAW_JSON` mode returns
  structured degraded JSON; `doctor` gained `ai_runtime/semantic_model`
  (pass/warn + guidance). Motivated by four repeat reports
  (2026-10-18..10-29).

### Added

- **CLI `undo --preview` (T3)**: lists each operation it would revert
  (timestamp, op kind, file, node path, description, txn id), writes
  ZERO bytes, and does not consume the undo stack (proven by test).

### Polished

- **Full fmt + clippy sweep**: `cargo fmt --check` and
  `cargo clippy --lib --tests -- -D warnings` are both clean
  repo-wide; the 9.1-era test-module debt is closed.

## Upgrade Instructions

- **Path-dependency consumers (Motor2 m.fl.)**: no API-signature
  changes; the guardian-pairing fix and the added optional `force`
  parameter are additive. No `### BREAKING (lib)` items in this
  release.
- Rebuild/restart MCP hosts to pick up the new binary
  (stale-daemon lesson 2026-10-28).
- The Motor2 (IV) workaround ("append at the block END") is no longer
  necessary; `list_nodes` `node_path`-scope remains OPEN as a
  sub-function request (see `GTW_MCP_ISSUE_LOG.md`).

## Validation

- `cargo test --lib`: 272 green (incl. 4 guardian-alignment
  regression tests + 3 lexical-degrade tests + T1/T2/T3 tests)
- `cargo test --test insert_position`: 14 green
- `cargo clippy --lib --tests -- -D warnings`: clean (repo-wide)
- `cargo fmt --check`: clean (repo-wide)
- Evidence: `GTW_MCP_ISSUE_LOG.md` (2026-10-31 entries I–IV),
  `docs/CODE_INDEX_SQLITE_DESIGN.md` (design only, no behaviour change)

## Known issues (open, tracked)

- T4 (NO-OP contract: Ok + `applied=false` instead of Err) — deferred,
  coordinated with Motor2's wrapper update
- T6 (canonical relation-ID format) — deferred, coordinated (blocks
  code-index SQLite Phase 3)
- `list_nodes` node_path scope (sub-function, (IV)/C)

# GnawTreeWriter — Release Notes (v0.14.0)

**Date:** 2026-10-05
**Type:** Minor Release

## Summary

Semantic search finds implementations instead of declarations, and
indexing runs an order of magnitude fewer model forwards — safely on
GPU, with a budget that can no longer exhaust VRAM.

## Changes

### New features
- **Satellite reranking** — `sense` (satellite) now serves
  implementation-nodes first: declarations demoted by a query-scaled
  prior, implementations nudged, lexical matches rewarded across
  preview + paths, chunk prefixes classified correctly, raw window
  widened so long bodies reach the reranker. Inventory queries ("which
  modules…") flip the priors back to declarations. Reference query
  "how is undo implemented…": real impls/fn on top (was: 10/10 module
  decls and attributes).
- **Batched embeddings** — `get_embeddings` embeds up to 16 texts per
  forward (padded, masked mean-pool), order-preserving and row-equivalent
  to the sequential path. Used by the project indexer (two-phase walk +
  batch) and both zoom-JIT loops: forwards go from N to ~N/16.

### Fixed
- **CUDA out-of-memory during GPU indexing** — budget-aware greedy
  batching (`B × max_len² ≤ 2^22`) replaces flat B=16; giant
  definition-body rows fall back to B=1. Full GPU reindex after the
  fix: 223 files / 90 s / exit 0.

### Changed
- Heavy ModernBERT equivalence test marked `#[ignore]` (run with
  `-- --ignored --nocapture`; prints sequential-vs-batch timing).
  Short-text debug micro-bench: 1.1-1.2× — the structural win is
  fewer forwards, launch reduction and OOM safety, not the debug ratio.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
# or GPU-enabled:
scripts/build-gpu.sh
```

No breaking changes. Existing semantic indexes stay valid (embedding
math unchanged for single texts); a fresh `ai index` picks up the
batched path automatically.


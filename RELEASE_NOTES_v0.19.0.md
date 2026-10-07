# GnawTreeWriter — Release Notes (v0.19.0)

**Date:** 2026-10-07
**Type:** Minor Release

## Summary

Semantic search (`sense`) became measurably better across the board: a
third (graph-proximity) reranking channel, diversification that no
longer drops the exact symbol you asked for, and index content that
gives the embedding model something to work with. On the 50-case eval
set the pipeline went from **60/64/70/72% recall @ k=5/10/20/100** to
**58/68/92%**, recall@1 from 20% to 30%, MRR 0.351 → 0.437 — and the
pipeline now **beats raw cosine similarity at the tail** (92 vs 90 @
k=100), which is what a hybrid reranker is supposed to do.

## Changes

### Added
- **Graph-proximity rerank channel** — call-graph adjacency of the top
  seed files (callers + callees) fused as a bounded reciprocal-rank
  term; hubs (>100 resolved edges) excluded; silently degrades to the
  two-channel ranking without graphs.
- **Recall-eval diagnostics** — per-case `expected_cosine_rank`,
  `expected_pipeline_rank` (uncapped) and `top_pipeline_files`, the
  calibration dataset that separates "model never surfaced it" from
  "the reranker pushed it out".
- **Headline-zone diversification** — the per-file cap (4) applies
  only to the first 5 served slots; deferred entries lead the tail
  pass, so nothing in the served window is dropped. 9/12 tail misses
  were exact symbols crowded out by their own file's heads.
- **Preview depth 97 → 240 chars** — the embedder sees real context
  instead of bare signature heads; raw recall@1 16% → 24%.
- **Module-doc suffix in embeddings** — each entry embeds its file's
  `//!` documentation appended: the file's own description bridges
  agent queries to symbols (recall@100 90 → 92%, recall@1 30%).

### Fixed
- Restored the pre-graph fused sort that was lost during a debug
  removal — queries without graph adjacency were served in raw cosine
  order (lex channel and priors silently ignored).

## Upgrade Instructions

1. Pull and rebuild (`cargo install --path .`, or
   `scripts/build-gpu.sh` for the CUDA build).
2. **Re-index**: `gnawtreewriter ai index` — preview depth and the
   module-doc suffix change every stored vector.
3. No API breaks: MCP/CLI signatures and `rerank_satellite` are
   backward compatible. Lib consumers on 0.18: nothing removed, only
   additions (`rerank_satellite_with_graphs`, eval diagnostics).

## Verification

- 236 lib tests green, clippy clean, CI (Validate / Test AI Features /
  Test MCP Examples) success on the release commit's parent.
- Eval: `gnawtreewriter ai recall-eval evals/sense_recall.json`

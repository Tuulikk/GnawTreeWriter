# GnawTreeWriter — Release Notes (v0.18.0)

**Date:** 2026-10-06
**Type:** Minor Release (new features, lib-breaking changes called out)

## Summary

Semantic search got a working retrieval model, a measurable quality
baseline and a real storage engine. A recall@k harness
(`ai recall-eval`) proved the old embedding backend (raw ModernBERT
MLM checkpoint) returned **recall@5 = 0%**; switching to a
retrieval-tuned model (BAAI/bge-base-en-v1.5) brought it to
**54.2%** with MRR 0.30. The vector index moved from ~90 MB of
per-file JSON shards to a single SQLite database with atomic per-file
upserts — fixing the stale-entries (orphan) defect — with automatic
one-time migration. This release also carries the rest of Guardian v2
(structured `edit_verdict`, impact reports via the knowledge graph,
resolution observability, import-aware call narrowing).

## Changes

### Semantic search (BGE + recall harness + SQLite)
- Embedding backend switched to **BAAI/bge-base-en-v1.5** (CLS
  pooling, L2-normalized, BGE v1.5 query prefix on the query side
  only). Legacy MLM path kept for old model dirs.
- **`ai recall-eval <eval-set>`** — recall@1/recall@k/MRR harness;
  `evals/sense_recall.json` ships 24 baseline cases.
- **SQLite index storage** (`embeddings.db`, WAL, bundled rusqlite):
  8× smaller on disk, faster cold loads, automatic idempotent
  migration from legacy JSON shards, and `save_index` now deletes a
  file's previous rows — no more stale entries after re-indexing.
- 512-token truncation on the embedder tokenizer.

### Guardian v2 (since 0.17.0, included here)
- Structural deltas, invariant contracts, no-op guard + edit receipts,
  structured `edit_verdict` on all rejection paths, impact reports
  (`impact: {symbol, callers, sites}`) sourced from the knowledge
  graph, resolution observability (same-file precedence, import-aware
  narrowing, `unresolved: N` reporting, doctor check).

## BREAKING (lib)

- `AiModel` gained `Bge`; `ModernBertModel.model` replaced by
  `backbone: EmbeddingBackbone` (`ModernBertMlm` | `Bge`).
- `IntegrityReport` gained `deltas: Vec<EditDelta>`.
- `Batch` gained `last_verdict` / `impacts` RefCell fields.
- `FileGraph` gained `imports` (`#[serde(default)]` — old graph JSON
  stays loadable).

Motor2 and other path-dependents: bump the path dep and fix exhaustive
matches on `AiModel`/`EmbeddingBackbone`; embed via `get_embedding` /
`get_query_embedding` — never construct `ModernBertModel` literals.

## Upgrade Instructions

1. `cargo install --path .` (or `scripts/build-gpu.sh` for GPU
   indexing).
2. Let the model download: `gnawtreewriter ai setup` (fetches
   BAAI/bge-base-en-v1.5, ~440 MB).
3. **Re-index** — old MLM vectors are useless:
   `rm -rf .gnawtreewriter_ai/index && gnawtreewriter ai index`
   (the SQLite migration also runs automatically on first load if you
   keep the old shards; either way the vectors must be re-computed).
4. Optional (developers): set `indexing: { device: auto }` in
   `gnawtreewriter.yaml` with a CUDA build — full src/ indexing drops
   from hours to ~20 s. GPU is never default.

## Verification

- 234 lib tests + 24 MCP integration tests green
- `cargo clippy --lib/--tests` clean; `--no-default-features
  --all-targets` clean (cfg-hole fix for `recall-eval` + BGE types)
- Recall A/B pre/post SQLite migration: identical (12.5 / 54.2 / 0.299)

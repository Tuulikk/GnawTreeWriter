# GnawTreeWriter — Release Notes (v0.16.0)

**Date:** 2026-10-05
**Type:** Minor Release

## Summary

Semantic search got a feedback loop: it now says when it does NOT trust
itself, remembers how often a query failed before, and can expand the
query through the local LLM for a second semantic channel.

## Changes

### New features
- **`sense {expand: true}`** (satellite, opt-in): LFM2.5 expands the
  question into content terms; the expanded text is embedded as a
  second channel fused with the original by max cosine and feeds the
  reranker's lexical side. One LLM call + one embedding; non-mamba
  builds get an explicit skip note. Live: 7.4 s end-to-end with terms
  like `transaction log / persistence / entry storage`.
- **Search feedback loop** (AUTO-koppling): every satellite answer
  carries `search_quality {prior_failures, suspect_reason, expansion}`.
  Suspicious outcomes (`empty`, `low_cosine`, `no_lex_overlap`) are
  appended to `.gnawtreewriter_search_log.jsonl` (gitignored, rotated
  at 1 MB), and `prior_failures` — earlier failures of the same
  normalized query — appears in success notes and no-matches errors so
  agents can adjust instead of trusting a flat score.

### Fixed
- `edit --ask --all --preview` ignored `--preview`; preview now always
  wins (even with `--force`).

### Verified
- 232 tests green, clippy `-D warnings` clean on default AND mamba
  builds, fmt clean; feedback loop + expansion live-tested against the
  real index and models.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
# for expansion (expand: true):
cargo install --path . --features modernbert,mcp,mamba
# or GPU-enabled:
scripts/build-gpu.sh
```

No breaking changes: new optional request field (`expand`) and new
optional response field (`search_quality`).


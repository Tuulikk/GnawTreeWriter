# GnawTreeWriter — Release Notes (v0.9.8)

**Date:** 2026-10-01
**Type:** Patch Release (bug fixes)

## Summary
Reliability release born from real-world dogfooding: an MCP usage anomaly
logged in `GTW_MCP_ISSUE_LOG.md` was traced to five root causes and fixed,
and a systematic UTF-8 audit uncovered seven more latent byte-slice bugs.

## Highlights
- **MCP survives anything:** every request runs in an isolated task — a
  panicking handler returns a JSON-RPC error instead of killing the
  connection (the "Not connected" failure mode is gone).
- **Large files work:** `sense`/`get_skeleton` on 1000+ node files complete
  within client timeouts. Embedding caps (24 nodes × 1200 chars) keep first
  JIT calls fast; the model and index cache persist across calls via a
  shared broker (first call ~25-30 s, then ~1 s).
- **UTF-8 safe by construction:** char-boundary panics (em-dash, CJK, emoji)
  fixed across sense, pipeline, project indexer, CLI previews and secrets
  redaction. New lint rule `no_byte_slice_strings` guards the pattern.
- **Bounded memory:** JIT cache capped at 32 files (was unbounded — 511 MB
  after 5.5 h).
- **Honest search responses:** empty satellite search now explains itself
  (match count + "build the project index" hint) instead of returning a
  bare "Satelite search results" header.

## Validation
- 8/8 test suites green, `cargo clippy -- -D warnings` clean, cfg-parity
  checked (with and without `modernbert`).
- Reproduction case (1116-line Rust file): timeout + panic before →
  23.6 s success after; follow-up skeleton call 0.1 s.
- Full narrative in `GTW_MCP_ISSUE_LOG.md` (omgång 3-5).

## Upgrade Instructions
Rebuild/install as usual: `cargo install --path .` Restart any running
`gnawtreewriter mcp stdio` processes so hosts pick up the new binary.

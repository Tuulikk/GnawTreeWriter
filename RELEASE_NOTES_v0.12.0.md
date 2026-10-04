# GnawTreeWriter — Release Notes (v0.12.0)

**Date:** 2026-10-04
**Type:** Minor Release (new features, backward compatible)

## Summary

ROADMAP 9.5 complete: every open item from the MCP issue log is either
fixed or explicitly closed with a verdict — plus the tools that make GTW
self-diagnosing and honest. A `quick-replace` that could report success
without changing a single byte is gone; nothing in GTW lies about
results anymore.

## Changes

### New features
- **MCP `doctor`** — one-call health diagnostic (parser smokes, backup
  integrity, transaction log): `{healthy, passed, failed, warnings,
  total, checks[]}` + guided text. Shared core with the CLI so they can
  never diverge. Made practical by a perf fix: **80 s → 0.5 s**
  (light backup scan instead of parsing 2 GB of backups; parallel
  parser smokes).
- **Lint rule `rust_string_byte_slice`** with a rewrite fix
  (`$X[..$Y]` → `$X.get(..$Y).unwrap_or($X)`) — the leftover byte-slice
  class from the UTF-8 audit, smoke-proven on real hits and through
  `lint --fix --preview`.
- **Rule messages interpolate `$X` bindings** — findings name the actual
  code (`if line is a String/&str`), not the template.

### Fixed
- **`quick-replace` no-op lie (finding #14)**: searching text that isn't
  there now fails loudly with next-step guidance instead of printing
  "✓ applied" over zero changed bytes. Same for an identical replacement.
- **`get_skeleton` bare-header answers**: skeleton in the text channel,
  explicit `truncated` flag, guided error for empty — never silent.
- **`sense` header-only answers**: top matches in the text channel for
  zoom and satellite; empty zoom guides instead of showing a label.
- **Unknown lint rule ids** now fail loudly instead of reading as
  "No issues found".
- Triage checklists from 2026-09-30/10-01 all closed with verdicts.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
# or, GPU-enabled (unchanged from 0.11.0):
scripts/build-gpu.sh
```

No breaking changes: new MCP tool (`doctor`), new rule id, stricter
errors on previously-silent failures (a no-match `quick-replace` now
exits non-zero — that is the point).


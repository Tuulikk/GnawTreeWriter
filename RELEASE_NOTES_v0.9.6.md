# GnawTreeWriter — Release Notes (v0.9.6)

**Date:** 2026-08-24
**Type:** Minor Release

## Summary

v0.9.6 is the "AI context" release — it makes GnawTreeWriter dramatically faster at the
workflows AI agents use most (packing, exploring, indexing) and adds new tools for
navigating and compressing codebases.

## Highlights

### ⚡ Parallel Processing (rayon)

The heavy AST workflows now run across all CPU cores while preserving deterministic
output order — results are byte-identical to the sequential versions:

| Operation | Before | After |
|---|---|---|
| `pack --compress` | 676 ms | 335 ms (-50%) |
| `explore` directory (level 1) | 318 ms | 130 ms (-59%) |
| `index_entities` (20 files) | 177 ms | 107 ms (-40%) |
| `index_relations` (20 files) | 84 ms | 40 ms (-52%) |

Measured on GnawTreeWriter's own codebase (79 files, 169k tokens).

### 🗺️ `explore` Command

Map-like navigation with four zoom levels — from project overview down to full file
contents, with token counts at every level:

```bash
gnawtreewriter explore . --level 0            # overview: directory tree + tokens
gnawtreewriter explore src/core --level 1     # directory: per-file summaries
gnawtreewriter explore src/core/pack.rs --level 2  # file: structural signatures
gnawtreewriter explore src/core/pack.rs --level 3  # full source
```

### 🧠 Session-Level Parse Cache

Files parsed once are reused across tool calls in the same session — no redundant
AST parses when multiple tools touch the same file.

### 📦 `pack --compress-threshold N`

Compress only files larger than N tokens. Small files stay fully readable, big files
get the ⋮---- treatment — useful for huge repos.

## Changes

### Added
- `explore` command with 4 zoom levels
- Session-level parse cache (`parse_cache.rs`)
- `pack --compress-threshold N` flag
- `scripts/benchmark_ai.sh` — reproducible performance benchmark

### Changed
- Parallelized pack (all 4 output formats), explore directory level, and MCP
  `index_entities` / `index_relations` batch handlers with rayon
- README and CHANGELOG expanded with the new features and benchmark numbers

## Upgrade Instructions

```bash
cd GnawTreeWriter
git pull origin master
cargo install --path .
gnawtreewriter --version  # Should show 0.9.6
```

## Testing

All tests passing:

```
cargo test
# test result: ok. 147 passed; 0 failed (lib) + integration suites
```

`cargo clippy --all-targets` is clean (0 errors).

## Acknowledgments

Thanks to the AI agent collaborators who benchmarked, optimized, and documented
this release — including the parallelization work that cut pack time in half.

# GnawTreeWriter — Release Notes (v0.11.0)

**Date:** 2026-10-04
**Type:** Minor Release (new feature, backward compatible)

## Summary

Opt-in GPU acceleration for semantic indexing — conservative by default.
GnawTreeWriter stays a CPU-first tool: the GPU is only ever used when you
explicitly ask for it, and even then only when at least 20% of VRAM is free.

## Changes

### New features
- **`indexing.device` in `gnawtreewriter.yaml`** (`auto` | `cpu` | `cuda`) —
  the one flag that controls whether `ai index` may use the GPU. Absent
  file/setting = CPU. Invalid values fail safe to CPU.
- **`ai index --gpu`** — one-off opt-in for a single run (overrides the file).
- **20%-VRAM safety gate** — `auto` picks the GPU only when built with
  `--features cuda` AND ≥20% of VRAM is free; never squeezes the desktop or
  other GPU workloads. Every device decision is logged with its reason.
  Query paths (sense, zoom, MCP) always stay on CPU.
- **`scripts/build-gpu.sh`** — one-command GPU build for any Linux box with
  podman + NVIDIA driver: compiles inside an `nvidia/cuda` container (no host
  CUDA toolkit needed — works on Fedora), auto-detects your GPU's compute
  capability, ships exactly the missing runtime libraries, installs
  binary + wrapper. Warm rebuilds ~2 minutes (cached volumes).

### Docs
- AGENTS.md: "GPU indexing (opt-in; devs recommended)" — recommendation for
  developer machines is changing the one flag in the file.
- README: "GPU-accelerated indexing (optional, off by default)" section.
- GTW_MCP_ISSUE_LOG.md: GPU entry + finding #14 (`quick-replace` reported
  success while changing zero bytes — logged as a GTW tool bug with the
  guidance principle: fail loudly, never fake success).

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp   # CPU build (unchanged)
# or, for GPU indexing:
scripts/build-gpu.sh
```

No breaking changes: without `gnawtreewriter.yaml` or `--gpu`, behavior is
identical to previous releases (CPU-only). Existing rules files are untouched.


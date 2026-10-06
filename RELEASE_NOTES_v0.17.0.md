# GnawTreeWriter — Release Notes (v0.17.0)

**Date:** 2026-10-05
**Type:** Minor Release (Phase 10: Agent Confidence & Motivation)

## Summary

Agents can now verify, predict and correlate every GTW operation — and
ask which tool to use without having read our skill file. Everything an
LLM agent needs to *dare* GTW: previews before reverting, receipts
after writing, idempotent retries after timeouts, an independent diff,
and a syntax gate our own docs already promised.

## Changes

### New features (Phase 10)
- **`validate <file>` (CLI + MCP)** — strict pre-edit syntax gate:
  node count on success, `E_STRICT_PARSE` + position + partial-read
  guidance on failure, nonzero CLI exit. Fixes AGENTS.md advertising a
  command that never existed.
- **MCP `diff`** — independent post-write verification
  (`{old_file, new_file}` or node mode), same core as `gnaw-diff`.
- **`undo {preview: true}`** — see exactly what would be reverted
  (id/file/op/description, newest first), zero mutation.
- **Write receipts** — `transaction_id` on edit/insert/semantic_insert,
  `transaction_ids [src, tgt]` on move, per-written-file ids on batch.
- **Idempotent retries** — `edit_node` with content already in place
  returns `already_applied: true` (zero bytes written) instead of
  `E_EDIT_REJECTED`: the timeout → retry → scary-rejection spiral is
  closed.
- **MCP `guide {situation?}`** — the skill situations table as 21
  machine-readable rows for hosts that never load SKILL.md (Motor2);
  every referenced tool is test-pinned to the registry.
- **Cold-start notes** in model-tool descriptions ("first call may take
  20–30 s — raise the host timeout").

### BREAKING (lib) — policy established
- Release checklist now requires a `### BREAKING (lib)` section for
  lib-yta changes before release (retroactive 0.16.0 note included).

### Fixed (carried from [Unreleased])
- **`--no-default-features` builds** (Motor2 report): `sense_with` was
  missing its feature gate — 8 compile errors for path-dependents. Now
  gated + honest stub; `validate.yml` runs
  `cargo check --no-default-features --all-targets` as the permanent
  regression threshold.
- **`index_relations` empty `from`** (Motor2 dashboard): calls resolve
  to the enclosing function or a synthetic `callsite:{line}` — verified
  0-empty on their exact report file; UI "(intern)" patch can go.
- **Doc-drift pass**: GTW_INSTRUCTIONS said 30 tools while 34 were
  registered — now 37 and pinned by
  `integration_mcp_instructions_listed` so it cannot recur; README/SKILL
  fixed (incl. a nonexistent `diff_to_batch` MCP claim); v0.16.0 test
  count corrected.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
# or GPU-enabled:
scripts/build-gpu.sh
```

No breaking changes for MCP clients (all new fields/tools optional).
Lib consumers: `SenseResponse::Satelite` gained `quality` (see BREAKING
in CHANGELOG); `Batch` gained a skipped field — source-compatible.


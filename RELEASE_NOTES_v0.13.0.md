# GnawTreeWriter — Release Notes (v0.13.0)

**Date:** 2026-10-04
**Type:** Minor Release

## Summary

ROADMAP Phase 9 is fully complete (9.1–9.6). This release makes GTW
*guiding and accountable* for agent users: every error tells you the next
step, the agent skill documents when to use which tool and what to do
when GTW fails, and a contract test keeps it that way.

## Changes

### New features
- **SKILL.md for agents, rewritten**: situations-table (situation →
  MCP tool → complete example call), the fallback rule (one timeout →
  one detour → log it → verify bytes), correct batch format,
  `doctor`-based status.
- **AGENTS.md "for agents USING GTW"**: six-step diagnostic chain,
  issue-log note taxonomy, escalation rule.
- **Error-guidance contract test**: raw error passthrough, bare
  feature-gate strings and stub lies are banned from live code —
  mechanically enforced.

### Changed
- ~30 `tool_error` sites in the MCP server now carry next-step guidance
  (rebuild hints, `ai setup`/`ai status`, `search_nodes`/`explore`,
  `list_nodes`+`preview_edit`, `rules list`, issue-log pointers).
  Zero raw strings remain.

### Fixed
- SKILL.md shipped the wrong batch format (`search`/`replace` — the
  finding #13 spec lie) and two `gnawtreewolf` typos; hardcoded stale
  version claim replaced by `doctor`.

## Upgrade Instructions

```bash
cargo install --path . --features modernbert,mcp
# GPU-enabled:
scripts/build-gpu.sh
```

No breaking changes.


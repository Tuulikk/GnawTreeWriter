# Design: code-index SQLite store with a committed markdown projection

**Status:** Accepted direction (2026-10-31) — implementation pending
**Origin:** User discussion, 2026-10-31 GTW session (after v0.20.1 release)
**Scope note:** This precedes T3–T6 in the issue-log backlog; T6 (canonical
ID format) is a *prerequisite* for Phase 3, not for Phase 1.

---

## Summary

Split today's single generated artifact (`code-index.md`) into two layers:

| Layer | Artifact | Git | Role |
|---|---|---|---|
| Machine | `.gnawtreewriter_ai/code_index.db` (SQLite, WAL) | **ignored** | Source of truth; incremental updates; SQL-queryable relations |
| Human/agent | `code-index.md` | **committed** | Rendered projection — readable diffs in PRs, rev-stamped header, hand-written intro splices |

`.gnawtreewriter_ai/` is already in `.gitignore` (line 5) — the DB needs no
gitignore work.

## Motivation

1. **Queries instead of dumps.** Today an agent gets ~1 900 lines and greps
   them. SQL answers "all functions under `crates/motor2-server`",
   "entities changed since rev X", "impl methods without tests" directly.
2. **Incremental updates.** Rows keyed by `(path, hash)` — only changed
   files re-parse. Markdown regenerates the whole tree each run.
3. **Relations.** Entity↔entity edges (calls/imports) become joinable
   rows. This unlocks the 2026-10-14 symbol-existence proposal ("refers to
   `ar_giltig`, not in index, nearest: `is_valid`") which needs exactly
   this entity store.
4. **Cross-tool consumption without new deps.** rusqlite (bundled) is
   already a GTW dependency (vector index, `semantic_index.rs`). Motor2
   and Gnawdriver have their own rusqlite. OpenCode agents with no GTW
   tools can still run read-only `sqlite3` queries (tool-guard allows
   reads; it blocks redirect-writes — read-only discipline is enforced by
   the agent layer itself).
5. **Triggers + runners ("self-running bits").** See dedicated section.

## Consolidation rule — never a fifth index family

Current landscape (2026-10-31):

| Index | Storage |
|---|---|
| Vector index (sense) | SQLite (`.gnawtreewriter_ai/index`, `semantic_index.rs`) |
| Knowledge graph (index_entities/relations) | per-file JSON (`.gnawtreewriter_ai/graph/*.json`) |
| ALF | `alf.json` |
| code-index | markdown at repo root |

The issue log (2026-10-05, Motor2) already documented the damage of "two
indexes, one expectation". **Rule:** the code-index DB must *consolidate*,
not *add*: Phase 3 migrates the knowledge-graph JSON into the same DB file
(tables `entities`/`relations`). One DB file, one connection, one tool
family.

Silver lining: the DB's join keys ARE the entity/relationship IDs — the
design forces resolving the canonical ID format (T6, issue log
2026-10-07: `Entity.id` full format today, `Relation.to` still bare
names). Phase 3 is blocked until that decision lands.

## Schema sketch (v1)

```
files(path TEXT PRIMARY KEY, hash TEXT, rev TEXT, parsed_at INTEGER)
items(id TEXT, file TEXT, kind TEXT, name TEXT,
      signature TEXT, line INTEGER, node_path TEXT)
relations(from_id TEXT, to_id TEXT, kind TEXT, line INTEGER)
intros(dir TEXT PRIMARY KEY, body TEXT)          -- hand-written blocks
staleness_queue(file TEXT, reason TEXT, queued_at INTEGER)
```

- WAL mode; `PRAGMA user_version` drives migrations (v1 at birth).
- Corruption recovery = delete file + full re-index; the artifact is
  fully derivable, nothing precious lives in it.
- `items.node_path` carries the GTW dot-path **alongside** the line
  anchor — this closes the chain `code-index → sense → read_node →
  edit_node` (line numbers shift on every edit; dot-paths survive).

## Triggers vs runners (the "self-running" split)

SQLite triggers can only write *inside* the DB — they cannot parse
files. The design therefore separates:

- **Triggers (in-DB):** maintain `staleness_queue` rows, dirty flags,
  counters (e.g. queued on `/items` writes that supersede an older
  revision).
- **Runners (outside):** do the actual work —
  - git hook (`post-commit`) → incremental refresh of changed files;
  - `code-index --check` in CI / `doctor` → non-zero exit when the
    committed markdown's header rev ≠ HEAD;
  - optional ambient MCP daemon polling `staleness_queue` (ROADMAP
    add-on territory).
- GTW edit responses may append to `staleness_queue` directly — a
  precise alternative to mtime polling.

## MCP surface

One tool: `code_index` with `mode` = `query | refresh | check`.

This is the base level of the capability ladder (bas → struktur →
semantik): **the always-works path must NOT require the DB** — direct
markdown generation (today's behavior) stays as the fallback when the
DB is absent or corrupt. Zero-setup charm is the feature's virtue.

## Non-goals / risks

- **No auto-staging in hooks:** a hook may update `code-index.md`, but
  commits stay explicit and agent-visible.
- **The committed markdown can be one commit behind the world** between
  renders — that is exactly why the `--check` gate exists.
- **rusqlite version bumps are intake-risk** for path-dep consumers
  (2026-10-06 §36 lesson: rusqlite 0.31→0.32 broke Motor2's workspace
  resolution mid-pass). Schema migrations must be called out in the
  CHANGELOG like any consumer-facing contract.
- Threads/processes: single-writer discipline (GTW transaction-ish
  behavior); readers may be any tool.

## Phased plan (proposal)

1. **Phase 1** — `code-index --db` mode: schema v1, incremental
   upsert, markdown rendered from the DB when present (direct fallback
   otherwise).
2. **Phase 2** — `--check` gate + MCP `code_index` tool + doctor
   staleness check.
3. **Phase 3** — knowledge-graph migration into the same DB; canonical
   ID format (T6) resolved and adopted first.
4. **Phase 4 (optional)** — trigger plumbing + runner hooks (git hook,
   staleness writer in edit responses).

## References

- `GTW_MCP_ISSUE_LOG.md` — triage 2026-10-31; T6 (2026-10-07);
  symbol-existence proposal (2026-10-14); §36 intake lesson (rusqlite
  0.32); "two indexes, one expectation" (2026-10-05)
- Motor2 session 2026-10-29 — code-index origin (knowledge-layer action,
  `code-index` command born in 0.20.1-lineage)
- `docs/FUTURE_CONCEPTS.md` — add-on / daemon direction
- Memory: `gnawtreewriter:decision:code-index-sqlite-split` (this decision,
  stored 2026-10-31)

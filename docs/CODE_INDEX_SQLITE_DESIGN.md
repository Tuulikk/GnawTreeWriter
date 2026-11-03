# Design: code-index SQLite store with a committed markdown projection

**Status:** Accepted direction (2026-10-31) · Reviewed 2026-11-02 — 13 findings resolved · Phase 1–3 COMPLETE (store, gate+MCP, graph consolidation) · T6 resolved · Remaining: trigger plumbing/runners (Phase 4, optional)
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

## Schema (v1 — as built)

```
meta(key TEXT PRIMARY KEY, value TEXT NOT NULL)
-- generation_key, gtw_version, extractor_version, schema_version,
-- last_refresh_rev, last_refresh_at

files(path TEXT PRIMARY KEY,         -- canonical_rel_path: POSIX separators,
      hash TEXT NOT NULL,            -- index-root relative; single walker
      lines INTEGER NOT NULL,
      warning TEXT,                  -- formatted last parse warning (NULL = clean)
      parsed_at INTEGER NOT NULL)

items(id INTEGER PRIMARY KEY,
      file TEXT NOT NULL REFERENCES files(path) ON DELETE CASCADE,
      parent_id INTEGER REFERENCES items(id) ON DELETE CASCADE,  -- NULL = top level
      kind TEXT NOT NULL,            -- category (fn/struct/impl/...)
      name TEXT NOT NULL,            -- identifier — consumer revalidation key
      signature TEXT NOT NULL,       -- UNtruncated at storage (cap ~512 chars);
      line INTEGER NOT NULL,         -- truncation to max_sig happens at render
      node_path TEXT NOT NULL)       -- GTW dot-path, valid at indexed hash

staleness_queue(file TEXT PRIMARY KEY, reason TEXT NOT NULL,
                from_rev TEXT, queued_by TEXT, queued_at INTEGER NOT NULL)
```

Deltas from the original sketch, per review decisions: `files.rev` dropped
(P7); `intros` is NOT created (intros are file-sourced, decision #2);
`staleness_queue` gains a PK and is a queue (upsert, latest event wins),
not a log (decision #8); `meta` is new (decisions #1, #7, #9).

**Schema v2 (T6 landed 2026-11-03):** `entities`, `relations` and
`symbols` born — the knowledge-graph consolidation (Phase 3) STARTED in
the same db, filled during refresh from the same parse. `relations`
carries the honest split: `to_name` always, `to_id` NULLable with the
FK on it only (synthetic callsite from-ids are not entity rows — no FK
on `from_id`); unresolved targets aggregate into `symbols` with an
external flag. Entities carry canonical qualified IDs
(`Foo::new`, `#N` suffixes).

- WAL mode; `PRAGMA user_version` drives migrations (v1 at birth).
- `items.node_path` closes the chain `code-index → sense → read_node →
  edit_node` — with the freshness caveat (decision #4): valid at the indexed
  hash; consumers re-validate `name` before editing.
- Corruption recovery = close, delete `code_index.db` **and its `-wal`/`-shm`
  sidecars**, full re-index — surfaced as a notice in the artifact, never
  silent (decision #3). The "fully derivable, nothing precious" claim softens
  in Phase 3 when the knowledge graph moves in: recovery then also costs a
  full graph re-index.

## Triggers vs runners (the "self-running" split)

SQLite triggers can only write *inside* the DB — they cannot parse
files. The design therefore separates:

- **Triggers (in-DB):** maintain `staleness_queue` rows, dirty flags,
  counters (e.g. queued on `/items` writes that supersede an older
  revision).
- **Runners (outside):** do the actual work —
  - git hook (`post-commit`) → incremental refresh of changed files
    (installed via explicit command only, never silently — P4);
  - `code-index --check` in CI / `doctor` → content-hash gate (decision #6):
    fast tier = committed file's normalized body vs its own `body_sha256`
    header; full tier = re-render from the checkout and byte-compare. No
    rev-string logic.
  - optional ambient MCP daemon polling `staleness_queue` (ROADMAP
    add-on territory).
- GTW edit responses may append to `staleness_queue` directly — a precise
  alternative to mtime polling — under the ownership rule (decision #8):
  the edit path may only open an EXISTING DB, never create one.

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
- Threads/processes: `busy_timeout = 5000` + one retry, then degrade to the
  direct path (P5 — the operation must succeed without the DB). Readers: any
  tool, read-only URI recommended (`file:…?mode=ro`, decision #13); refresh
  ends with `wal_checkpoint(TRUNCATE)` so read-only consumers see a clean
  file (`immutable=1` only for archived copies).
- **Merge rule (P3):** `code-index.md` is never hand-merged — take any side,
  re-run the tool. Stated in the artifact header.

## Phased plan (proposal)

1. **Phase 1** — ✅ `code-index --db` mode: schema v1, incremental upsert,
   markdown rendered from the DB when present (direct fallback otherwise).
   Includes the review decisions: generation-keyed cache, toplevel root
   normalization, WAL-sidecar recovery, busy-degrade, parity tests.
2. **Phase 2** — ✅ content-hash `--check` gate (decision #6: `> Body-hash:`
   header over the body = fast tier; fresh-render body compare = full
   tier; `--fix` rewrites; DB-free per P6) + MCP `code_index` tool,
   modes refresh/query/check (`GTW_INSTRUCTIONS.md` parity in the same
   change) + doctor `code_index` store check. Deviation from the sketch:
   the header hash is md5 (drift detection, not adversarial) — same
   hasher as the rest of the codebase, no new dependency.
3. **Phase 3** — ✅ COMPLETE: entities/relations/symbols born and filled
   (schema v2), node_path added (schema v3), `RelationalIndexer` and all
   consumers (impact analysis, blueprint, doctor resolution stats, the
   MCP callers tool) read the store DB-first; legacy JSON pipeline kept
   as the no-DB fallback only — JSON is no longer WRITTEN when the store
   exists. Old `.gnawtreewriter_ai/graph/*.json` files can be deleted;
   cleanup is cosmetic and left to the user.
4. **Phase 4** — ✅ core DONE: staleness writes from the edit funnel
   (open-existing, never create, best-effort) + `--install-hook`/
   `--uninstall-hook` (guarded idempotent block, preserves user logic,
   never auto-stages). Daemon polling remains ROADMAP add-on territory.

## Review 2026-11-02 — decisions

Pre-implementation review (GTW session) found 13 issues; all resolved. The
decisions are baked into the sections above; this list is the trace.

### Cache identity — the action-key principle

Reuse requires identity of everything that produced the row, not just the
content: `generation_key = hash(schema_version + extractor_version +
extensions)` stored in `meta`. Mismatch → warned, planned full re-upsert
(same path as corruption recovery, but expected). Presentation options
(`max_sig`, `--json`, `--output`) are NOT part of the key — signatures are
stored untruncated (cap ~512) and truncated at render, which makes parity
between the direct and DB paths a construction property. `EXTRACTOR_VERSION`
is a manual const, bumped on any change to item extraction.

### Multi-person principles (for and from git)

The DB is per-clone derived state — like `.git/index` and `target/` — never
shared; only the committed projection is shared.

- **P1 Per-clone derived state** — the DB is never shared across clones.
- **P2 One shared truth** — consumers read the committed markdown or their
  own clone's DB, never another clone's.
- **P3 Generated-file merge rule** — never hand-merge `code-index.md`: take
  any side, re-run the tool.
- **P4 Hooks are personal** — runners install via explicit command only,
  never silently.
- **P5 Contention degrades, never fails** — busy_timeout + one retry, then
  the direct path; always-works holds under concurrency.
- **P6 Verification needs no local state** — gates re-render from a
  checkout and compare bytes; no DB required.
- **P7 Stamps live in one place** — rev/dirty/time/version only in the
  header and `meta`; rows carry no identity.

### The 13 findings

| # | Finding | Decision |
|---|---|---|
| 1 | Cache key = content only | Action-key generation (above) |
| 2 | Intros: two sources of truth | The intro FILE is the only source; no `intros` table. DB caching, if ever, is keyed by file hash — cache, never source |
| 3 | Recovery misses WAL sidecars | Recovery deletes `db` + `-wal` + `-shm`, then full re-index — surfaced as a notice in the artifact, never silent |
| 4 | node_path stability oversold | `node_path` is valid at the indexed hash; consumers re-validate `name` via read_node before edit (generalized B1 mechanic, `GTW_RAPPORT_2026-11-01.md`) |
| 5 | Parity asserted, not required | "DB render == direct render (modulo stamps)" is a tested invariant: markdown, JSON, and incremental identity |
| 6 | `--check` rev-string fragile | Content-hash gate: header carries `body_sha256`; full tier re-renders + byte-compares the normalized body, fast tier checks header/body self-consistency; rev logic removed |
| 7 | Single-writer without mechanics | `busy_timeout=5000`, one retry, then P5 degrade; `meta` written last so an aborted refresh heals on the next run |
| 8 | queue without PK; edit-path ownership | PK on file, upsert — a queue, not a log; refresh drains it; the edit path may only open an existing DB, NEVER create one |
| 9 | `rev` per file row | Dropped — `meta.last_refresh_rev` (P7) |
| 10 | DB location for subdir roots | `--db` normalizes the index root to git toplevel (one clone, one DB; no-git dirs use the walked root), announced when it differs |
| 11 | Path normalization unspecified | `canonical_rel_path()`: POSIX separators, index-root relative; one walker (the tool's own) |
| 12 | `relations` born dead / bare names | **Resolved (T6, 2026-11-03):** `relations` + `entities` + `symbols` born in schema v2 with the honest `to_name`/`to_id` split; `items.id` format decided: qualified (`Foo::new`), `#N`-disambiguated, canonical paths — survives line shifts |
| 13 | "Readers may be any tool" | Checkpoint (TRUNCATE) after refresh; consumers use `mode=ro` URIs |

## References

- `GTW_MCP_ISSUE_LOG.md` — triage 2026-10-31; T6 (2026-10-07);
  symbol-existence proposal (2026-10-14); §36 intake lesson (rusqlite
  0.32); "two indexes, one expectation" (2026-10-05)
- Motor2 session 2026-10-29 — code-index origin (knowledge-layer action,
  `code-index` command born in 0.20.1-lineage)
- `docs/FUTURE_CONCEPTS.md` — add-on / daemon direction
- Memory: `gnawtreewriter:decision:code-index-sqlite-split` (this decision,
  stored 2026-10-31)

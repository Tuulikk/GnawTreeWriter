# GnawTreeWriter — Release Notes (v0.20.1)

**Date:** 2026-10-31
**Type:** Patch Release (bug fixes)

## Summary

Two agent-facing write-path bugs from the 2026-10-31 triage are fixed:
in-range insert positions could land **inside a sibling's body** (silent
misplacement, undo-recoverable only if the agent verifies), and
`quick-insert` reported fake success ("✓ applied: 0 insertion(s)") when
the `--after` regex matched nothing — writing zero bytes and logging a
no-op transaction.

## Changes

### Fixed

- **insert: in-range position could land inside a sibling (T2)** — the
  arbitrary-index branch used `children[idx - 3].end_line` as a raw
  insert index. For single-line or token children (e.g. a root's first
  child's anonymous `fn` token) that wrote into the MIDDLE of a
  sibling's body. Live repro: position 3 on a root with 5 functions
  nested the new function inside the first one. Slots (`3..=len`) now
  insert at the addressed sibling's **start boundary** and never inside
  it; out-of-range positions keep the Issue 17 contract (append after
  the last child).

  **Behavior note for lib consumers:** in-range insert placement
  changes (bug → boundary semantics). No API signatures changed.

- **quick-insert: "✓ applied: 0 insertion(s)" with nothing written (T1)** —
  `--after` is a REGULAR EXPRESSION, so an unescaped anchor like
  `## [Unreleased]` (brackets form a character class) matched no line —
  and zero matches were still reported as success. Zero insertions now
  fail loudly with a regex-escaping tip; nothing is written and no
  transaction is logged.

- **clippy: lib target is clean under `-D warnings`** — one
  `needless_borrow` in the new `outline` call path removed.

### Added (carried from Unreleased, now released)

- **`code-index` command** — generated project structure index
  (`gnawtreewriter code-index <root> -o code-index.md --intro <file>`):
  tree + per-file named items with line anchors, git-rev-stamped
  header, hand-written intro splicing, lenient parsing with honest
  warnings. Born from Motor2's 2026-10-29 need to answer "where is X?"
  without reading the codebase. 5 unit tests.

## Upgrade Instructions

- Path-dependency consumers (Motor2 m.fl.): bump the path-dep — no
  API-signature changes, but see the insert behavior note above.
- Rebuild/reinstall and restart MCP hosts/sessions so daemons pick up
  the new binary (stale-daemon lesson 2026-10-28).
- `quick-insert --after` patterns containing `[](){}.*+?|^$. \` must be
  regex-escaped — previously silently matching nothing, now a loud
  error with guidance.

## Validation

- `cargo test --lib`: 264 green (incl. new `test_quick_insert_no_match_fails`)
- `cargo test --test insert_position`: 14 green (incl. 2 new regression
  tests: `insert_pos3_does_not_nest_inside_first_root_fn`,
  `insert_in_range_slot_lands_between_siblings`)
- `cargo clippy --lib -- -D warnings`: clean
- Evidence & triage: `GTW_MCP_ISSUE_LOG.md` (2026-10-31 entries)

## Known issues (open, tracked in GTW_MCP_ISSUE_LOG.md)

- `undo --preview` missing in CLI (T3) — MCP has preview
- edit NO-OP returns Err instead of Ok + `applied=false` (T4)
- `sense`/`search_semantic` hard-error without the bge-base model
  instead of degrading to lexical search (T5)
- `Relation.to` still carries bare names (ID-format decision, T6)
- clippy debt in test modules (`--lib --tests -D warnings` still red;
  lib alone is clean)

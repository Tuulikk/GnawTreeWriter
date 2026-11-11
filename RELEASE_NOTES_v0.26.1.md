# GnawTreeWriter — Release Notes (v0.26.1)

**Date:** 2026-11-11
**Type:** Patch release — one CI fix, one measured constant

## Summary

Two things: the CI gate that went red on `master` is now fed by the flow instead
of by hand, and the `sense` confidence threshold is **measured** rather than
guessed — and the measurement says the old number was wrong in both directions.

## Fixed — the tracked artifact is refreshed by the commit flow

`Validate` failed on two commits (`29bb9a70`, `eaa078de`) and passed on the one
that regenerated `code-index.md` (`e28e29e8`). The new CI gate was working; the
flow had nothing that refreshed the artifact — the install-hook wrote only the
store (`--db`), and no hook was installed locally at all.

- `install-hook` now also writes `code-index.md` **when the file exists**.
- `scripts/commit-build.sh` refreshes the artifact **after** install (with the
  new binary, since both the version stamp and the extraction can differ),
  commits it separately when it changed, and ends with
  `RESULT fail:code-index` instead of `ok` if it is still stale.

Proven in its own run: `artefakten var inaktuell — committad separat (trädet
förblir rent)` → `RESULT ok` → CI green.

## Changed — the `answerable` threshold is now data

New: **`ai recall-eval <positives> --negatives <file>`** measures whether a
single cosine can separate "answerable" from "concept not in this repo" — and
refuses to suggest a number when the sets overlap. `evals/sense_negatives.json`
holds 18 absent-concept queries.

Measured 2026-11-11 (bge-base-en-v1.5, 50 positives / 18 negatives):

| top cosine | positives | negatives |
|---|---|---|
| min | 0.586 | 0.505 |
| p50 | 0.758 | 0.622 |
| max | 0.927 | **0.714** |

**Separation −0.127: the sets overlap, so no threshold can decide.** The old
provisional 0.68 was wrong in both directions — it admitted 3 of 18 absent-concept
queries (they reached 0.714) while still calling 5 of 50 true queries unverified.

**`SENSE_ANSWERABLE_COSINE`: 0.68 → 0.72.** Above the measured negative maximum
with margin: **0/18 negatives pass; 12/50 true queries read as unverified (24 %)**.
That is a deliberate choice of *"unverified"* over *"confidently wrong"*: a
follow-up `read_node` costs a turn, a trusted wrong answer costs the session.

`answerable` therefore stays a **label with the cosine next to it**
(`answerable`, `cosine_threshold`), never a verdict — because the overlap is now
proven, not suspected.

## Upgrade notes

- No API changes. `SENSE_ANSWERABLE_COSINE` is public and changed value; the
  threshold is also reported in every `sense` payload, so consumers can scale
  consistently without hard-coding it.
- Re-run the calibration for another repo or embedder: a threshold measured on
  `bge-base` means nothing for a different model.

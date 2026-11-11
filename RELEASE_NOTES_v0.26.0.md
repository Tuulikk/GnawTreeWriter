# GnawTreeWriter — Release Notes (v0.26.0)

**Date:** 2026-11-11
**Type:** Minor release (additive; no breaking lib changes for `Motor2` readers)

## Summary

0.26.0 is a **trust** release: every claim in it can be checked by a command,
and the places where the tool could previously be confidently wrong now say so
instead of guessing.

This release was cut because a readiness pass found five defects that only
verification could see — a gate nobody ran, an artifact that dirtied itself, a
cache that resolved the wrong thing, a warning that lived on the wrong surface,
and a graph number that was explained away instead of published.

## Highlights

### AI / model resolution
- Model cache is resolved **per model** (`ModelSource`, `ResolvedModel`,
  `resolve()`), not once per manager by probing `modernbert`. A project-pinned
  model no longer hides a machine-wide one.
- `ai status` prints every model with its path and root (`local` | `global`).
- `AiManager::index_model()` is the single owner of "the embedder the index
  uses"; `sense` compares it against the store's `model_info` and emits
  `model_mismatch` instead of silently ranking incomparable vectors.

### `sense`
- **`sense { root }`** — search a named project instead of inheriting the
  daemon's cwd. Root is canonicalized, guarded (`$HOME`, `/`, workspace
  containers) and echoed back. Brokers are cached per root.
- A root with no `.git`/`.gnawtreewriter_ai` says it has no index and no pinned
  models, so lexical hits are not mistaken for semantic answers.
- The cosine threshold has one owner (`SENSE_ANSWERABLE_COSINE`, distinct from
  `SENSE_COSINE_LOW`) and the verdict now **travels in the payload**
  (`answerable`, `cosine_threshold`) — it previously existed only in `cli.rs`,
  so MCP callers got no warning at all.

### Files and undo
- `scaffold` logs `OperationType::CreateFile` with a content hash; plain `undo`
  **refuses** it and points at **`undo --files`**, which removes the file only
  when it is byte-identical to what GTW wrote. Content scope never deletes files.
- `heal` + `validate`: `validate` now offers `heal` only when the failure really
  is a brace problem (`heal_advice()` measures first), CLI and MCP sharing one
  implementation.

### Reliability of the toolchain itself
- `code-index --check` is **in CI** (it existed but nothing ran it, and the
  artifact was stale on a clean master).
- The generated artifact is **content-stamped**, not rev-stamped: it no longer
  dirties itself on every commit, and `code-index .` renders the same title as
  an absolute path.
- `doctor`: the graph-resolution check warns on a **regression** (floor 5 %)
  instead of being permanently yellow against a structural ceiling. 29 checks,
  0 warnings.

## Upgrade notes

- Nothing to change for existing users. `Motor2` sees additive lib surface:
  `AiStatus { bge_installed, models }`, `AiManager::{resolve, resolved_models,
  index_model}`, `ModelSource`, `ResolvedModel`, `SemanticIndexManager::
  store_model_mismatch`, `heal_advice`.
- `undo` no longer silently reverts a file creation — it refuses with the
  `undo --files` remedy. That is intentional.

## Known limits in this release

- Graph `calls` resolution stays at ~10–13 % by design until type inference
  exists; `calls`-based `impact` is a **lower bound**.
- The cosine threshold is still calibrated on 7 queries; it is reported, not
  enforced.
- `core::experiment` is library-only (the saved CLI patch is provably broken).
- **Crush is untested** — no Crush session has ever run.

## Proven before tagging

- 428 tests green across 23 suites; `clippy --all-targets -D warnings` clean;
  `cargo check --no-default-features --all-targets` clean.
- CI on the last pushed HEAD: Validate, Test AI Features and **Test MCP
  Examples** (real Node/Python/Rust MCP clients over stdio) all green.
- `code-index . --check` green; `scripts/pack-skill.sh --check` green;
  `doctor` 29/29 with 0 warnings; `git status` clean.

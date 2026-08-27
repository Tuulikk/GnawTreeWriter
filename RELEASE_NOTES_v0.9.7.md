# GnawTreeWriter — Release Notes (v0.9.7)

**Date:** 2026-08-25
**Type:** Minor Release

## Summary

v0.9.7 delivers a **semgrep-inspired rules engine** that turns `lint` into a real code quality tool, and uses rules to **augment the local LLM** — injecting code-pattern expertise into edit proposals without changing the model. It also adds 70 builtin rules across 7 languages, agent-facing rule-writing tools, and a new multi-edit mode.

## Highlights

### Rules Engine (Steps 1–5)

A deterministic pattern-matching engine built on GTW's AST, with semgrep-like YAML rules:

- **`lint` is now real lint**: 70 builtin rules across rust (10), python (10), javascript (10), typescript (10), go (10), java (10), c (10) — original formulations, not copied from semgrep-rules (whose license forbids redistribution).
- **Edit guardian (Duplex Loop 2.0)**: after any edit, builtin rules check the new code. Error-severity findings block the edit (unless `--force`); warnings are printed. From "does it parse?" to "is it good code?"
- **`rules add` + MCP `add_rule`**: agents can write rules through a validated tool — the pattern must compile for the target language before being saved.
- **`lint --discover`**: the local LFM2.5 model proposes project-specific rules from the codebase; each proposal is validated (must compile + match ≥1 file) before saving.
- **`edit --ask --all`**: model proposes a change for one occurrence, GTW applies it consistently to every matching line in the file — rule-guided multi-edit.
- **Duplicate-finding fix**: same code location is now reported once per rule, not multiple times.

### Architecture

```
Pattern YAML → compile (Rust: $X substitution) → structural AST match → findings
                    │
                    ├─ lint: report to CLI/JSON
                    ├─ edit-guardian: block/warn after edit
                    ├─ prompt-annotations: inject into LLM context
                    └─ discover: LLM proposes new patterns → validate → save
```

## Upgrade Instructions

```bash
cd GnawTreeWriter
git pull origin master
cargo install --path .
gnawtreewriter --version  # Should show 0.9.7
```

## Testing

All tests passing:

```
cargo test
# 170+ lib tests, integration tests, rules tests
```

`cargo clippy --all-targets` is clean (0 errors).

## Acknowledgments

- Semgrep's rule format inspired the `$X` placeholder design (the expressions themselves are original formulations).
- The LFM2.5 model runs locally via candle — enabling rules-driven lint and LLM-augmented edits without external dependencies.

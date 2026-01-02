# GnawTreeWriter — Release Notes (v0.3.3)

**Date:** 2025-01-02  
**Type:** Patch Release — Documentation improvements

---

## Summary

Release `v0.3.3` is a documentation-focused patch release that improves the onboarding experience for new users by adding a prominent Installation section to the README. This release also formalizes the version history by properly marking v0.3.1 and v0.3.2 as released in the changelog.

---

## Changes

### Documentation Improvements

#### Installation Section in README
- **Added prominent Installation section** at the top of README.md (immediately after Features)
- Includes clear step-by-step instructions for installing from source
- Lists prerequisites (Rust, Git)
- Provides Quick Start examples for immediate usage
- Improves GitHub repository visibility and first-time user experience

Example from new Installation section:
```bash
# Clone the repository
git clone https://github.com/Tuulikk/GnawTreeWriter.git
cd GnawTreeWriter

# Build and install
cargo install --path .

# Verify installation
gnawtreewriter --version
```

#### CHANGELOG Cleanup
- Marked v0.3.2 as released (removed "Unreleased" status)
- Added v0.3.3 entry with documentation changes
- Cleaned up version history for clarity

---

## Why This Release?

While v0.3.2 introduced powerful features (diff-to-batch, quick command), the Installation instructions were buried deep in the README under the "Development" section. This made it difficult for new users to quickly understand how to get started with GnawTreeWriter.

This patch release ensures that:
- ✅ New visitors to the GitHub repository immediately see how to install
- ✅ Installation instructions appear in the GitHub README preview
- ✅ Quick Start examples are readily accessible
- ✅ Version history is properly documented in CHANGELOG.md

---

## What's Included (No Code Changes)

This is a **documentation-only release**. All functionality from v0.3.2 remains unchanged:
- Batch Operations (atomic multi-file edits)
- Quick Command (fast single-file edits)
- Diff-to-Batch conversion
- Named References (tags)
- Implicit Sessions
- Time Travel & Restoration
- All 11+ supported languages

---

## Upgrade Instructions

If you're on v0.3.2 or earlier:

```bash
cd GnawTreeWriter
git pull origin master
cargo install --path .
```

Verify the update:
```bash
gnawtreewriter --version
# Should output: gnawtreewriter 0.3.3
```

---

## Version History Context

- **v0.3.0** (2025-12-28) — Batch operations, tags, implicit sessions, generic parser
- **v0.3.1** (2025-12-31) — Code quality improvements (clippy warnings reduced)
- **v0.3.2** (2025-12-31) — Diff-to-batch, quick command, 5 new tests
- **v0.3.3** (2025-01-02) — **This release** — Installation documentation improvements

---

## Testing

All 32 tests continue to pass:
```bash
cargo test
# test result: ok. 32 passed; 0 failed; 0 ignored
```

No regressions introduced (documentation-only changes).

---

## For New Users

If this is your first time using GnawTreeWriter:

1. **Installation**: Follow the new Installation section in README.md
2. **Quick Start**: Try the examples in the Installation section
3. **Learn More**: Check out `gnawtreewriter examples` and `gnawtreewriter wizard`
4. **AI Integration**: See AGENTS.md for AI agent workflows
5. **Advanced Usage**: Read BATCH_USAGE.md for multi-file operations

---

## Acknowledgments

Thanks to user feedback highlighting the difficulty in finding installation instructions. This release addresses that pain point and makes GnawTreeWriter more accessible to new users.

---

**Status:** Released — Tag `v0.3.3` created and pushed to GitHub.
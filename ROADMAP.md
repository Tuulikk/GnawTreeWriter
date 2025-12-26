# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines planned features and improvements.

## Current Status: v0.1.0 (Released 2025-12-26)

### ✅ Completed Features

- Multi-language support (Python, Rust, TypeScript/TSX, PHP, HTML, QML)
- Tree-based editing with dot-notation paths
- Basic CLI commands (analyze, show, edit, insert, delete)
- Fuzzy-edit with multi-strategy matching
- Diff preview for all edit operations
- Automatic backup system with timestamps
- Multi-file operations (analyze, lint, find)
- Human-friendly lint mode
- QML-specific property injection (add-property)

---

## v0.2.0 - QML Parser Improvements

### Priority: HIGH
### Target: Q1 2026

### Goals

Fix QML parser limitations and improve reliability for QML development.

- [ ] Fix QML parser path duplication bug for nested components
- [ ] Improve QML parser to handle complex nesting correctly
- [ ] Add QML-specific validation (unresolved imports, invalid bindings)
- [ ] Add QML runtime error detection (beyond parse errors)
- [ ] Improve property insertion logic for QML components
- [ ] Add QML signal/slot validation

### Technical Debt

- [ ] Refactor QML parser to use TreeSitter instead of custom parser
- [ ] Add comprehensive QML test suite with edge cases
- [ ] Document all QML-specific node types and behaviors

---

## v0.3.0 - Developer Experience

### Priority: HIGH
### Target: Q2 2026

### Goals

Improve ergonomics for both LLM and human developers.

- [ ] Add `undo` command with version history
- [ ] Add `redo` command
- [ ] Implement transaction/batch mode for multiple atomic edits
- [ ] Add `restore` command to restore from specific backup
- [ ] Improve backup cleanup (auto-cleanup old backups)
- [ ] Add configuration file support (~/.gnawtreewriter/config.toml)
- [ ] Add shell completions (bash, zsh, fish)
- [ ] Add man page generation

---

## v0.4.0 - LLM Integration

### Priority: MEDIUM
### Target: Q3 2026

### Goals

Make GnawTreeWriter even more LLM-friendly and powerful.

- [ ] Add content-based node IDs (hash-based addressing)
- [ ] Implement anchor nodes for robust path resolution
- [ ] Add LLM-specific output formats (structured JSON for LLM parsing)
- [ ] Add reasoning/validation mode for LLM operations
- [ ] Implement context window optimization (minimal context, focused edits)
- [ ] Add LLM mode with simplified, deterministic responses

---

## v0.5.0 - Advanced Features

### Priority: MEDIUM
### Target: Q4 2026

### Goals

Advanced editing capabilities and tooling integration.

- [ ] Add `rename` command (rename node by path or fuzzy)
- [ ] Add `move` command (move node to different parent)
- [ ] Add `copy` command (copy node to different location)
- [ ] Add `refactor` command (complex transformations)
- [ ] Implement cross-file references analysis
- [ ] Add dependency graph visualization
- [ ] Add import/export of edit scripts

---

## Future Enhancements

### Language Support

- [ ] Go language support
- [ ] Java language support
- [ ] C++ language support
- [ ] C# language support
- [ ] C language support
- [ ] Swift language support
- [ ] Kotlin language support

### Editor Integration

- [ ] VS Code extension
- [ ] Vim plugin
- [ ] Neovim plugin
- [ ] Emacs mode
- [ ] JetBrains plugin (IntelliJ, CLion, etc.)
- [ ] LSP server implementation

### Advanced Tooling

- [ ] Web interface for visual tree editing
- [ ] VS Code LLM chat integration
- [ ] Cursor integration
- [ ] GitHub Copilot extension
- [ ] CLI GUI (TUI mode)

### Performance

- [ ] Parallel file processing for batch operations
- [ ] Incremental parsing for large files
- [ ] Caching of parsed trees
- [ ] Lazy loading for very large projects

### Security

- [ ] Sandbox execution mode
- [ ] Permission-based file access control
- [ ] Audit log of all operations
- [ ] Encryption of backup files

---

## Contributing

Want to help? See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Areas of High Impact

1. **QML Parser Fixes**: QML is the most fragile parser and needs the most attention
2. **Test Suite**: Add comprehensive tests for all languages
3. **Performance**: Optimize for large files and projects
4. **Language Support**: Add parsers for popular languages

### Suggested First Contributions

- Fix specific QML parser bug (path duplication)
- Add basic test suite for one language
- Improve error messages
- Add one new language parser
- Write example usage scripts

---

## Changelog

For detailed version history, see [CHANGELOG.md](CHANGELOG.md).

## Architecture

For technical details, see [ARCHITECTURE.md](ARCHITECTURE.md).

## LLM Integration

For LLM-specific guidance, see [LLM_INTEGRATION.md](LLM_INTEGRATION.md).

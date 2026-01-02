# GnawTreeWriter — Release Notes (v0.3.4)

**Date:** 2025-01-03  
**Type:** Minor Release

## Summary

This release brings GnawTreeWriter to the cutting edge with **tree-sitter 0.26.3** (the latest version) and adds full support for three major programming languages: **C, C++, and Bash**. All existing parsers have been upgraded to their latest compatible versions, ensuring better performance, stability, and compatibility.

## 🎉 New Language Support

### C Language (`.c`, `.h`)
- Full TreeSitter-based AST parsing
- Support for all C constructs (functions, structs, macros, etc.)
- Header file support

### C++ Language (`.cpp`, `.hpp`, `.cc`, `.cxx`, `.hxx`, `.h++`)
- Complete C++ parsing with class definitions, templates, namespaces
- Modern C++ features supported
- Multiple file extension variants

### Bash Scripting (`.sh`, `.bash`)
- Shell script parsing with function definitions
- Control flow structures (loops, conditionals)
- Variable expansion and command substitution

## 🚀 Major Upgrades

### TreeSitter Core: 0.24 → 0.26.3
The core tree-sitter library has been upgraded to the latest stable version, bringing:
- Improved parsing performance
- Better error recovery
- Enhanced API stability

### Updated Parser Versions
All language parsers have been upgraded to their latest compatible versions:
- `tree-sitter-python`: 0.25.0
- `tree-sitter-rust`: 0.24.0  
- `tree-sitter-go`: 0.25.0
- `tree-sitter-php`: 0.24.2
- `tree-sitter-bash`: 0.25.1 ✨ NEW
- `tree-sitter-c`: 0.24.1 ✨ NEW
- `tree-sitter-cpp`: 0.23.4 ✨ NEW
- `tree-sitter-html`: 0.23.2
- `tree-sitter-typescript`: 0.23.2
- `tree-sitter-javascript`: 0.25.0

## 🔧 Technical Improvements

### Parser API Updates
All parser implementations have been updated to use the tree-sitter 0.26 API with the new `LanguageFn` pattern. This ensures compatibility with the latest parser ecosystem and prepares the codebase for future enhancements.

### Code Quality
- **Zero clippy warnings**: All linting warnings have been resolved
- **32/32 tests passing**: Full test coverage maintained
- **Clean release build**: No compilation warnings or errors

### Fixed Issues
- Removed duplicated `#[cfg(test)]` attribute in undo_redo.rs
- Removed needless borrows in TransactionLog::load calls
- Replaced `find().is_some()` with cleaner `any()` pattern

## 📚 Documentation Updates

- Updated README with new language support
- Added C, C++, and Bash to supported languages table
- Updated AGENTS.md with current parser examples
- Comprehensive CHANGELOG entries
- Added example files for new languages

## 🎯 Current Language Support (15 Languages)

GnawTreeWriter now supports **15 programming languages**:

| Category | Languages |
|----------|-----------|
| **Systems** | C, C++, Rust, Go |
| **Scripting** | Python, Bash, PHP |
| **Web** | TypeScript, JavaScript, HTML, CSS |
| **UI** | QML |
| **Data** | JSON, YAML, TOML, XML, Markdown |

## 📦 Installation

### From Source (Recommended)
```bash
git clone https://github.com/Tuulikk/GnawTreeWriter.git
cd GnawTreeWriter
cargo install --path .
```

### Verify Installation
```bash
gnawtreewriter --version
# Should output: gnawtreewriter 0.3.4
```

## 🧪 Testing the New Languages

### Test C Support
```bash
gnawtreewriter list examples/hello.c --filter-type function_definition
gnawtreewriter analyze examples/hello.c
```

### Test C++ Support
```bash
gnawtreewriter list examples/hello.cpp --filter-type function_definition
gnawtreewriter analyze examples/hello.cpp
```

### Test Bash Support
```bash
gnawtreewriter list examples/hello.sh --filter-type function_definition
gnawtreewriter analyze examples/hello.sh
```

## 🔄 Upgrade Instructions

### From 0.3.3 or Earlier

1. Pull the latest changes:
   ```bash
   cd GnawTreeWriter
   git pull origin master
   git checkout v0.3.4
   ```

2. Rebuild and reinstall:
   ```bash
   cargo clean
   cargo install --path .
   ```

3. Verify the upgrade:
   ```bash
   gnawtreewriter --version
   # Should show: gnawtreewriter 0.3.4
   ```

No breaking changes — all existing workflows continue to work as before.

## 🐛 Known Issues

None at this time. All tests pass and clippy reports zero warnings.

## 📈 What's Next?

Looking ahead to future releases:
- Java language support
- Kotlin language support
- Swift language support
- Additional TreeSitter grammar upgrades
- Performance optimizations for large codebases

## 🙏 Acknowledgments

- TreeSitter community for excellent parser infrastructure
- Rust community for world-class tooling
- All contributors and testers

## 📄 License

This project is licensed under the **Mozilla Public License 2.0 (MPL-2.0)**.

---

**Full Changelog**: https://github.com/Tuulikk/GnawTreeWriter/compare/v0.3.3...v0.3.4  
**Repository**: https://github.com/Tuulikk/GnawTreeWriter  
**Issues**: https://github.com/Tuulikk/GnawTreeWriter/issues
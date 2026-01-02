# GnawTreeWriter — Release Notes (v0.4.0)

**Date:** 2025-01-03  
**Type:** Minor Release

## Summary

This release adds powerful new refactoring capabilities and Java language support, making GnawTreeWriter even more capable for large-scale development and AI-assisted workflows.

## 🎉 New Language Support

### Java Language (`.java`)
- Full TreeSitter-based AST parsing for Java
- Support for classes, interfaces, methods, and all Java constructs
- Type-aware symbol detection and manipulation
- Added comprehensive example file: `examples/HelloWorld.java`

Java support enables GnawTreeWriter to work with one of the most widely used enterprise programming languages, expanding the tool's reach to millions of Java developers worldwide.

## 🔄 New Feature: Refactor/Rename

### AST-Aware Symbol Renaming

GnawTreeWriter now includes intelligent refactoring capabilities that understand code structure:

**What it does:**
- Rename functions, variables, classes across files with confidence
- Distinguishes declarations from usages (knows when you're defining vs using)
- Recursively search entire directories with `--recursive` flag
- Validates new symbol names against language-specific reserved keywords

**Why it's better than search-and-replace:**
- ✅ Understands scope (won't accidentally rename unrelated variables with same name)
- ✅ Type-aware (won't rename comments or strings containing the symbol)
- ✅ Safe by default (validates syntax before applying)
- ✅ Preview mode shows exactly what will change
- ✅ Transaction logging for undo/redo support

**Example refactoring:**
```bash
# Preview renaming a method in a Java file
gnawtreewriter rename greet sayHello examples/HelloWorld.java --preview

# Apply the rename (changes 4 occurrences: 1 definition + 3 usages)
gnawtreewriter rename greet sayHello examples/HelloWorld.java

# Rename a class recursively across an entire project
gnawtreewriter rename MyClass NewClass src/ --recursive
```

**Supported symbol types:**
- Function definitions
- Variable declarations
- Class definitions
- Method calls
- Property identifiers
- Type identifiers
- Field identifiers

**Language-specific validation:**
- Python: `def`, `class`, `import`, `return`, `if`, `else`, `for`, `while`, `try`, `except`
- Rust: `fn`, `let`, `mut`, `pub`, `impl`, `struct`, `enum`, `mod`, `use`, `crate`, `super`
- Java: `class`, `interface`, `extends`, `implements`, `public`, `private`, `protected`, `static`, `final`, `abstract`
- C/C++: `class`, `struct`, `namespace`, `template`, `public`, `private`, `protected`, `virtual`, `friend`
- Go: `func`, `var`, `const`, `type`, `struct`, `interface`, `package`, `import`
- Kotlin: `fun`, `val`, `var`, `class`, `interface`, `object`, `package`, `import`
- JavaScript: `function`, `class`, `const`, `let`, `var`, `if`, `else`, `for`, `while`, `return`
- Bash: `function`, `if`, `then`, `else`, `fi`, `for`, `done`, `while`, `do`

## 🛡 New Feature: Global Dry-Run

### Preview Mode for All Write Operations

A global `--dry-run` flag has been added that works with all commands that make changes to files:

**What it does:**
- Shows exactly what would happen without making any changes
- Displays diffs and previews for all edit operations
- Validates operations in memory before attempting to write
- Increases safety across the entire tool

**Supported commands:**
- `edit` - Preview node edits
- `insert` - Preview insertions
- `delete` - Preview deletions
- `quick` - Preview replace operations
- `rename` - Preview refactoring operations
- `add-property` - Preview property additions (QML)
- `add-component` - Preview component additions (QML)

**Example usage:**
```bash
# Preview an edit without applying
gnawtreewriter edit app.py "0.1" 'def new_function():' --dry-run

# Preview a refactor without applying
gnawtreewriter rename oldFunction newFunction src/ --dry-run --recursive

# Safe workflow: always preview first
gnawtreewriter edit main.rs "1.2" 'fn updated():' --dry-run
# Review output, then run without --dry-run to apply
gnawtreewriter edit main.rs "1.2" 'fn updated():'
```

**Benefits:**
- ✅ Zero-risk exploration
- ✅ Validates syntax before writing
- ✅ Shows exact impact of changes
- ✅ Perfect for AI agents to preview before applying
- ✅ Helps humans understand complex refactorings

## 📈 Current Language Support (16 Languages)

GnawTreeWriter now supports **16 programming and data languages**:

| Category | Languages |
|----------|-----------|
| **Systems** | C, C++, Rust, Go, Java |
| **Scripting** | Python, Bash, PHP, JavaScript |
| **Web** | TypeScript, JavaScript, HTML, CSS |
| **UI** | QML |
| **Data** | JSON, YAML, TOML, XML, Markdown |

## 🔧 Technical Improvements

### TreeSitter 0.26.3 Stability
- All parsers upgraded to use TreeSitter 0.26 API
- TreeSitter Java parser (v0.23.5) integrated
- Consistent `LanguageFn` pattern across all parsers
- Zero breaking changes to existing functionality

### Code Quality
- All 32 unit tests pass
- Zero clippy warnings in release build
- Comprehensive test coverage for new features

### Performance
- Fast symbol search with AST traversal
- Efficient directory recursion with smart ignore patterns
- In-memory validation before disk operations

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
# Should output: gnawtreewriter 0.4.0
```

## 🧪 Testing the New Features

### Test Java Support
```bash
# List all methods in Java file
gnawtreewriter list examples/HelloWorld.java --filter-type method_declaration

# Analyze Java file structure
gnawtreewriter analyze examples/HelloWorld.java
```

### Test Refactor/Rename
```bash
# Preview refactoring a method (dry-run)
gnawtreewriter rename greet sayHello examples/HelloWorld.java --preview

# Actually perform the rename
gnawtreewriter rename greet sayHello examples/HelloWorld.java

# Refactor across entire directory (recursive)
gnawtreewriter rename MyClass NewClass src/ --recursive

# Dry-run mode for maximum safety
gnawtreewriter rename myFunction newFunction project/ --dry-run --recursive
```

### Test Global Dry-Run
```bash
# Preview any edit operation
gnawtreewriter edit app.py "0.1" 'def new():' --dry-run
gnawtreewriter quick file.py 'old' 'new' --dry-run
gnawtreewriter rename oldFunc newFunc src/ --dry-run
```

## 🔄 Upgrade Instructions

### From 0.3.4 or Earlier

1. Pull the latest changes:
   ```bash
   cd GnawTreeWriter
   git pull origin master
   git checkout v0.4.0
   ```

2. Rebuild and reinstall:
   ```bash
   cargo clean
   cargo install --path .
   ```

3. Verify the upgrade:
   ```bash
   gnawtreewriter --version
   # Should show: gnawtreewriter 0.4.0
   ```

No breaking changes - all existing workflows continue to work!

## 🐛 Known Issues

None at this time. All features tested and working as expected.

## 📈 What's Next?

Looking ahead to future releases:
- **v0.5.0**: Additional languages (Swift, Zig, Ruby, R, Mojo, Lua, Dart)
- **v0.6.0**: Additional refactor operations (Clone, Move, Extract Method)
- **v0.7.0**: Semantic search (find by code meaning, not just text)
- **v0.8.0**: Interactive mode and better progress indicators
- **v0.9.0**: Config file support and Git hooks integration

**Path to v1.0.0:**
- Target: 25+ supported languages
- All refactor operations implemented
- Enhanced developer experience features
- Full documentation and examples

## 🙏 Acknowledgments

- TreeSitter community for excellent parser infrastructure
- Rust community for world-class tooling
- Java community for feedback on parser integration
- All contributors and testers

## 📄 License

This project is licensed under the **Mozilla Public License 2.0 (MPL-2.0)**.

---

**Full Changelog**: https://github.com/Tuulikk/GnawTreeWriter/compare/v0.3.4...v0.4.0  
**Repository**: https://github.com/Tuulikk/GnawTreeWriter  
**Issues**: https://github.com/Tuulikk/GnawTreeWriter/issues
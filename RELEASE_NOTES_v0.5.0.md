# GnawTreeWriter — Release Notes (v0.5.0)

**Date:** 2025-01-06  
**Type:** Minor Release

## Summary

This release introduces powerful code duplication capabilities with the new **Clone operation** and adds support for **Zig**, a modern systems programming language. These additions make GnawTreeWriter even more versatile for developers working with cutting-edge technologies.

## 🔄 New Feature: Clone Operation

### Duplicate Code with Precision

The Clone operation allows you to duplicate code nodes and structures within or between files with AST-level precision.

**What it does:**
- Clone functions, classes, structs, or any AST node
- Duplicate code within the same file or copy between files
- Preview changes with diff before applying
- Perfect for creating similar components or duplicating boilerplate code

**Why it's powerful:**
- ✅ **Structure-aware**: Clones entire node hierarchies, not just text
- ✅ **Preview mode**: See exactly what will be duplicated before applying
- ✅ **Cross-file support**: Clone from any file to any other file
- ✅ **Validated**: Syntax validation ensures cloned code is valid
- ✅ **Transaction logged**: Full undo/redo support

**Example usage:**
```bash
# Clone a function within the same file (preview first)
gnawtreewriter clone app.py "0.1" app.py "0.2" --preview

# Apply the clone
gnawtreewriter clone app.py "0.1" app.py "0.2"

# Clone a function from one file to another
gnawtreewriter clone src.rs "1.0" dest.rs "2.0"

# Clone a Zig struct
gnawtreewriter clone main.zig "5" utils.zig "0" --preview
```

**Use cases:**
- Creating similar components with slight variations
- Duplicating boilerplate code structures
- Copying utility functions between modules
- Creating test templates from existing tests
- Scaffolding new features from existing ones

## 🌍 New Language: Zig Support

### Modern Systems Programming

Zig is a modern systems programming language that emphasizes robustness, optimality, and maintainability. GnawTreeWriter now provides full support for Zig development.

**What's supported:**
- Full TreeSitter-based AST parsing for `.zig` files
- Functions, structs, enums, and unions
- Test declarations
- Comptime constructs
- Error handling patterns
- All standard Zig syntax

**Why Zig?**
Zig is gaining rapid adoption among systems programmers who want:
- Manual memory management without undefined behavior
- Compile-time code execution
- Simple, readable syntax
- C interoperability
- No hidden control flow

By supporting Zig early, GnawTreeWriter appeals to early adopters and modern systems programmers who value cutting-edge tools.

**Example usage:**
```bash
# Analyze Zig code structure
gnawtreewriter analyze examples/hello.zig

# List all functions
gnawtreewriter list examples/hello.zig --filter-type function_declaration

# Clone a function
gnawtreewriter clone examples/hello.zig "3" examples/hello.zig "0" --preview

# Rename a symbol across Zig files
gnawtreewriter rename myFunc newFunc src/ --recursive --preview
```

## 📈 Current Language Support (17 Languages)

GnawTreeWriter now supports **17 programming languages**:

| Category | Languages |
|----------|-----------|
| **Systems** | C, C++, Rust, Go, Zig |
| **Enterprise** | Java |
| **Scripting** | Python, Bash, PHP, JavaScript |
| **Web** | TypeScript, JavaScript, HTML, CSS |
| **UI** | QML |
| **Data** | JSON, YAML, TOML, XML, Markdown |

## 🔧 Technical Improvements

### Clone Operation Implementation
- CLI-layer implementation for maximum flexibility
- Uses existing Insert operation for validation and transaction logging
- Preview mode leverages diff system for clear visualization
- Helper function `find_node_by_path` for efficient node lookup

### Zig Parser Integration
- TreeSitter Zig grammar v1.1.2
- Consistent with other TreeSitter parsers (LanguageFn pattern)
- Full example file with functions, structs, and tests
- Zero compilation warnings

### Code Quality
- All 32 unit tests pass
- Zero clippy warnings in release build
- Comprehensive test coverage maintained
- Clean release build

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
# Should output: gnawtreewriter 0.5.0
```

## 🧪 Testing the New Features

### Test Clone Operation
```bash
# Preview cloning a function in Zig
gnawtreewriter clone examples/hello.zig "3" examples/hello.zig "0" --preview

# Clone a function in Python
gnawtreewriter clone app.py "0.1.2" app.py "0.2" --preview

# Clone between files
gnawtreewriter clone source.rs "1.0" destination.rs "2.0"
```

### Test Zig Support
```bash
# Analyze Zig file structure
gnawtreewriter analyze examples/hello.zig

# List all function declarations
gnawtreewriter list examples/hello.zig --filter-type function_declaration

# Show a specific node
gnawtreewriter show examples/hello.zig "3"
```

## 🔄 Upgrade Instructions

### From 0.4.0 or Earlier

1. Pull the latest changes:
   ```bash
   cd GnawTreeWriter
   git pull origin master
   git checkout v0.5.0
   ```

2. Rebuild and reinstall:
   ```bash
   cargo clean
   cargo install --path .
   ```

3. Verify the upgrade:
   ```bash
   gnawtreewriter --version
   # Should show: gnawtreewriter 0.5.0
   ```

No breaking changes - all existing workflows continue to work!

## 🐛 Known Issues

None at this time. All features tested and working as expected.

## 📈 What's Next?

Looking ahead to future releases:

### v0.6.0 (Next Release)
- Additional languages: Swift, Ruby, R, Lua, Dart
- Move operation (relocate code between files)
- Enhanced documentation

### v0.7.0
- Extract Method refactoring
- Semantic search capabilities
- Interactive mode

### v0.8.0
- Config file support
- Git hooks integration
- Progress indicators

### Path to v1.0.0
- Target: 25+ supported languages
- Complete refactoring suite (Rename, Clone, Move, Extract)
- Enhanced developer experience
- Full documentation and guides

## 🎯 Refactoring Suite Progress

**Completed:**
- ✅ **Rename**: AST-aware symbol renaming across files
- ✅ **Clone**: Duplicate code nodes/structures

**Planned:**
- 🔜 **Move**: Relocate code between files
- 🔜 **Extract Method**: Extract code into reusable functions
- 🔜 **Extract Variable**: Extract expressions into variables
- 🔜 **Inline**: Inline functions/variables

## 🙏 Acknowledgments

- TreeSitter community for excellent parser infrastructure
- Zig community for the modern systems programming language
- Rust community for world-class tooling
- All contributors and early adopters

## 📄 License

This project is licensed under the **Mozilla Public License 2.0 (MPL-2.0)**.

---

**Full Changelog**: https://github.com/Tuulikk/GnawTreeWriter/compare/v0.4.0...v0.5.0  
**Repository**: https://github.com/Tuulikk/GnawTreeWriter  
**Issues**: https://github.com/Tuulikk/GnawTreeWriter/issues
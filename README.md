# GnawTreeWriter

Tree-based code editor for LLM-assisted editing. Edit code files based on tree structure levels, avoiding bracket issues and structural problems from LLM code generation.

## Features

- Parse QML and Python files into tree structures
- Analyze file tree structure
- Edit nodes at specific tree paths
- Insert, edit, and delete tree nodes
- CLI interface optimized for both LLM and human usage

## Installation

```bash
cargo build --release
```

## Usage

### Analyze file structure

```bash
cargo run -- analyze path/to/file.qml
```

### Show specific node

```bash
cargo run -- show path/to/file.qml "0.2.1"
```

### Edit node

```bash
cargo run -- edit path/to/file.qml "0.2.1" "width: 100"
```

### Insert node (0 = before, 1 = after, 2 = as child)

```bash
cargo run -- insert path/to/file.qml "0.2" 0 "height: 200"
```

### Delete node

```bash
cargo run -- delete path/to/file.qml "0.2.1"
```

## Architecture

- Parser engines using TreeSitter
- Tree-based node manipulation
- Deterministic edits without bracket issues
- LLM-friendly CLI interface

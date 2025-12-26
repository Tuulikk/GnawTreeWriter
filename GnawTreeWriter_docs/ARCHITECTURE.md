# GnawTreeWriter Documentation

## Overview

GnawTreeWriter is a tree-based code editor designed for LLM-assisted editing. It works at the AST (Abstract Syntax Tree) level, allowing LLMs to make precise edits without worrying about syntax errors, mismatched brackets, or structural issues.

## How It Works

1. **Parse**: Source files are parsed into tree structures using TreeSitter or custom parsers
2. **Navigate**: Tree nodes are accessible via dot-notation paths (e.g., "1.2.0")
3. **Edit**: Operations target specific nodes, preserving structure
4. **Write**: Changes are applied deterministically back to source files

## Architecture

### Parser Engines

Each language has a dedicated parser implementing the `ParserEngine` trait:

```rust
pub trait ParserEngine {
    fn parse(&self, code: &str) -> Result<TreeNode>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;
}
```

#### Currently Supported Languages
- **Python**: Uses TreeSitter for full AST parsing
- **QML**: Custom parser handling QML-specific structure

#### Adding New Languages

1. Create a new file in `src/parser/{language}.rs`
2. Implement the `ParserEngine` trait
3. Add the parser to `src/parser/mod.rs` in `get_parser()`
4. Update Cargo.toml with necessary dependencies

### Tree Structure

```rust
pub struct TreeNode {
    pub id: String,
    pub path: String,           // Dot-notation path like "1.2.0"
    pub node_type: String,      // AST node type
    pub content: String,         // Source code for this node
    pub start_line: usize,
    pub end_line: usize,
    pub children: Vec<TreeNode>,
}
```

### Core Operations

#### Edit
Replace the content of a specific node.

```bash
gnawtreewriter edit file.qml "0.2.1" "width: 100"
```

#### Insert
Add new content at a specific position relative to a node.

```bash
# Position 0: before the node
# Position 1: after the node
# Position 2: as a child of the node
gnawtreewriter insert file.qml "0.2" 0 "height: 200"
```

#### Delete
Remove a node and its content from the tree.

```bash
gnawtreewriter delete file.qml "0.2.1"
```

## LLM Integration

### Approach

The tool is designed to work with LLMs in two main ways:

#### 1. Structure-Aware Editing
LLMs can focus on logic rather than syntax. The tool handles:
- Correct bracket placement
- Proper indentation
- Parent-child relationships
- Node positioning

#### 2. Query-Response Pattern
LLMs can query the tree structure to understand code context, then request specific edits.

### Example LLM Workflow

1. **Analyze**: Get tree structure
   ```bash
   gnawtreewriter analyze app.qml
   ```

2. **Understand**: LLM analyzes tree to find target node
   ```json
   {
     "node_type": "Rectangle",
     "path": "0",
     "children": [...]
   }
   ```

3. **Edit**: LLM requests precise edit
   ```bash
   gnawtreewriter edit app.qml "0.1.2" "color: \"red\""
   ```

## API Reference

### CLI Commands

#### analyze
Analyze file and show tree structure in JSON format.

```bash
gnawtreewriter analyze <file_path>
```

#### show
Show content of a specific node.

```bash
gnawtreewriter show <file_path> <node_path>
```

#### edit
Edit a node's content.

```bash
gnawtreewriter edit <file_path> <node_path> <new_content>
```

#### insert
Insert new content relative to a node.

```bash
gnawtreewriter insert <file_path> <parent_path> <position> <content>
```

Position values:
- `0`: Insert before the node
- `1`: Insert after the node  
- `2`: Insert as a child (where applicable)

#### delete
Delete a node from the tree.

```bash
gnawtreewriter delete <file_path> <node_path>
```

## Development

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Development Workflow

1. Make changes to parser or core logic
2. Test with example files
3. Build to check for errors
4. Commit with descriptive message
5. Update CHANGELOG.md

## Limitations

- Current QML parser is basic and may not handle all QML constructs
- Insert operations are line-based and may need refinement
- No syntax validation after edits
- Limited support for complex refactoring operations

## Future Enhancements

- Improved QML parser using TreeSitter grammar
- Support for more languages (JavaScript, TypeScript, C++, etc.)
- Batch operations for multiple edits
- Diff generation and preview
- Undo/redo functionality
- LLM-specific optimization layer
- Context-aware suggestions

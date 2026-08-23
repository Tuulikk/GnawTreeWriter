//! Code compression using AST analysis.
//!
//! Replaces function/method bodies with `⋮----` placeholders while
//! preserving signatures, imports, type definitions, and doc comments.
//! Reduces token count by ~60-70% while maintaining structural information.

use crate::parser::TreeNode;
use crate::GnawTreeWriter;

/// Compression result with metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompressedOutput {
    /// The compressed source code.
    pub code: String,
    /// Original token count.
    pub original_tokens: usize,
    /// Compressed token count.
    pub compressed_tokens: usize,
    /// Number of bodies compressed.
    pub bodies_compressed: usize,
    /// Compression ratio (0.0 - 1.0, higher = more compressed).
    pub ratio: f64,
}

/// Compress a source file by replacing function/method bodies with placeholders.
///
/// Preserves:
/// - Imports and use declarations
/// - Function/method signatures (parameters, return types)
/// - Type definitions (structs, enums, interfaces, traits)
/// - Doc comments and attributes/decorators
/// - Macro invocations (calls)
///
/// Replaces:
/// - Function/method bodies (block statements)
/// - Anonymous function/closure bodies
pub fn compress_file(file_path: &str) -> Result<CompressedOutput, anyhow::Error> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();
    let source = writer.get_source();

    Ok(compress_source(source, tree))
}

/// Compress source code from a string (for MCP/programmatic use).
pub fn compress_source(source: &str, tree: &TreeNode) -> CompressedOutput {
    let original_tokens = crate::core::token_count::estimate_code_tokens(source);

    let lines: Vec<&str> = source.lines().collect();
    let mut replacements: Vec<(usize, usize)> = Vec::new(); // (start_line, end_line) 1-based, inclusive

    collect_body_replacements(tree, &mut replacements);

    // Sort by start_line descending so we can apply from bottom to top
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    let mut compressed_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    for (start_line, end_line) in &replacements {
        // Convert 1-based to 0-based for indexing
        let start_idx = start_line.saturating_sub(1);
        let end_idx = end_line.saturating_sub(1); // inclusive

        if start_idx < compressed_lines.len() && end_idx < compressed_lines.len() {
            // Get indentation from the start line
            let indent: String = compressed_lines[start_idx]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();

            // Determine what to keep:
            // - If the block starts with `{` on the same line as signature, keep that line
            // - If the block ends with `}` on its own line, keep that line
            let first_line = &compressed_lines[start_idx];
            let last_line = compressed_lines[end_idx].trim_start();

            // Check if opening brace is on the first line (may be after signature)
            let keep_first = first_line.contains('{');
            let keep_last = last_line.starts_with('}') || last_line == "}";

            // Calculate what to replace (inner content)
            let replace_start = if keep_first { start_idx + 1 } else { start_idx };
            let replace_end = if keep_last { end_idx.saturating_sub(1) } else { end_idx };

            // Only replace if there's something to replace
            if replace_start <= replace_end && replace_start < compressed_lines.len() {
                let placeholder = format!("{}⋮----", indent);
                let drain_end = (replace_end + 1).min(compressed_lines.len());
                compressed_lines.drain(replace_start..drain_end);
                compressed_lines.insert(replace_start, placeholder);
            }
        }
    }

    let compressed_code = compressed_lines.join("\n");
    let compressed_tokens = crate::core::token_count::estimate_code_tokens(&compressed_code);

    let ratio = if original_tokens > 0 {
        1.0 - (compressed_tokens as f64 / original_tokens as f64)
    } else {
        0.0
    };

    CompressedOutput {
        code: compressed_code,
        original_tokens,
        compressed_tokens,
        bodies_compressed: replacements.len(),
        ratio,
    }
}

/// Recursively collect body nodes that should be compressed.
fn collect_body_replacements(
    node: &TreeNode,
    replacements: &mut Vec<(usize, usize)>,
) {
    // Check if this node's type indicates it has a compressible body
    if should_compress_body(node) {
        // Find the "body" child (block, statement_block, etc.)
        for child in &node.children {
            if is_body_node(child) {
                // Only compress if the body spans multiple lines
                if child.end_line > child.start_line {
                    replacements.push((child.start_line, child.end_line));
                }
                break;
            }
        }
    }

    // Recurse into children
    for child in &node.children {
        collect_body_replacements(child, replacements);
    }
}

/// Determine if a node type should have its body compressed.
fn should_compress_body(node: &TreeNode) -> bool {
    matches!(
        node.node_type.as_str(),
        // Rust
        "function_item"
            | "impl_item"
            | "trait_item"
            | "macro_definition"
            | "closure_expression"
        // Python
            | "function_definition"
            | "class_definition"
            | "lambda"
        // JavaScript/TypeScript
            | "function_declaration"
            | "function"
            | "arrow_function"
            | "class_declaration"
            | "class"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
        // Go
            | "method_declaration"
        // Java
            | "constructor_declaration"
        // C/C++
            | "function_definition"
            | "declaration_list" // struct/enum bodies
        // Kotlin
            | "lambda_literal"
    )
}

/// Determine if a child node represents a "body" that should be compressed.
fn is_body_node(node: &TreeNode) -> bool {
    matches!(
        node.node_type.as_str(),
        "block"
            | "statement_block"
            | "block_node" // QML
            | "declaration_list"
            | "class_body"
            | "function_body"
            | "arrow_function" // JS/TS arrow bodies
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_rust_function() {
        let source = r#"fn add(a: i32, b: i32) -> i32 {
    let sum = a + b;
    sum
}"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        eprintln!("=== Original ===\n{}", source);
        eprintln!("=== Compressed ===\n{}", result.code);
        eprintln!("=== Bodies: {} ===", result.bodies_compressed);

        assert!(result.code.contains("fn add(a: i32, b: i32) -> i32"),
            "Should preserve function signature. Got:\n{}", result.code);
        assert!(result.code.contains("⋮----"),
            "Should contain compression placeholder. Got:\n{}", result.code);
        assert!(!result.code.contains("let sum = a + b"),
            "Should not contain body code. Got:\n{}", result.code);
        assert!(result.bodies_compressed >= 1);
        assert!(result.ratio > 0.0);
    }

    #[test]
    fn test_compress_preserves_imports() {
        let source = r#"use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
}"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        assert!(result.code.contains("use std::collections::HashMap"),
            "Should preserve imports. Got:\n{}", result.code);
        assert!(result.code.contains("⋮----"),
            "Should contain compression placeholder. Got:\n{}", result.code);
    }

    #[test]
    fn test_compress_preserves_signature() {
        let source = r#"pub fn process_data(
    input: &str,
    config: &Config,
    verbose: bool,
) -> Result<Output, Error> {
    // complex implementation
    todo!()
}"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        assert!(result.code.contains("pub fn process_data"),
            "Should preserve function name. Got:\n{}", result.code);
        assert!(result.code.contains("input: &str"),
            "Should preserve first parameter. Got:\n{}", result.code);
        assert!(result.code.contains("config: &Config"),
            "Should preserve second parameter. Got:\n{}", result.code);
        assert!(result.code.contains("verbose: bool"),
            "Should preserve third parameter. Got:\n{}", result.code);
        assert!(result.code.contains("Result<Output, Error>"),
            "Should preserve return type. Got:\n{}", result.code);
        assert!(!result.code.contains("todo!()"),
            "Should not contain body. Got:\n{}", result.code);
    }

    #[test]
    fn test_compress_python_function() {
        let source = r#"def calculate_total(items):
    total = 0
    for item in items:
        total += item.price
    return total"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.py")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        assert!(result.code.contains("def calculate_total(items):"),
            "Should preserve function signature. Got:\n{}", result.code);
        assert!(result.code.contains("⋮----"),
            "Should contain compression placeholder. Got:\n{}", result.code);
        assert!(!result.code.contains("total = 0"),
            "Should not contain body code. Got:\n{}", result.code);
    }

    #[test]
    fn test_compress_empty_file() {
        let source = "";
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        assert_eq!(result.code, "");
        assert_eq!(result.original_tokens, 0);
        assert_eq!(result.compressed_tokens, 0);
    }

    #[test]
    fn test_compress_no_compressible_nodes() {
        let source = r#"struct Point {
    x: f64,
    y: f64,
}"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        // Struct definitions don't have compressible bodies in our implementation
        assert_eq!(result.bodies_compressed, 0);
        assert!(result.code.contains("x: f64"),
            "Should preserve struct fields. Got:\n{}", result.code);
    }

    #[test]
    fn test_compress_ratio_positive() {
        let source = r#"fn main() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        assert!(result.ratio > 0.0, "Compression ratio should be positive");
        assert!(result.compressed_tokens < result.original_tokens);
    }

    #[test]
    fn test_compress_multiple_functions() {
        let source = r#"fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}"#;
        let parser = crate::parser::get_parser(std::path::Path::new("test.rs")).unwrap();
        let tree = parser.parse(source).unwrap();
        let result = compress_source(source, &tree);

        assert!(result.code.contains("fn add(a: i32, b: i32) -> i32"));
        assert!(result.code.contains("fn multiply(a: i32, b: i32) -> i32"));
        assert!(result.bodies_compressed >= 2,
            "Should compress both functions. Got: {}", result.bodies_compressed);
    }
}

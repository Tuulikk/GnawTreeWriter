//! Code compression using AST analysis.
//!
//! Replaces function/method bodies with `⋮----` placeholders while
//! preserving signatures, imports, type definitions, and doc comments.
//! Optimized for token reduction while maintaining structural information.

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
pub fn compress_file(file_path: &str) -> Result<CompressedOutput, anyhow::Error> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();
    let source = writer.get_source();
    Ok(compress_source(source, tree))
}

/// Compress source code from a string.
pub fn compress_source(source: &str, tree: &TreeNode) -> CompressedOutput {
    let original_tokens = crate::core::token_count::estimate_code_tokens(source);
    let lines: Vec<&str> = source.lines().collect();
    let mut replacements: Vec<(usize, usize)> = Vec::new();

    collect_body_replacements(tree, &mut replacements);

    // Sort descending so we replace from bottom to top
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    let mut compressed_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    for (start_line, end_line) in &replacements {
        let start_idx = start_line.saturating_sub(1);
        let end_idx = end_line.saturating_sub(1);

        if start_idx >= compressed_lines.len() || end_idx >= compressed_lines.len() {
            continue;
        }

        let indent: String = compressed_lines[start_idx]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        let first_line = &compressed_lines[start_idx];
        let last_line = compressed_lines[end_idx].trim_start();

        let keep_first = first_line.contains('{');
        let keep_last = last_line.starts_with('}') || last_line == "}";

        let replace_start = if keep_first { start_idx + 1 } else { start_idx };
        let replace_end = if keep_last { end_idx.saturating_sub(1) } else { end_idx };

        if replace_start <= replace_end && replace_start < compressed_lines.len() {
            let placeholder = format!("{}⋮----", indent);
            let drain_end = (replace_end + 1).min(compressed_lines.len());
            compressed_lines.drain(replace_start..drain_end);
            compressed_lines.insert(replace_start, placeholder);
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
fn collect_body_replacements(node: &TreeNode, replacements: &mut Vec<(usize, usize)>) {
    if should_compress_body(node) {
        for child in &node.children {
            if is_body_node(child) && child.end_line > child.start_line {
                replacements.push((child.start_line, child.end_line));
                break;
            }
        }
    }

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
            | "macro_definition"
            | "closure_expression"
            | "match_expression"
            | "if_expression"
            | "while_expression"
            | "for_expression"
            | "loop_expression"
            | "block"
        // Python
            | "function_definition"
            | "class_definition"
            | "lambda"
            | "decorated_definition"
        // JavaScript/TypeScript
            | "function_declaration"
            | "function"
            | "arrow_function"
            | "class_declaration"
            | "class"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
            | "switch_statement"
            | "if_statement"
            | "for_statement"
            | "while_statement"
            | "do_statement"
        // Go
            | "function_declaration"
            | "method_declaration"
            | "if_statement"
            | "for_statement"
            | "switch_statement"
            | "select_statement"
        // Java
            | "constructor_declaration"
            | "method_declaration"
            | "if_statement"
            | "for_statement"
            | "while_statement"
            | "switch_expression"
        // C/C++
            | "function_definition"
            | "declaration_list"
            | "if_statement"
            | "for_statement"
            | "while_statement"
            | "switch_statement"
        // Kotlin
            | "function_declaration"
            | "class_declaration"
            | "lambda_literal"
            | "if_expression"
            | "when_expression"
            | "for_expression"
            | "while_expression"
        // Swift
            | "function_declaration"
            | "class_declaration"
            | "if_declaration"
            | "for_statement"
            | "while_statement"
            | "switch_statement"
        // Zig
            | "function_declaration"
            | "if_expression"
            | "for_expression"
            | "while_expression"
    )
    // Note: impl_item and trait_item are NOT included — their children are compressed individually
}

/// Determine if a child node represents a "body" that should be compressed.
fn is_body_node(node: &TreeNode) -> bool {
    matches!(
        node.node_type.as_str(),
        "block"
            | "statement_block"
            | "block_node"
            | "declaration_list"
            | "class_body"
            | "function_body"
            | "arrow_function"
            | "match_block"
            | "if_body"
            | "else_body"
            | "loop_body"
            | "block_scoped_statement"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compress(source: &str, ext: &str) -> CompressedOutput {
        let file_name = format!("test.{}", ext);
        let path = std::path::Path::new(&file_name);
        let parser = crate::parser::get_parser(path).unwrap();
        let tree = parser.parse(source).unwrap();
        compress_source(source, &tree)
    }

    #[test]
    fn test_compress_rust_function() {
        let source = r#"fn add(a: i32, b: i32) -> i32 {
    let sum = a + b;
    sum
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("fn add(a: i32, b: i32) -> i32"));
        assert!(result.code.contains("⋮----"));
        assert!(!result.code.contains("let sum = a + b"));
        assert!(result.bodies_compressed >= 1);
    }

    #[test]
    fn test_compress_preserves_imports() {
        let source = r#"use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("use std::collections::HashMap"));
        assert!(result.code.contains("⋮----"));
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
        let result = compress(source, "rs");
        assert!(result.code.contains("pub fn process_data"));
        assert!(result.code.contains("input: &str"));
        assert!(result.code.contains("config: &Config"));
        assert!(result.code.contains("verbose: bool"));
        assert!(result.code.contains("Result<Output, Error>"));
        assert!(!result.code.contains("todo!()"));
    }

    #[test]
    fn test_compress_python_function() {
        let source = r#"def calculate_total(items):
    total = 0
    for item in items:
        total += item.price
    return total"#;
        let result = compress(source, "py");
        assert!(result.code.contains("def calculate_total(items):"));
        assert!(result.code.contains("⋮----"));
        assert!(!result.code.contains("total = 0"));
    }

    #[test]
    fn test_compress_impl_block() {
        let source = r#"impl Config {
    pub fn new() -> Self {
        Config { debug: false }
    }

    pub fn debug(&self) -> bool {
        self.debug
    }
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("impl Config"));
        assert!(result.code.contains("pub fn new()"));
        assert!(result.code.contains("pub fn debug(&self)"));
        // Methods inside impl are compressed individually
        assert!(result.code.contains("⋮----"));
        assert!(!result.code.contains("Config { debug: false }"));
    }

    #[test]
    fn test_compress_trait_definition() {
        let source = r#"trait Drawable {
    fn draw(&self);
    fn bounding_box(&self) -> Rect;
}"#;
        let result = compress(source, "rs");
        // Trait definitions have method signatures but no bodies to compress
        // The methods should be preserved as-is
        assert!(result.code.contains("trait Drawable"),
            "Should preserve trait. Got:\n{}", result.code);
        assert!(result.code.contains("fn draw(&self)"),
            "Should preserve method signature. Got:\n{}", result.code);
        assert!(result.code.contains("fn bounding_box(&self)"),
            "Should preserve method signature. Got:\n{}", result.code);
    }

    #[test]
    fn test_compress_match_expression() {
        let source = r#"fn handle_input(key: char) -> Action {
    match key {
        'q' => Action::Quit,
        'a' => Action::Attack,
        _ => Action::Wait,
    }
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("fn handle_input(key: char)"));
        assert!(result.code.contains("match key"));
    }

    #[test]
    fn test_compress_nested_functions() {
        let source = r#"fn outer() {
    fn inner() {
        println!("nested");
    }
    inner();
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("fn outer()"));
        // Both functions should be compressed
        assert!(result.bodies_compressed >= 1);
    }

    #[test]
    fn test_compress_js_function() {
        let source = r#"function greet(name) {
    return `Hello, ${name}!`;
}"#;
        let result = compress(source, "js");
        assert!(result.code.contains("function greet(name)"));
        assert!(result.code.contains("⋮----"));
        assert!(!result.code.contains("return"));
    }

    #[test]
    fn test_compress_empty_file() {
        let result = compress("", "rs");
        assert_eq!(result.code, "");
        assert_eq!(result.original_tokens, 0);
    }

    #[test]
    fn test_compress_ratio_positive() {
        let source = r#"fn main() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}"#;
        let result = compress(source, "rs");
        assert!(result.ratio > 0.0);
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
        let result = compress(source, "rs");
        assert!(result.code.contains("fn add(a: i32, b: i32) -> i32"));
        assert!(result.code.contains("fn multiply(a: i32, b: i32) -> i32"));
        assert!(result.bodies_compressed >= 2);
    }

    #[test]
    fn test_compress_preserves_doc_comments() {
        let source = r#"/// Calculate the sum of two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("/// Calculate the sum"));
        assert!(result.code.contains("pub fn add"));
    }

    #[test]
    fn test_compress_preserves_attributes() {
        let source = r#"#[cfg(test)]
fn test_something() {
    assert!(true);
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("#[cfg(test)]"));
        assert!(result.code.contains("fn test_something()"));
    }

    #[test]
    fn test_compress_rust_generics() {
        let source = r#"fn process<T: Debug>(item: T) -> String {
    format!("{:?}", item)
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("fn process<T: Debug>(item: T)"));
        assert!(result.code.contains("⋮----"));
        assert!(!result.code.contains("format!"));
    }

    #[test]
    fn test_compress_closure() {
        let source = r#"let add = |a: i32, b: i32| -> i32 {
    a + b
};"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("|a: i32, b: i32|"));
        assert!(result.code.contains("⋮----"));
    }

    #[test]
    fn test_compress_python_decorator() {
        let source = r#"@decorator
def my_function():
    pass

@staticmethod
def helper():
    return 42"#;
        let result = compress(source, "py");
        assert!(result.code.contains("@decorator"));
        assert!(result.code.contains("def my_function()"));
        assert!(result.code.contains("@staticmethod"));
        assert!(result.code.contains("def helper()"));
    }

    #[test]
    fn test_compress_match_with_arms() {
        let source = r#"fn classify(x: i32) -> &'static str {
    match x {
        0 => "zero",
        1..=5 => "small",
        _ => "large",
    }
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("fn classify(x: i32)"));
        assert!(result.code.contains("match x"));
    }

    #[test]
    fn test_compress_nested_if() {
        let source = r#"fn check(a: bool, b: bool) -> i32 {
    if a {
        if b { 1 } else { 2 }
    } else {
        0
    }
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("fn check(a: bool, b: bool)"));
        assert!(result.code.contains("⋮----"));
    }

    #[test]
    fn test_compress_multiline_signature() {
        let source = r#"pub fn complex_function(
    first: String,
    second: Vec<u8>,
    third: Option<Box<dyn Error>>,
) -> Result<(), MyError> {
    Ok(())
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("pub fn complex_function("));
        assert!(result.code.contains("first: String"));
        assert!(result.code.contains("Result<(), MyError>"));
        assert!(result.code.contains("⋮----"));
    }

    #[test]
    fn test_compress_trait_with_default_impl() {
        let source = r#"trait Logger {
    fn log(&self, msg: &str);
    
    fn log_error(&self, err: &str) {
        self.log(&format!("ERROR: {}", err));
    }
}"#;
        let result = compress(source, "rs");
        assert!(result.code.contains("trait Logger"));
        assert!(result.code.contains("fn log(&self, msg: &str)"));
        // log_error has a default implementation, should be compressed
        assert!(result.code.contains("fn log_error(&self, err: &str)"));
    }
}

//! Relation extraction between entities for knowledge graph indexing.
//!
//! Extracts call relationships, imports, type usage, and other dependencies
//! that a Memory System can store as graph edges.

use crate::parser::TreeNode;
use crate::GnawTreeWriter;
use anyhow::Result;
use serde::Serialize;

/// A relationship between two entities.
#[derive(Debug, Clone, Serialize)]
pub struct Relation {
    /// Source entity ID (gtw:{file}:{type}:{name})
    pub from: String,
    /// Target entity ID (gtw:{file}:{type}:{name})
    pub to: String,
    /// Relation type (calls, imports, implements, uses, defines, contains)
    pub relation_type: String,
    /// Line number where the relation occurs
    pub line: usize,
    /// Source file
    pub file: String,
}

/// Result of relation extraction.
#[derive(Debug, Clone, Serialize)]
pub struct RelationIndex {
    /// File path
    pub file: String,
    /// All extracted relations
    pub relations: Vec<Relation>,
    /// Summary counts by relation type
    pub summary: std::collections::HashMap<String, usize>,
}

/// Extract relations from a file.
pub fn index_relations(file_path: &str) -> Result<RelationIndex> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();
    let source = writer.get_source();
    let lines: Vec<&str> = source.lines().collect();

    let mut relations = Vec::new();
    let mut summary = std::collections::HashMap::new();

    // Extract imports as relations
    extract_imports(tree, file_path, &lines, &mut relations);

    // Extract function calls
    extract_calls(tree, file_path, &lines, &mut relations);

    // Extract type usage (struct, enum references)
    extract_type_usage(tree, file_path, &lines, &mut relations);

    // Extract impl relationships
    extract_impl_relations(tree, file_path, &lines, &mut relations);

    // Count by type
    for rel in &relations {
        *summary.entry(rel.relation_type.clone()).or_insert(0) += 1;
    }

    Ok(RelationIndex {
        file: file_path.to_string(),
        relations,
        summary,
    })
}

// ── Extraction helpers ──────────────────────────────────────

fn get_line(lines: &[&str], line_num: usize) -> String {
    let idx = line_num.saturating_sub(1);
    if idx < lines.len() {
        lines[idx].trim().to_string()
    } else {
        String::new()
    }
}

fn extract_imports(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
    for child in &tree.children {
        if child.node_type == "use_declaration" || child.node_type == "import_statement" {
            let import_line = get_line(lines, child.start_line);
            let source_id = format!("gtw:{}:file:{}", file_path, std::path::Path::new(file_path)
                .file_stem().and_then(|s| s.to_str()).unwrap_or("unknown"));

            // Try to extract what's being imported
            let target = if let Some(module) = extract_import_target(&import_line) {
                module
            } else {
                import_line.clone()
            };

            relations.push(Relation {
                from: source_id,
                to: target,
                relation_type: "imports".to_string(),
                line: child.start_line,
                file: file_path.to_string(),
            });
        }

        // Recurse into children for nested imports
        for grandchild in &child.children {
            if grandchild.node_type == "use_declaration" || grandchild.node_type == "import_statement" {
                let import_line = get_line(lines, grandchild.start_line);
                let source_id = format!("gtw:{}:file:{}", file_path, std::path::Path::new(file_path)
                    .file_stem().and_then(|s| s.to_str()).unwrap_or("unknown"));

                let target = if let Some(module) = extract_import_target(&import_line) {
                    module
                } else {
                    import_line
                };

                relations.push(Relation {
                    from: source_id,
                    to: target,
                    relation_type: "imports".to_string(),
                    line: grandchild.start_line,
                    file: file_path.to_string(),
                });
            }
        }
    }
}

fn extract_import_target(import_line: &str) -> Option<String> {
    // "use std::collections::HashMap;" -> "std::collections::HashMap"
    if let Some(start) = import_line.find("use ") {
        let rest = &import_line[start + 4..];
        if let Some(end) = rest.find(';') {
            return Some(rest[..end].trim().to_string());
        }
    }
    // "import os" -> "os"
    if let Some(start) = import_line.find("import ") {
        let rest = &import_line[start + 7..];
        let end = rest.find(|c: char| c.is_whitespace() || c == ';').unwrap_or(rest.len());
        return Some(rest[..end].trim().to_string());
    }
    // "from X import Y" -> "X"
    if let Some(start) = import_line.find("from ") {
        let rest = &import_line[start + 5..];
        let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        return Some(rest[..end].trim().to_string());
    }
    None
}

fn extract_calls(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
    // Find function definitions and their calls
    let mut functions: Vec<(String, usize)> = Vec::new(); // (name, line)
    collect_functions(tree, file_path, lines, &mut functions);

    // For each function, find calls in its body
    for (func_name, func_line) in &functions {
        let func_id = format!("gtw:{}:function:{}", file_path, func_name);
        find_calls_in_scope(tree, &func_id, file_path, lines, relations);
    }

    // Also find standalone calls (not inside functions, e.g., top-level)
    find_calls_in_scope(tree, &format!("gtw:{}:file:{}", file_path,
        std::path::Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("unknown")),
        file_path, lines, relations);
}

fn collect_functions(tree: &TreeNode, file_path: &str, lines: &[&str], functions: &mut Vec<(String, usize)>) {
    if tree.node_type == "function_item" || tree.node_type == "function_definition"
        || tree.node_type == "function_declaration" {
        if let Some(name) = tree.get_name() {
            functions.push((name, tree.start_line));
        }
    }
    for child in &tree.children {
        collect_functions(child, file_path, lines, functions);
    }
}

fn find_calls_in_scope(tree: &TreeNode, caller_id: &str, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
    if tree.node_type == "call_expression" || tree.node_type == "call"
        || tree.node_type == "function_call" {
        // Extract called function name from children
        for child in &tree.children {
            if child.node_type == "identifier" || child.node_type == "field_expression"
                || child.node_type == "function_name" || child.node_type == "selector_expression" {
                let callee = child.get_name().unwrap_or(child.content.trim().to_string());
                if is_user_defined_call(&callee) {
                    relations.push(Relation {
                        from: caller_id.to_string(),
                        to: callee,
                        relation_type: "calls".to_string(),
                        line: tree.start_line,
                        file: file_path.to_string(),
                    });
                }
                break; // Only take the first identifier (the function name)
            }
        }
    }

    for child in &tree.children {
        find_calls_in_scope(child, caller_id, file_path, lines, relations);
    }
}

/// Filter out standard library and common method calls that aren't meaningful relationships.
fn is_user_defined_call(callee: &str) -> bool {
    // Skip empty, self, super, and very short names (likely local variables)
    if callee.is_empty() || callee == "self" || callee == "super" || callee.len() <= 1 {
        return false;
    }

    // Skip common standard library / built-in calls
    const SKIP: &[&str] = &[
        // Rust std
        "println", "eprintln", "print", "format", "vec", "panic", "assert", "assert_eq",
        "assert_ne", "debug_assert", "todo", "unimplemented", "unreachable", "dbg",
        "write", "writeln", "create", "new", "ok", "err", "unwrap", "expect",
        "map", "and_then", "unwrap_or", "unwrap_or_default", "unwrap_or_else",
        "filter", "map", "flat_map", "collect", "into_iter", "iter", "len", "is_empty",
        "push", "pop", "insert", "remove", "contains", "get", "set", "entry",
        "clone", "to_string", "to_owned", "as_str", "as_ref", "as_mut",
        "chars", "split", "trim", "replace", "contains", "starts_with", "ends_with",
        "find", "rfind", "position", "enumerate", "zip", "chain", "take", "skip",
        "sort", "sort_by", "sort_unstable", "dedup", "reverse", "rotate",
        "parse", "from_str", "try_from", "into", "from",
        "is_ok", "is_err", "is_some", "is_none",
        "abs", "sqrt", "powf", "powi", "floor", "ceil", "round",
        "min", "max", "clamp", "saturating_add", "saturating_sub",
        "default", "try_into",
        "to_lowercase", "to_uppercase", "eq_ignore_ascii_case",
        "join", "split_whitespace", "lines", "chars",
        "filter_map", "peek", "next", "advance", "size_hint",
        "count", "sum", "product", "fold", "reduce",
        "any", "all", "position",
        "push_str", "write_fmt",
        // Common method-like calls in Rust code
        "get", "set", "unwrap_or_default", "map_err", "map_ok",
        "unwrap_or_else", "and_then", "or_else", "map_or",
        "with_context", "context", "chain_err",
        "then", "then_some", "filter",
        "to_path_buf", "to_string_lossy", "as_os_str",
        "exists", "is_file", "is_dir", "read_dir",
        "parent", "file_name", "extension", "join",
        "strip_prefix", "strip_suffix",
        "canonicalize", "metadata",
        "push_str", "push", "extend",
        "len", "is_empty", "capacity",
        "contains", "starts_with", "ends_with",
        "find", "rfind", "match_indices",
        "split", "split_whitespace", "split_terminator",
        "trim", "trim_start", "trim_end",
        "replace", "replacen",
        "chars", "bytes", "as_bytes",
        "collect", "into_iter", "iter", "iter_mut",
        "into", "from", "try_into", "try_from",
        "clone", "copy",
        "is_ok", "is_err", "is_some", "is_none",
        "ok", "err", "unwrap", "expect",
        "unwrap_or", "unwrap_or_default", "unwrap_or_else",
        "map", "and_then", "or_else", "map_or", "map_or_else",
        "and", "or",
        "filter", "filter_map", "flat_map", "flatten",
        "take", "skip", "chain", "zip",
        "enumerate", "peekable",
        "any", "all", "find", "position",
        "fold", "reduce", "sum", "product", "count",
        "min", "max", "min_by_key", "max_by_key",
        "sort", "sort_by", "sort_unstable", "sort_by_key",
        "dedup", "dedup_by",
        "push", "pop", "insert", "remove",
        "contains", "get", "entry",
        "len", "is_empty", "capacity",
        "clone", "to_string", "to_owned",
        "as_str", "as_ref", "as_mut",
        "into", "from", "try_into",
    ];

    if SKIP.contains(&callee) {
        return false;
    }

    // Skip method calls (contain a dot — these are on objects, not top-level functions)
    // But allow calls like module::function (double colon)
    if callee.contains('.') && !callee.contains("::") {
        return false;
    }

    true
}

fn extract_type_usage(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
    // Look for type references in function signatures and variable declarations
    for child in &tree.children {
        if child.node_type == "type_identifier" || child.node_type == "scoped_type_identifier" {
            let type_name = child.get_name().unwrap_or(child.content.trim().to_string());
            // Try to find which function this belongs to
            let context_id = find_enclosing_function(tree, child.start_line, file_path);
            if let Some(caller) = context_id {
                relations.push(Relation {
                    from: caller,
                    to: type_name,
                    relation_type: "uses".to_string(),
                    line: child.start_line,
                    file: file_path.to_string(),
                });
            }
        }
        extract_type_usage(child, file_path, lines, relations);
    }
}

fn find_enclosing_function(tree: &TreeNode, target_line: usize, file_path: &str) -> Option<String> {
    if tree.node_type == "function_item" || tree.node_type == "function_definition" {
        if target_line >= tree.start_line && target_line <= tree.end_line {
            return tree.get_name().map(|n| format!("gtw:{}:function:{}", file_path, n));
        }
    }
    for child in &tree.children {
        if let Some(found) = find_enclosing_function(child, target_line, file_path) {
            return Some(found);
        }
    }
    None
}

fn extract_impl_relations(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
    if tree.node_type == "impl_item" {
        let impl_text = get_line(lines, tree.start_line);
        // "impl Display for Foo" or "impl Foo" or "impl<T> Display for Foo<T>"
        if let Some(trait_name) = extract_trait_from_impl(&impl_text) {
            let type_name = tree.get_name().unwrap_or_else(|| {
                // Try to extract from "impl Foo" or "impl<T> Foo"
                if let Some(start) = impl_text.find("impl") {
                    let after = &impl_text[start + 4..].trim();
                    let end = after.find('{').unwrap_or(after.len());
                    let text = after[..end].trim();
                    // Skip generic parameters
                    if let Some(gt) = text.find('>') {
                        text[gt + 1..].trim().to_string()
                    } else {
                        text.to_string()
                    }
                } else {
                    "Unknown".to_string()
                }
            });

            relations.push(Relation {
                from: format!("gtw:{}:impl:{}", file_path, type_name),
                to: trait_name,
                relation_type: "implements".to_string(),
                line: tree.start_line,
                file: file_path.to_string(),
            });
        }
    }

    for child in &tree.children {
        extract_impl_relations(child, file_path, lines, relations);
    }
}

fn extract_trait_from_impl(impl_text: &str) -> Option<String> {
    // "impl Display for Foo" -> Some("Display")
    // "impl<T> Display for Foo<T>" -> Some("Display")
    // "impl Foo" -> None (inherent impl)
    if let Some(for_pos) = impl_text.find(" for ") {
        let before_for = &impl_text[..for_pos];
        // Find the last identifier before " for "
        if let Some(impl_pos) = before_for.rfind("impl") {
            let between = &before_for[impl_pos + 4..].trim();
            // Skip generic parameters <T>
            let after_generics = if let Some(gt) = between.find('>') {
                between[gt + 1..].trim()
            } else {
                between.trim()
            };
            if !after_generics.is_empty() {
                return Some(after_generics.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_index_relations_basic() {
        let source = r#"
use std::collections::HashMap;

pub fn validate(input: &str) -> bool {
    let map = HashMap::new();
    process(map)
}

fn process(data: HashMap<String, String>) -> bool {
    !data.is_empty()
}
"#;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, source).unwrap();

        let result = index_relations(path.to_str().unwrap()).unwrap();

        assert!(result.summary.contains_key("imports"));
        assert!(result.summary.contains_key("calls"));
        assert!(result.summary["imports"] >= 1);
    }

    #[test]
    fn test_index_relations_impl_trait() {
        let source = r#"
impl std::fmt::Display for MyStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MyStruct")
    }
}
"#;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, source).unwrap();

        let result = index_relations(path.to_str().unwrap()).unwrap();

        let implements = result.relations.iter()
            .filter(|r| r.relation_type == "implements")
            .collect::<Vec<_>>();
        assert!(!implements.is_empty(), "Should find implements relation");
        assert!(implements[0].to.contains("Display"));
    }

    #[test]
    fn test_index_relations_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.rs");
        fs::write(&path, "").unwrap();

        let result = index_relations(path.to_str().unwrap()).unwrap();
        assert!(result.relations.is_empty());
        assert!(result.summary.is_empty());
    }

    #[test]
    fn test_index_relations_python_imports() {
        let source = r#"
import os
from pathlib import Path

def main():
    print(os.getcwd())
"#;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.py");
        fs::write(&path, source).unwrap();

        let result = index_relations(path.to_str().unwrap()).unwrap();

        assert!(result.summary.contains_key("imports"));
        assert!(result.summary["imports"] >= 2);
    }

    #[test]
    fn test_extract_import_target() {
        assert_eq!(extract_import_target("use std::collections::HashMap;"), Some("std::collections::HashMap".to_string()));
        assert_eq!(extract_import_target("import os"), Some("os".to_string()));
        assert_eq!(extract_import_target("from pathlib import Path"), Some("pathlib".to_string()));
    }

    #[test]
    fn test_extract_trait_from_impl() {
        assert_eq!(extract_trait_from_impl("impl Display for Foo"), Some("Display".to_string()));
        assert_eq!(extract_trait_from_impl("impl<T> Display for Foo<T>"), Some("Display".to_string()));
        assert_eq!(extract_trait_from_impl("impl Foo"), None);
    }
}

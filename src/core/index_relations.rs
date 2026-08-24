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
        if child.node_type == "use_declaration" || child.node_type == "import_statement"
            || child.node_type == "import_from_statement" {
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
            if grandchild.node_type == "use_declaration" || grandchild.node_type == "import_statement"
                || grandchild.node_type == "import_from_statement" {
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
    // "from X import Y" -> "X" (check BEFORE plain "import" since it's more specific)
    if let Some(start) = import_line.find("from ") {
        let rest = &import_line[start + 5..];
        let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        return Some(rest[..end].trim().to_string());
    }
    // "import os" -> "os"
    if let Some(start) = import_line.find("import ") {
        let rest = &import_line[start + 7..];
        let end = rest.find(|c: char| c.is_whitespace() || c == ';').unwrap_or(rest.len());
        return Some(rest[..end].trim().to_string());
    }
    None
}

    fn extract_calls(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
        let mut defined_funcs: std::collections::HashSet<String> = std::collections::HashSet::new();
        collect_defined_functions(tree, &mut defined_funcs);
        find_calls_in_scope(tree, &defined_funcs, file_path, lines, relations);
    }

    fn collect_defined_functions(tree: &TreeNode, funcs: &mut std::collections::HashSet<String>) {
        if tree.node_type == "function_item" || tree.node_type == "function_definition"
            || tree.node_type == "function_declaration" {
            if let Some(name) = tree.get_name() {
                funcs.insert(name);
            }
        }
        for child in &tree.children {
            collect_defined_functions(child, funcs);
        }
    }

    fn find_calls_in_scope(tree: &TreeNode, defined: &std::collections::HashSet<String>, file_path: &str, _lines: &[&str], relations: &mut Vec<Relation>) {
        if tree.node_type == "call_expression" {
            for child in &tree.children {
                if child.node_type == "identifier" {
                    let callee = child.get_name().unwrap_or_default();
                    if defined.contains(&callee) || callee.contains("::") {
                        relations.push(Relation {
                            from: String::new(),
                            to: callee,
                            relation_type: "calls".to_string(),
                            line: tree.start_line,
                            file: file_path.to_string(),
                        });
                    }
                    break;
                }
            }
        }

        for child in &tree.children {
            find_calls_in_scope(child, defined, file_path, _lines, relations);
        }
    }

fn extract_type_usage(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>) {
    // Only look at top-level function signatures for type references,
    // not inside function bodies (too noisy)
    for child in &tree.children {
        if child.node_type == "function_item" || child.node_type == "function_definition"
            || child.node_type == "function_declaration" {
            // Check parameters and return type for type references
            for param in &child.children {
                if param.node_type == "parameters" || param.node_type == "type_annotation"
                    || param.node_type == "return_type" {
                    extract_type_refs(param, file_path, lines, relations, child);
                }
            }
        }
        // Also check struct fields and enum variants
        if child.node_type == "struct_item" || child.node_type == "enum_item"
            || child.node_type == "trait_item" {
            extract_type_refs(child, file_path, lines, relations, child);
        }
    }
}

fn extract_type_refs(node: &TreeNode, file_path: &str, _lines: &[&str], relations: &mut Vec<Relation>, context: &TreeNode) {
    if node.node_type == "type_identifier" || node.node_type == "scoped_type_identifier" {
        let type_name = node.get_name().unwrap_or(node.content.trim().to_string());
        let context_name = context.get_name().unwrap_or_default();
        if !context_name.is_empty() && type_name != context_name {
            relations.push(Relation {
                from: format!("gtw:{}:function:{}", file_path, context_name),
                to: type_name,
                relation_type: "uses".to_string(),
                line: node.start_line,
                file: file_path.to_string(),
            });
        }
    }
    for child in &node.children {
        extract_type_refs(child, file_path, _lines, relations, context);
    }
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

    #[test]
    fn test_debug_mcp_relations() {
        let result = index_relations("src/mcp/mod.rs").unwrap();
        assert!(result.relations.len() < 200, "Too many relations: {}", result.relations.len());
        assert!(result.relations.iter().any(|r| r.relation_type == "calls"));
    }
}

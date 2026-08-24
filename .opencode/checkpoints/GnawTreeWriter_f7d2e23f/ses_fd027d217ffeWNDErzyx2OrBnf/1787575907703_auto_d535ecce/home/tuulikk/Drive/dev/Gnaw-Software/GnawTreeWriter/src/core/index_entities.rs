//! Entity extraction from source files for memory indexing.
//!
//! Extracts structured entity information from AST nodes that a Memory System
//! can store in its knowledge graph.

use crate::parser::TreeNode;
use crate::GnawTreeWriter;
use anyhow::Result;
use serde::Serialize;

/// An extracted entity from source code.
#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    /// Deterministic ID: gtw:{file}:{type}:{name}
    pub id: String,
    /// Entity type (function, struct, enum, impl, trait, type_alias, import)
    pub entity_type: String,
    /// Entity name (function name, struct name, etc.)
    pub name: String,
    /// Full signature or declaration line
    pub signature: String,
    /// File path (relative)
    pub file: String,
    /// 1-based line number
    pub line: usize,
    /// Visibility (public, private, crate, pub(crate), etc.)
    pub visibility: String,
    /// Doc comment if present
    pub doc_comment: Option<String>,
    /// Token count of the entity
    pub tokens: usize,
    /// Child entities (e.g., methods in an impl block)
    pub children: Vec<Entity>,
}

/// Result of entity extraction.
#[derive(Debug, Clone, Serialize)]
pub struct EntityIndex {
    /// File path
    pub file: String,
    /// All extracted entities
    pub entities: Vec<Entity>,
    /// Top-level imports
    pub imports: Vec<String>,
    /// Exported entities (pub)
    pub exports: Vec<String>,
    /// Total entity count
    pub entity_count: usize,
}

/// Extract entities from a file.
pub fn index_entities(file_path: &str, include_private: bool) -> Result<EntityIndex> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();
    let source = writer.get_source();
    let lines: Vec<&str> = source.lines().collect();

    let mut entities = Vec::new();
    let mut imports = Vec::new();
    let mut exports = Vec::new();

    for child in &tree.children {
        match child.node_type.as_str() {
            // Imports
            "use_declaration" => {
                let sig = get_node_source(child, &lines);
                imports.push(sig.trim().to_string());
            }
            // Functions
            "function_item" | "function_definition" | "function_declaration" => {
                if let Some(entity) = extract_function(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Structs
            "struct_item" | "struct_declaration" | "class_declaration" | "class_definition" => {
                if let Some(entity) = extract_struct(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Enums
            "enum_item" | "enum_declaration" => {
                if let Some(entity) = extract_enum(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Impl blocks
            "impl_item" => {
                if let Some(entity) = extract_impl(child, &lines, file_path, include_private) {
                    entities.push(entity);
                }
            }
            // Traits
            "trait_item" | "trait_declaration" | "interface_declaration" => {
                if let Some(entity) = extract_trait(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Type aliases
            "type_item" | "type_alias" | "type_alias_declaration" => {
                if let Some(entity) = extract_type_alias(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Constants
            "const_item" | "const_declaration" => {
                if let Some(entity) = extract_const(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Static variables
            "static_item" | "static_declaration" => {
                if let Some(entity) = extract_static(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            // Modules (Rust)
            "mod_item" | "mod_declaration" => {
                if let Some(entity) = extract_module(child, &lines, file_path, include_private) {
                    if entity.visibility == "pub" {
                        exports.push(entity.name.clone());
                    }
                    entities.push(entity);
                }
            }
            _ => {}
        }
    }

    Ok(EntityIndex {
        file: file_path.to_string(),
        entity_count: entities.len() + imports.len(),
        imports,
        exports,
        entities,
    })
}

// ── Extraction helpers ──────────────────────────────────────

fn get_node_source(node: &TreeNode, lines: &[&str]) -> String {
    let start = node.start_line.saturating_sub(1);
    let end = node.end_line.min(lines.len());
    lines[start..end].join("\n")
}

fn get_first_line(node: &TreeNode, lines: &[&str]) -> String {
    let idx = node.start_line.saturating_sub(1);
    if idx < lines.len() {
        lines[idx].trim().to_string()
    } else {
        node.content.clone()
    }
}

fn detect_visibility(node: &TreeNode) -> String {
    for child in &node.children {
        if child.node_type == "visibility_modifier" || child.node_type == "visibility" {
            let content = child.content.trim();
            if !content.is_empty() && content != "unnamed" {
                return content.to_string();
            }
            for grandchild in &child.children {
                if !grandchild.content.trim().is_empty() && grandchild.content.trim() != "unnamed" {
                    return grandchild.content.trim().to_string();
                }
            }
            // Check the node_type of grandchildren — "pub" node type
            for grandchild in &child.children {
                if grandchild.node_type == "pub" {
                    return "pub".to_string();
                }
            }
        }
    }
    // Check if content starts with "pub"
    let content = node.content.trim();
    if content.starts_with("pub ") || content.starts_with("pub{") || content.starts_with("pub(") {
        if content.starts_with("pub(") {
            let end = content.find(')').unwrap_or(4);
            return content[..=end].to_string();
        }
        return "pub".to_string();
    }
    "private".to_string()
}

fn extract_doc_comment(node: &TreeNode, lines: &[&str]) -> Option<String> {
    // Doc comments are typically on lines before the entity
    let start = node.start_line.saturating_sub(1);
    let mut doc_lines = Vec::new();

    // Walk backwards from entity start looking for /// or //!
    let mut i = start;
    while i > 0 {
        i -= 1;
        let line = lines[i].trim();
        if line.starts_with("///") || line.starts_with("//!") {
            let doc_text = line.trim_start_matches('/').trim();
            doc_lines.insert(0, doc_text.to_string());
        } else if line.starts_with("#[") || line.is_empty() || line.starts_with("//") {
            break;
        } else {
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join(" "))
    }
}

fn extract_function(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;
    let tokens = crate::core::token_count::estimate_code_tokens(&get_node_source(node, lines));
    let doc = extract_doc_comment(node, lines);

    Some(Entity {
        id: format!("gtw:{}:function:{}", file_path, name),
        entity_type: "function".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: doc,
        tokens,
        children: vec![],
    })
}

fn extract_struct(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;
    let tokens = crate::core::token_count::estimate_code_tokens(&get_node_source(node, lines));
    let doc = extract_doc_comment(node, lines);

    // Extract fields — they live inside field_declaration_list
    let mut children = Vec::new();
    for child in &node.children {
        // Direct field_declaration (some parsers)
        if child.node_type == "field_declaration" || child.node_type == "field_definition" {
            if let Some(field_name) = child.get_name() {
                children.push(Entity {
                    id: format!("gtw:{}:field:{}.{}", file_path, name, field_name),
                    entity_type: "field".to_string(),
                    name: field_name,
                    signature: get_first_line(child, lines),
                    file: file_path.to_string(),
                    line: child.start_line,
                    visibility: detect_visibility(child),
                    doc_comment: None,
                    tokens: 0,
                    children: vec![],
                });
            }
        }
        // Nested inside field_declaration_list (Rust, C++)
        if child.node_type == "field_declaration_list" || child.node_type == "declaration_list" {
            for field in &child.children {
                if field.node_type == "field_declaration" || field.node_type == "field_definition" {
                    if let Some(field_name) = field.get_name() {
                        children.push(Entity {
                            id: format!("gtw:{}:field:{}.{}", file_path, name, field_name),
                            entity_type: "field".to_string(),
                            name: field_name,
                            signature: get_first_line(field, lines),
                            file: file_path.to_string(),
                            line: field.start_line,
                            visibility: detect_visibility(field),
                            doc_comment: None,
                            tokens: 0,
                            children: vec![],
                        });
                    }
                }
            }
        }
    }

    Some(Entity {
        id: format!("gtw:{}:struct:{}", file_path, name),
        entity_type: "struct".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: doc,
        tokens,
        children,
    })
}

fn extract_enum(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;
    let tokens = crate::core::token_count::estimate_code_tokens(&get_node_source(node, lines));
    let doc = extract_doc_comment(node, lines);

    // Extract variants
    let mut children = Vec::new();
    for child in &node.children {
        if child.node_type == "enum_variant" || child.node_type == "variant" {
            if let Some(var_name) = child.get_name() {
                children.push(Entity {
                    id: format!("gtw:{}:variant:{}.{}", file_path, name, var_name),
                    entity_type: "variant".to_string(),
                    name: var_name,
                    signature: get_first_line(child, lines),
                    file: file_path.to_string(),
                    line: child.start_line,
                    visibility: "public".to_string(),
                    doc_comment: None,
                    tokens: 0,
                    children: vec![],
                });
            }
        }
    }

    Some(Entity {
        id: format!("gtw:{}:enum:{}", file_path, name),
        entity_type: "enum".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: doc,
        tokens,
        children,
    })
}

fn extract_impl(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let signature = get_first_line(node, lines);
    let line = node.start_line;
    let tokens = crate::core::token_count::estimate_code_tokens(&get_node_source(node, lines));

    // Extract type name from first child or signature
    let type_name = node.get_name()
        .or_else(|| {
            // Try to extract from "impl<T> Foo" or "impl Foo"
            let sig = signature.clone();
            if let Some(start) = sig.find("impl") {
                let after_impl = &sig[start+4..].trim();
                if let Some(end) = after_impl.find('{') {
                    Some(after_impl[..end].trim().to_string())
                } else {
                    Some(after_impl.to_string())
                }
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Extract methods
    let mut children = Vec::new();
    for child in &node.children {
        if child.node_type == "function_item" || child.node_type == "method_definition" {
            if let Some(entity) = extract_function(child, lines, file_path, include_private) {
                children.push(entity);
            }
        }
    }

    Some(Entity {
        id: format!("gtw:{}:impl:{}", file_path, type_name),
        entity_type: "impl".to_string(),
        name: type_name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: "public".to_string(), // impl blocks don't have visibility
        doc_comment: None,
        tokens,
        children,
    })
}

fn extract_trait(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;
    let tokens = crate::core::token_count::estimate_code_tokens(&get_node_source(node, lines));
    let doc = extract_doc_comment(node, lines);

    // Extract method signatures (not bodies)
    let mut children = Vec::new();
    for child in &node.children {
        if child.node_type == "function_item" || child.node_type == "method_definition" {
            if let Some(entity) = extract_function(child, lines, file_path, include_private) {
                children.push(entity);
            }
        }
    }

    Some(Entity {
        id: format!("gtw:{}:trait:{}", file_path, name),
        entity_type: "trait".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: doc,
        tokens,
        children,
    })
}

fn extract_type_alias(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;
    let doc = extract_doc_comment(node, lines);

    Some(Entity {
        id: format!("gtw:{}:type:{}", file_path, name),
        entity_type: "type_alias".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: doc,
        tokens: 1,
        children: vec![],
    })
}

fn extract_const(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;

    Some(Entity {
        id: format!("gtw:{}:const:{}", file_path, name),
        entity_type: "const".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: None,
        tokens: 1,
        children: vec![],
    })
}

fn extract_static(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;

    Some(Entity {
        id: format!("gtw:{}:static:{}", file_path, name),
        entity_type: "static".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: None,
        tokens: 1,
        children: vec![],
    })
}

fn extract_module(node: &TreeNode, lines: &[&str], file_path: &str, include_private: bool) -> Option<Entity> {
    let vis = detect_visibility(node);
    if !include_private && vis == "private" {
        return None;
    }

    let name = node.get_name().unwrap_or_default();
    let signature = get_first_line(node, lines);
    let line = node.start_line;

    Some(Entity {
        id: format!("gtw:{}:mod:{}", file_path, name),
        entity_type: "module".to_string(),
        name,
        signature,
        file: file_path.to_string(),
        line,
        visibility: vis,
        doc_comment: None,
        tokens: 0,
        children: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_index_entities_basic() {
        let source = r#"use std::collections::HashMap;

/// A configuration struct
pub struct Config {
    pub name: String,
    pub debug: bool,
}

pub fn init(config: Config) -> bool {
    config.debug
}

fn private_helper() {}"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        std::fs::write(&path, source).unwrap();

        let result = index_entities(path.to_str().unwrap(), true).unwrap();

        eprintln!("Imports: {:?}", result.imports);
        eprintln!("Exports: {:?}", result.exports);
        for e in &result.entities {
            eprintln!("  {} '{}' vis='{}'", e.entity_type, e.name, e.visibility);
        }

        assert_eq!(result.imports.len(), 1);
        assert!(result.imports[0].contains("HashMap"));
        assert!(result.exports.contains(&"Config".to_string()),
            "Config should be in exports. Got: {:?}", result.exports);
        assert!(result.exports.contains(&"init".to_string()),
            "init should be in exports. Got: {:?}", result.exports);
        assert!(!result.exports.contains(&"private_helper".to_string()));
        assert!(result.entity_count >= 4);
    }

    #[test]
    fn test_index_entities_private_filtered() {
        let source = r#"
pub fn public_fn() {}
fn private_fn() {}
pub struct PublicStruct {}
struct PrivateStruct {}
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, source).unwrap();

        let result = index_entities(path.to_str().unwrap(), false).unwrap();

        let names: Vec<&str> = result.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"public_fn"));
        assert!(names.contains(&"PublicStruct"));
        assert!(!names.contains(&"private_fn"));
        assert!(!names.contains(&"PrivateStruct"));
    }

    #[test]
    fn test_index_entities_struct_fields() {
        let source = r#"
pub struct User {
    pub id: i32,
    name: String,
    email: String,
}
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, source).unwrap();

        let result = index_entities(path.to_str().unwrap(), true).unwrap();

        let user = result.entities.iter().find(|e| e.name == "User").unwrap();
        assert_eq!(user.entity_type, "struct");
        assert!(user.children.len() >= 2); // at least id and name
    }

    #[test]
    fn test_index_entities_impl_methods() {
        let source = r#"
impl Config {
    pub fn new() -> Self {
        Config { name: String::new(), debug: false }
    }
    fn internal(&self) -> bool {
        self.debug
    }
}
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, source).unwrap();

        let result = index_entities(path.to_str().unwrap(), true).unwrap();

        let impl_entity = result.entities.iter().find(|e| e.entity_type == "impl").unwrap();
        assert!(impl_entity.children.len() >= 1); // at least new()
    }

    #[test]
    fn test_index_entities_enum_variants() {
        let source = r#"
pub enum Color {
    Red,
    Green,
    Blue,
    Rgb(u8, u8, u8),
}
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, source).unwrap();

        let result = index_entities(path.to_str().unwrap(), true).unwrap();

        let color = result.entities.iter().find(|e| e.name == "Color").unwrap();
        assert_eq!(color.entity_type, "enum");
        assert!(color.children.len() >= 3); // Red, Green, Blue
    }

    #[test]
    fn test_index_entities_deterministic_ids() {
        let source = "pub fn add(a: i32, b: i32) -> i32 { a + b }";

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("math.rs");
        fs::write(&path, source).unwrap();

        let r1 = index_entities(path.to_str().unwrap(), true).unwrap();
        let r2 = index_entities(path.to_str().unwrap(), true).unwrap();

        assert_eq!(r1.entities[0].id, r2.entities[0].id);
        assert!(r1.entities[0].id.starts_with("gtw:"));
    }
}

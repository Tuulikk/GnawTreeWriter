pub mod bash;
pub mod c;
pub mod cpp;
pub mod csharp;
pub mod css;
pub mod dart;
pub mod generic;
pub mod go;
pub mod html;
pub mod java;
pub mod javascript;
pub mod json;
pub mod kotlin;
pub mod markdown;
pub mod php;
pub mod python;
pub mod qml;
pub mod qml_tree_sitter;
pub mod rust;
pub mod slint;
pub mod sql;
pub mod svelte;
pub mod swift;
pub mod text;
pub mod toml;
pub mod typescript;
pub mod xml;
pub mod yaml;
pub mod error;
pub use error::{SyntaxError, ParseResult};
pub mod zig;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeNode {
    pub id: String,
    pub path: String,
    pub node_type: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default)]
    pub start_col: usize,
    #[serde(default)]
    pub end_col: usize,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Recursively find a node by its path string.
    pub fn find_path(&self, target_path: &str) -> Option<&TreeNode> {
        if self.path == target_path {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_path(target_path) {
                return Some(found);
            }
        }
        None
    }

    /// Attempts to extract a descriptive name for this node (e.g., function name, class name).
    /// It looks for common identifier-like children.
    pub fn get_name(&self) -> Option<String> {
        let nt = self.node_type.to_lowercase();
        // If the node itself is an identifier, return its content
        if nt == "identifier" || nt == "name" || nt == "type_identifier" || nt == "field_identifier" {
            return Some(self.content.clone());
        }
        
        // Look for identifiers in immediate children
        for child in &self.children {
            let cnt = child.node_type.to_lowercase();
            if cnt == "identifier" || cnt == "name" || cnt == "type_identifier" || cnt == "field_identifier" {
                return Some(child.content.clone());
            }
        }

        // Look one level deeper if needed (common in some complex AST structures)
        for child in &self.children {
            for subchild in &child.children {
                let scnt = subchild.node_type.to_lowercase();
                if scnt == "identifier" || scnt == "name" || scnt == "type_identifier" || scnt == "field_identifier" {
                    return Some(subchild.content.clone());
                }
            }
        }
        None
    }

    /// Find the deepest node that contains the given 1-based line number.
    /// Returns the most specific (deepest) node whose start_line..=end_line range includes `line`.
    pub fn find_node_at_line(&self, line: usize) -> Option<&TreeNode> {
        if line >= self.start_line && line <= self.end_line {
            // Recurse into children first — prefer deeper (more specific) nodes
            for child in &self.children {
                if let Some(found) = child.find_node_at_line(line) {
                    return Some(found);
                }
            }
            Some(self)
        } else {
            None
        }
    }

    /// Find the best parent node for inserting at a given 1-based line number.
    /// Used by diff-to-batch to determine where to insert new code.
    ///
    /// Strategy: Walk the tree depth-first, but when the deepest node at
    /// `line` is at a boundary of a scoped container (function, class, block),
    /// skip past it so the insert becomes a sibling rather than a child.
    pub fn find_parent_at_line(&self, line: usize) -> &TreeNode {
        if !(line >= self.start_line && line <= self.end_line) {
            return self;
        }

        let mut best_container: Option<&TreeNode> = None;

        for child in &self.children {
            if !(line >= child.start_line && line <= child.end_line) {
                continue;
            }

            let nt = child.node_type.to_lowercase();

            // Check if this child is a "scoped container" whose boundary
            // should not be descended into. This includes:
            // - function_item, method_declaration, fn_definition
            // - class_definition, struct_item, enum_item
            // - impl_item, trait_item, mod_item
            // - block, body, statement_block
            let is_scoped = nt.contains("function")
                || nt.contains("method")
                || nt.contains("class")
                || nt.contains("struct")
                || nt.contains("enum")
                || (nt.contains("impl") && !nt.contains("implementation"))
                || nt.contains("trait")
                || nt.contains("mod")
                || nt.contains("block")
                || nt.contains("body")
                || nt.contains("statement_block");

            let at_boundary = line <= child.start_line + 1
                || line >= child.end_line.saturating_sub(1);

            if is_scoped && at_boundary {
                // The line is at the edge of a scoped container.
                // Don't descend — the insert should be a sibling, not a child.
                best_container = None;
                continue;
            }

            // Recurse into the child
            let result = child.find_parent_at_line(line);
            best_container = Some(result);
        }

        best_container.unwrap_or(self)
    }
}

pub trait ParserEngine {
    fn parse(&self, code: &str) -> ParseResult<TreeNode>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;
}

pub fn to_parse_result<T>(res: anyhow::Result<T>) -> ParseResult<T> {
    res.map_err(SyntaxError::from)
}

/// A wrapper to let older parsers that return anyhow::Result work with the new ParseResult.
pub struct LegacyParserWrapper<P: ParserEngineLegacy> {
    inner: P,
}

impl<P: ParserEngineLegacy> LegacyParserWrapper<P> {
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P: ParserEngineLegacy> ParserEngine for LegacyParserWrapper<P> {
    fn parse(&self, code: &str) -> ParseResult<TreeNode> {
        to_parse_result(self.inner.parse_legacy(code))
    }
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        self.inner.get_supported_extensions()
    }
}

pub trait ParserEngineLegacy {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;
}
pub fn get_parser(file_path: &Path) -> anyhow::Result<Box<dyn ParserEngine>> {
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "qml" => Ok(Box::new(LegacyParserWrapper::new(qml_tree_sitter::QmlTreeSitterParser::new()))),
        "py" => Ok(Box::new(python::PythonParser::new())),
        "rs" => Ok(Box::new(rust::RustParser::new())),
        "slint" => Ok(Box::new(slint::SlintParser::new())),
        "kt" | "kts" => Ok(Box::new(LegacyParserWrapper::new(kotlin::KotlinParser::new()))),
        "swift" => Ok(Box::new(LegacyParserWrapper::new(swift::SwiftParser::new()))),
        "cs" => Ok(Box::new(LegacyParserWrapper::new(csharp::CSharpParser::new()))),
        "dart" => Ok(Box::new(LegacyParserWrapper::new(dart::DartParser::new()))),
        "svelte" => Ok(Box::new(LegacyParserWrapper::new(svelte::SvelteParser::new()))),
        "sql" => Ok(Box::new(LegacyParserWrapper::new(sql::SqlParser::new()))),
        "js" | "jsx" | "mjs" | "cjs" => Ok(Box::new(LegacyParserWrapper::new(javascript::JavaScriptParser::new()))),
        "ts" | "tsx" => Ok(Box::new(LegacyParserWrapper::new(typescript::TypeScriptParser::new()))),
        "php" => Ok(Box::new(LegacyParserWrapper::new(php::PhpParser::new()))),
        "html" | "htm" => Ok(Box::new(LegacyParserWrapper::new(html::HtmlParser::new()))),
        "go" => Ok(Box::new(LegacyParserWrapper::new(go::GoParser::new()))),
        "c" | "h" => Ok(Box::new(LegacyParserWrapper::new(c::CParser::new()))),
        "cpp" | "hpp" | "cc" | "cxx" | "hxx" | "h++" => Ok(Box::new(LegacyParserWrapper::new(cpp::CppParser::new()))),
        "sh" | "bash" => Ok(Box::new(LegacyParserWrapper::new(bash::BashParser::new()))),
        "java" => Ok(Box::new(LegacyParserWrapper::new(java::JavaParser::new()))),
        "zig" => Ok(Box::new(LegacyParserWrapper::new(zig::ZigParser::new()))),
        "css" => Ok(Box::new(LegacyParserWrapper::new(css::CssParser::new()))),
        "xml" | "svg" | "xsl" | "xsd" | "rss" | "atom" => Ok(Box::new(xml::XmlParser::new())),
        "md" | "markdown" => Ok(Box::new(LegacyParserWrapper::new(markdown::MarkdownParser::new()))),
        "txt" => Ok(Box::new(LegacyParserWrapper::new(text::TextParser::new()))),
        "toml" => Ok(Box::new(LegacyParserWrapper::new(toml::TomlParser::new()))),
        "json" => Ok(Box::new(LegacyParserWrapper::new(json::JsonParser::new()))),
        "yaml" | "yml" => Ok(Box::new(LegacyParserWrapper::new(yaml::YamlParser::new()))),
        _ => {
            Ok(Box::new(generic::GenericParser::new()))
        }
    }
}
pub mod bash;
pub mod c;
pub mod cpp;
pub mod css;
pub mod generic;
pub mod go;
pub mod html;
pub mod java;
pub mod json;
pub mod markdown;
pub mod php;
pub mod python;
pub mod qml;
pub mod qml_tree_sitter;
pub mod rust;
pub mod text;
pub mod toml;
pub mod typescript;
pub mod xml;
pub mod yaml;

use anyhow::Result;
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
    pub children: Vec<TreeNode>,
}

pub trait ParserEngine {
    fn parse(&self, code: &str) -> Result<TreeNode>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;
}

pub fn get_parser(file_path: &Path) -> Result<Box<dyn ParserEngine>> {
    // Get file extension, defaulting to empty string if none exists
    // This handles files like README, Dockerfile, etc.
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "qml" => Ok(Box::new(qml_tree_sitter::QmlTreeSitterParser::new())),
        "py" => Ok(Box::new(python::PythonParser::new())),
        "rs" => Ok(Box::new(rust::RustParser::new())),
        "ts" | "tsx" => Ok(Box::new(typescript::TypeScriptParser::new())),
        "php" => Ok(Box::new(php::PhpParser::new())),
        "html" | "htm" => Ok(Box::new(html::HtmlParser::new())),
        "go" => Ok(Box::new(go::GoParser::new())),
        "c" | "h" => Ok(Box::new(c::CParser::new())),
        "cpp" | "hpp" | "cc" | "cxx" | "hxx" | "h++" => Ok(Box::new(cpp::CppParser::new())),
        "sh" | "bash" => Ok(Box::new(bash::BashParser::new())),
        "java" => Ok(Box::new(java::JavaParser::new())),
        "css" => Ok(Box::new(css::CssParser::new())),
        "xml" | "svg" | "xsl" | "xsd" | "rss" | "atom" => Ok(Box::new(xml::XmlParser::new())),
        "md" | "markdown" => Ok(Box::new(markdown::MarkdownParser::new())),
        "txt" => Ok(Box::new(text::TextParser::new())),
        "toml" => Ok(Box::new(toml::TomlParser::new())),
        "json" => Ok(Box::new(json::JsonParser::new())),
        "yaml" | "yml" => Ok(Box::new(yaml::YamlParser::new())),
        _ => {
            // Use generic parser for all other file types
            // This enables backup/history for ALL files, not just those we can parse
            Ok(Box::new(generic::GenericParser::new()))
        }
    }
}

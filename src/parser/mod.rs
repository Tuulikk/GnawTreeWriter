pub mod qml;
pub mod python;
pub mod rust;
pub mod typescript;
pub mod php;
pub mod html;

use anyhow::{Context, Result};
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
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .context("No file extension found")?;

    match extension {
        "qml" => Ok(Box::new(qml::QmlParser::new())),
        "py" => Ok(Box::new(python::PythonParser::new())),
        "rs" => Ok(Box::new(rust::RustParser::new())),
        "ts" | "tsx" => Ok(Box::new(typescript::TypeScriptParser::new())),
        "php" => Ok(Box::new(php::PhpParser::new())),
        "html" | "htm" => Ok(Box::new(html::HtmlParser::new())),
        _ => Err(anyhow::anyhow!("Unsupported file extension: {}", extension)),
    }
}

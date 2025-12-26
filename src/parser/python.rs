use crate::parser::{ParserEngine, TreeNode};
use anyhow::Result;
use tree_sitter::Parser;

pub struct PythonParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngine for PythonParser {
    fn parse(&self, code: &str) -> Result<TreeNode> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::language()).expect("Failed to load Python grammar");
        let tree = parser.parse(code, None).ok_or_else(|| anyhow::anyhow!("Failed to parse Python"))?;
        Ok(Self::build_tree(&tree.root_node(), code, "".to_string())?)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["py"]
    }
}

impl PythonParser {
    fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode> {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let content = if let Ok(s) = std::str::from_utf8(&source.as_bytes()[start_byte..end_byte]) {
            s.to_string()
        } else {
            String::new()
        };

        let node_type = node.kind().to_string();
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        let mut children = Vec::new();
        let mut cursor = node.walk();

        for (i, child) in node.children(&mut cursor).enumerate() {
            let child_path = if path.is_empty() {
                i.to_string()
            } else {
                format!("{}.{}", path, i)
            };
            children.push(Self::build_tree(&child, source, child_path)?);
        }

        let id = path.clone();

        Ok(TreeNode {
            id,
            path,
            node_type,
            content,
            start_line,
            end_line,
            children,
        })
    }
}

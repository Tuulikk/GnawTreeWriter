use anyhow::Result;
use tree_sitter::{Parser, Language};
use crate::parser::{ParserEngine, TreeNode};

pub struct GoParser;

impl GoParser {
    pub fn new() -> Self {
        Self
    }

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

impl ParserEngine for GoParser {
    fn parse(&self, source_code: &str) -> Result<TreeNode> {
        let mut parser = Parser::new();
        let language: Language = tree_sitter_go::language();
        parser.set_language(&language)?;

        let tree = parser.parse(source_code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Go code"))?;

        Ok(Self::build_tree(&tree.root_node(), source_code, String::new())?)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["go"]
    }
}
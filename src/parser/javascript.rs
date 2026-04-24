use crate::parser::{TreeNode, ParserEngineLegacy};
use tree_sitter::Parser;
use anyhow::Result;

pub struct JavaScriptParser;

impl Default for JavaScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for JavaScriptParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let mut parser = Parser::new();
        let language = unsafe {
            std::mem::transmute::<tree_sitter_language::LanguageFn, fn() -> tree_sitter::Language>(
                tree_sitter_javascript::LANGUAGE,
            )()
        };
        parser.set_language(&language)?;

        let tree = parser
            .parse(code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse JavaScript"))?;

        Self::build_tree(&tree.root_node(), code, "".to_string())
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["js", "jsx", "mjs", "cjs"]
    }
}

impl JavaScriptParser {
    fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode> {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let content = if let Some(s) = source.get(start_byte..end_byte) {
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
            start_col: 0,
            end_col: 0,
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

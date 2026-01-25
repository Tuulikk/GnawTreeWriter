use crate::parser::{TreeNode, ParserEngineLegacy};

pub struct TextParser;

impl Default for TextParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TextParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for TextParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let lines: Vec<&str> = code.lines().collect();
        let mut root_children = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            root_children.push(TreeNode {
                id: format!("line_{}", i),
                path: i.to_string(),
                node_type: "text_line".to_string(),
                content: line.to_string(),
                start_line: i + 1,
                end_line: i + 1,
                children: Vec::new(),
            });
        }

        Ok(TreeNode {
            id: "root".to_string(),
            path: "0".to_string(),
            node_type: "text_file".to_string(),
            content: code.to_string(),
            start_line: 1,
            end_line: if lines.is_empty() { 1 } else { lines.len() },
            children: root_children,
        })
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["txt"]
    }
}
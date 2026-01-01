use crate::parser::{ParserEngine, TreeNode};
use anyhow::Result;

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

impl ParserEngine for TextParser {
    fn parse(&self, code: &str) -> Result<TreeNode> {
        let lines: Vec<&str> = code.lines().collect();
        let mut children = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            children.push(TreeNode {
                id: format!("{}", i),
                path: format!("{}", i),
                node_type: "line".to_string(),
                content: line.to_string(),
                start_line: line_num,
                end_line: line_num,
                children: Vec::new(),
            });
        }

        Ok(TreeNode {
            id: "".to_string(),
            path: "".to_string(),
            node_type: "document".to_string(),
            content: String::new(),
            start_line: 1,
            end_line: lines.len(),
            children,
        })
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["txt"]
    }
}

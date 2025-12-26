use crate::parser::{ParserEngine, TreeNode};
use anyhow::Result;

pub struct QmlParser;

impl QmlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngine for QmlParser {
    fn parse(&self, code: &str) -> Result<TreeNode> {
        Ok(self.parse_qml(code, "root".to_string(), 1)?)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["qml"]
    }
}

impl QmlParser {
    fn parse_qml(&self, code: &str, path: String, line: usize) -> Result<TreeNode> {
        let lines: Vec<&str> = code.lines().collect();
        let mut children = Vec::new();
        let mut current_content = String::new();
        let mut depth = 0;
        let start_line = line;

        for (i, line_content) in lines.iter().enumerate() {
            let actual_line = line + i;
            let trimmed = line_content.trim_start();
            let new_depth = line_content.len() - trimmed.len();

            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            if trimmed.starts_with("}") {
                if depth > 0 {
                    depth -= 2;
                }
                continue;
            }

            if trimmed.ends_with("{") {
                let component_name = trimmed[..trimmed.len() - 1].trim();
                if !component_name.is_empty() {
                    let child_path = format!("{}.{}", path, children.len());
                    let mut subtree = self.parse_nested_qml(
                        &lines[i + 1..],
                        child_path.clone(),
                        actual_line + 1,
                        new_depth + 2,
                    )?;

                    subtree.node_type = component_name.to_string();
                    children.push(subtree);
                }
                continue;
            }

            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line_content);
        }

        Ok(TreeNode {
            id: path.clone(),
            path,
            node_type: "QmlDocument".to_string(),
            content: current_content,
            start_line,
            end_line: line + lines.len(),
            children,
        })
    }

    fn parse_nested_qml(
        &self,
        lines: &[&str],
        path: String,
        line: usize,
        target_depth: usize,
    ) -> Result<TreeNode> {
        let mut children = Vec::new();
        let mut properties = Vec::new();
        let mut current_content = String::new();
        let start_line = line;
        let mut i = 0;

        while i < lines.len() {
            let line_content = lines[i].to_string();
            let trimmed = line_content.trim_start();
            let current_depth = line_content.len() - trimmed.len();

            if trimmed.is_empty() || trimmed.starts_with("//") {
                i += 1;
                continue;
            }

            if trimmed.starts_with("}") && current_depth == target_depth - 2 {
                break;
            }

            if current_depth == target_depth {
                if trimmed.ends_with("{") {
                    let component_name = trimmed[..trimmed.len() - 1].trim();
                    let child_path = format!("{}.{}", path, children.len());
                    let subtree = self.parse_nested_qml(
                        &lines[i + 1..],
                        child_path.clone(),
                        line + i + 1,
                        target_depth + 2,
                    )?;
                    
                    let mut result = subtree;
                    result.node_type = component_name.to_string();
                    children.push(result);
                    
                    i += 1;
                    while i < lines.len() {
                        let d = lines[i].len() - lines[i].trim_start().len();
                        if d < target_depth + 2 {
                            break;
                        }
                        i += 1;
                    }
                } else {
                    properties.push(line_content.clone());
                    if !current_content.is_empty() {
                        current_content.push('\n');
                    }
                    current_content.push_str(&line_content);
                    i += 1;
                }
            } else if current_depth > target_depth {
                i += 1;
            } else {
                break;
            }
        }

        Ok(TreeNode {
            id: path.clone(),
            path,
            node_type: "Component".to_string(),
            content: current_content,
            start_line,
            end_line: line + i,
            children,
        })
    }
}

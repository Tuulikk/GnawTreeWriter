use crate::parser::{TreeNode, ParserEngineLegacy};

pub struct QmlParser;

impl Default for QmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl QmlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for QmlParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let mut root_children = Vec::new();
        let lines: Vec<&str> = code.lines().collect();
        let mut current_obj_name = "unknown".to_string();
        let mut in_obj = false;
        let mut obj_start_line = 1;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            if trimmed.ends_with("{") {
                if !in_obj {
                    current_obj_name = trimmed.trim_end_matches("{").trim().to_string();
                    in_obj = true;
                    obj_start_line = i + 1;
                }
            } else if trimmed == "}" {
                if in_obj {
                    root_children.push(TreeNode {
                        id: format!("obj_{}", root_children.len()),
                        path: root_children.len().to_string(),
                        node_type: "ui_object".to_string(),
                        content: current_obj_name.clone(),
                        start_line: obj_start_line,
                        end_line: i + 1,
                        children: Vec::new(),
                    });
                    in_obj = false;
                }
            } else if in_obj {
                if let Some((prop_name, prop_value)) = self.parse_property(trimmed) {
                    let last_idx = root_children.len();
                    if last_idx > 0 {
                        let parent = &mut root_children[last_idx - 1];
                        let child_idx = parent.children.len();
                        let parent_path = parent.path.clone();
                        parent.children.push(TreeNode {
                            id: format!("{}_{}", prop_name, child_idx),
                            path: format!("{}.{}", parent_path, child_idx),
                            node_type: "ui_property".to_string(),
                            content: format!("{}: {}", prop_name, prop_value),
                            start_line: i + 1,
                            end_line: i + 1,
                            children: Vec::new(),
                        });
                    }
                }
            }
        }

        Ok(TreeNode {
            id: "root".to_string(),
            path: "0".to_string(),
            node_type: "qml_file".to_string(),
            content: "QML".to_string(),
            start_line: 1,
            end_line: lines.len(),
            children: root_children,
        })
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["qml"]
    }
}

impl QmlParser {
    fn parse_property(&self, line: &str) -> Option<(String, String)> {
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            if !name.is_empty() && !value.is_empty() {
                return Some((name, value));
            }
        }
        None
    }
}
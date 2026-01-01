use crate::parser::{ParserEngine, TreeNode};
use anyhow::Result;

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

impl ParserEngine for QmlParser {
    fn parse(&self, code: &str) -> Result<TreeNode> {
        let mut root = TreeNode {
            id: "root".to_string(),
            path: "root".to_string(),
            node_type: "QmlDocument".to_string(),
            content: code.to_string(),
            start_line: 1,
            end_line: code.lines().count(),
            children: Vec::new(),
        };

        let lines: Vec<&str> = code.lines().collect();
        let mut current_component: Option<(TreeNode, Vec<TreeNode>)> = None;
        let mut component_stack: Vec<(TreeNode, Vec<TreeNode>, usize)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let depth = line.len() - trimmed.len();

            if trimmed.starts_with("import ") {
                root.children.push(TreeNode {
                    id: format!("root.{}", root.children.len()),
                    path: format!("root.{}", root.children.len()),
                    node_type: "Import".to_string(),
                    content: trimmed.to_string(),
                    start_line: i + 1,
                    end_line: i + 1,
                    children: Vec::new(),
                });
            } else if trimmed.ends_with("{") {
                let component_name = trimmed.strip_suffix('{').unwrap().trim();
                if !component_name.is_empty() {
                    let component = TreeNode {
                        id: format!("root.{}", root.children.len()),
                        path: format!("root.{}", root.children.len()),
                        node_type: component_name.to_string(),
                        content: String::new(),
                        start_line: i + 1,
                        end_line: i + 1,
                        children: Vec::new(),
                    };

                    if let Some((comp, props)) = current_component {
                        component_stack.push((comp, props, depth));
                    }

                    current_component = Some((component, Vec::new()));
                }
            } else if trimmed.starts_with("}") {
                if let Some((comp, props, _comp_depth)) = component_stack.pop() {
                    if let Some((ref mut root_comp, ref mut root_props)) = current_component {
                        root_comp.children = root_props.clone();
                        root_comp.end_line = i + 1;
                        root.children.push(std::mem::take(root_comp));
                    }
                    current_component = Some((comp, props));
                } else if let Some((ref mut comp, ref mut props)) = current_component {
                    comp.children = props.clone();
                    comp.end_line = i + 1;
                    root.children.push(std::mem::take(comp));
                    current_component = None;
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                if let Some((ref mut _comp, ref mut props)) = current_component {
                    if let Some((prop_name, prop_value)) = self.parse_property(trimmed) {
                        props.push(TreeNode {
                            id: format!("root.{}.{}", root.children.len(), props.len()),
                            path: format!("root.{}.{}", root.children.len(), props.len()),
                            node_type: "Property".to_string(),
                            content: format!("{}: {}", prop_name, prop_value),
                            start_line: i + 1,
                            end_line: i + 1,
                            children: Vec::new(),
                        });
                    }
                }
            }
        }

        Ok(root)
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

use crate::parser::{TreeNode, ParserEngineLegacy};
use anyhow::Result;
use serde_yaml::Value;

pub struct YamlParser;

impl Default for YamlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for YamlParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let value: Value = serde_yaml::from_str(code)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;

        // Build the root node
        let root = self.build_value_node(&value, "".to_string(), 1, 1)?;
        Ok(root)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["yaml", "yml"]
    }
}

impl YamlParser {
    #[allow(clippy::only_used_in_recursion)]
    fn build_value_node(
        &self,
        value: &Value,
        path: String,
        start_line: usize,
        end_line: usize,
    ) -> Result<TreeNode> {
        let (node_type, children) = match value {
            Value::String(_) => ("string".to_string(), vec![]),
            Value::Number(_) => ("number".to_string(), vec![]),
            Value::Bool(_) => ("boolean".to_string(), vec![]),
            Value::Null => ("null".to_string(), vec![]),
            Value::Tagged(_) => ("tagged".to_string(), vec![]),
            Value::Sequence(arr) => {
                let mut sequence_children = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    let child_path = if path.is_empty() {
                        i.to_string()
                    } else {
                        format!("{}.{}", path, i)
                    };
                    sequence_children
                        .push(self.build_value_node(item, child_path, start_line, end_line)?);
                }
                ("sequence".to_string(), sequence_children)
            }
            Value::Mapping(map) => {
                let mut mapping_children = Vec::new();
                for (key, val) in map.iter() {
                    let key_str = match key {
                        Value::String(s) => s.clone(),
                        _ => format!("{:?}", key),
                    };
                    let child_path = if path.is_empty() {
                        key_str.clone()
                    } else {
                        format!("{}.{}", path, key_str)
                    };
                    mapping_children
                        .push(self.build_value_node(val, child_path, start_line, end_line)?);
                }
                ("mapping".to_string(), mapping_children)
            }
        };

        let content = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Tagged(t) => format!("{:?}", t),
            Value::Sequence(_) | Value::Mapping(_) => "".to_string(),
        };

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
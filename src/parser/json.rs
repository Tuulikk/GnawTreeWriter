use crate::parser::{TreeNode, ParserEngineLegacy};
use anyhow::Result;
use serde_json::Value;

pub struct JsonParser;

impl Default for JsonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for JsonParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let value: Value = serde_json::from_str(code)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        // Build the root node
        let root = self.build_value_node(&value, "".to_string(), 1, 1)?;
        Ok(root)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["json"]
    }
}

impl JsonParser {
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
            Value::Array(arr) => {
                let mut array_children = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    let child_path = if path.is_empty() {
                        i.to_string()
                    } else {
                        format!("{}.{}", path, i)
                    };
                    array_children
                        .push(self.build_value_node(item, child_path, start_line, end_line)?);
                }
                ("array".to_string(), array_children)
            }
            Value::Object(obj) => {
                let mut object_children = Vec::new();
                for (key, val) in obj.iter() {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    object_children
                        .push(self.build_value_node(val, child_path, start_line, end_line)?);
                }
                ("object".to_string(), object_children)
            }
        };

        let content = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) | Value::Object(_) => "".to_string(),
        };

        let id = path.clone();

        Ok(TreeNode { start_col: 0, end_col: 0,
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
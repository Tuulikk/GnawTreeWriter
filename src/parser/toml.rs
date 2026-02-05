use crate::parser::{TreeNode, ParserEngineLegacy};
use anyhow::Result;
use toml::Value;

pub struct TomlParser;

impl Default for TomlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TomlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for TomlParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let value: Value = code
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse TOML: {}", e))?;

        // Build the root node
        let root = self.build_value_node(&value, "".to_string(), 1, 1)?;
        Ok(root)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["toml"]
    }
}

impl TomlParser {
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
            Value::Integer(_) => ("integer".to_string(), vec![]),
            Value::Float(_) => ("float".to_string(), vec![]),
            Value::Boolean(_) => ("boolean".to_string(), vec![]),
            Value::Datetime(_) => ("datetime".to_string(), vec![]),
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
            Value::Table(table) => {
                let mut table_children = Vec::new();
                for (key, val) in table.iter() {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    table_children
                        .push(self.build_value_node(val, child_path, start_line, end_line)?);
                }
                ("table".to_string(), table_children)
            }
        };

        let content = match value {
            Value::String(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Datetime(d) => d.to_string(),
            Value::Array(_) | Value::Table(_) => "".to_string(),
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
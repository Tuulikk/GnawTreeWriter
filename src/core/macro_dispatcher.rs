//! Macro-Aware Parser Layer
//!
//! Detects macro invocations (e.g. `json!({...})`) in the AST and
//! injects virtual sub-trees parsed by language-specific parsers.
//! This makes JSON inside `json!()` macros addressable as nodes.

use crate::core::TreeNode;
use anyhow::Result;
use std::collections::HashMap;

// ─── Trait ──────────────────────────────────────────────────────────────────

/// A parser that can parse the content of a specific macro's token tree.
pub trait MacroParser: Send + Sync {
    /// The macro name to match (e.g. "json" for `json!(...)`).
    fn macro_name(&self) -> &str;
    /// Parse `content` (the raw text inside the macro's `{...}`)
    /// into a list of virtual TreeNode children at `base_path`.
    fn parse_macro_body(&self, content: &str, base_path: &str) -> Result<Vec<TreeNode>>;
}

// ─── Dispatcher ─────────────────────────────────────────────────────────────

/// Registry of macro-name → parser mappings.
#[derive(Default)]
pub struct MacroDispatcher {
    parsers: HashMap<String, Box<dyn MacroParser>>,
}

impl MacroDispatcher {
    pub fn new() -> Self {
        Self { parsers: HashMap::new() }
    }

    pub fn register(&mut self, parser: Box<dyn MacroParser>) {
        let name = parser.macro_name().to_string();
        self.parsers.insert(name, parser);
    }

    pub fn get(&self, name: &str) -> Option<&dyn MacroParser> {
        self.parsers.get(name).map(|b| b.as_ref())
    }

    pub fn has(&self, name: &str) -> bool {
        self.parsers.contains_key(name)
    }
}

/// Global dispatcher (lazy-initialized with default parsers).
static DISPATCHER: std::sync::OnceLock<MacroDispatcher> = std::sync::OnceLock::new();

pub fn dispatcher() -> &'static MacroDispatcher {
    DISPATCHER.get_or_init(|| {
        let mut d = MacroDispatcher::new();
        d.register(Box::new(JsonMacroParser));
        d
    })
}

// ─── JSON Macro Parser ──────────────────────────────────────────────────────

struct JsonMacroParser;

impl MacroParser for JsonMacroParser {
    fn macro_name(&self) -> &str {
        "json"
    }

    fn parse_macro_body(&self, content: &str, base_path: &str) -> Result<Vec<TreeNode>> {
        let value: serde_json::Value = serde_json::from_str(content)?;
        Ok(vec![json_value_to_tree(&value, base_path)])
    }
}

fn json_value_to_tree(value: &serde_json::Value, base_path: &str) -> TreeNode {
    let (node_type, content, children) = match value {
        serde_json::Value::Object(map) => {
            let mut kids = Vec::new();
            for (i, (key, val)) in map.iter().enumerate() {
                // Even = key node, Odd = value node
                let key_path = format!("{}.{}", base_path, i * 2);
                kids.push(TreeNode {
                    id: key_path.clone(), path: key_path,
                    node_type: format!("json_key:{}", key), content: key.clone(),
                    start_line: 0, end_line: 0, start_col: 0, end_col: 0,
                    children: vec![],
                });
                let val_path = format!("{}.{}", base_path, i * 2 + 1);
                kids.push(json_value_to_tree(val, &val_path));
            }
            ("json_object", content_str(value), kids)
        }
        serde_json::Value::Array(arr) => {
            let mut kids = Vec::new();
            for (i, val) in arr.iter().enumerate() {
                let child_path = format!("{}.{}", base_path, i);
                kids.push(json_value_to_tree(val, &child_path));
            }
            ("json_array", content_str(value), kids)
        }
        serde_json::Value::String(s) => ("json_string", format!("\"{}\"", s), vec![]),
        serde_json::Value::Number(n) => ("json_number", n.to_string(), vec![]),
        serde_json::Value::Bool(b) => ("json_bool", b.to_string(), vec![]),
        serde_json::Value::Null => ("json_null", "null".to_string(), vec![]),
    };
    TreeNode {
        id: base_path.to_string(),
        path: base_path.to_string(),
        node_type: node_type.to_string(),
        content,
        start_line: 0, end_line: 0, start_col: 0, end_col: 0,
        children,
    }
}

fn content_str(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(_) => "{ ... }".to_string(),
        serde_json::Value::Array(_) => "[ ... ]".to_string(),
        other => other.to_string(),
    }
}

// ─── Integration helpers ────────────────────────────────────────────────────

/// Extract the macro name from a `macro_invocation` node.
/// Returns `None` if the node is not a macro invocation or has no identifier.
pub fn extract_macro_name(node: &TreeNode) -> Option<&str> {
    if node.node_type != "macro_invocation" {
        return None;
    }
    // The first child of a macro_invocation is the macro name identifier
    node.children.first().map(|c| c.content.as_str())
}

/// Check if a TreeNode represents a token_tree that could contain macro content.
pub fn is_token_tree(node: &TreeNode) -> bool {
    node.node_type == "token_tree"
}

/// Parse token tree content through the dispatcher if a matching macro exists.
pub fn try_expand_macro(macro_name: &str, token_content: &str, base_path: &str) -> Option<Vec<TreeNode>> {
    let d = dispatcher();
    let parser = d.get(macro_name)?;
    parser.parse_macro_body(token_content, base_path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_macro_parses_object() {
        let content = r#"{"name": "analyze", "version": 1}"#;
        let result = JsonMacroParser.parse_macro_body(content, "0").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].node_type, "json_object");
    }

    #[test]
    fn test_json_macro_parses_array() {
        let content = r#"["a", "b", 42]"#;
        let result = JsonMacroParser.parse_macro_body(content, "0").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].node_type, "json_array");
        // Array has 3 children
        assert_eq!(result[0].children.len(), 3);
    }

    #[test]
    fn test_json_macro_keys_are_addressable() {
        let content = r#"{"tools": [{"name": "test"}]}"#;
        let result = JsonMacroParser.parse_macro_body(content, "0").unwrap();
        let obj = &result[0];
        // tools key at 0 (even)
        assert_eq!(obj.children[0].node_type, "json_key:tools");
        // tools value at 1 (odd) - json_array
        assert_eq!(obj.children[1].node_type, "json_array");
        let arr = &obj.children[1];
        assert_eq!(arr.children.len(), 1);
        assert_eq!(arr.children[0].node_type, "json_object");
        let inner = &arr.children[0];
        // name key at 0
        assert_eq!(inner.children[0].node_type, "json_key:name");
        // name value at 1
        assert_eq!(inner.children[1].node_type, "json_string");
        assert_eq!(inner.children[1].content, r#""test""#);
    }

    #[test]
    fn test_dispatcher_registers_json() {
        let d = dispatcher();
        assert!(d.has("json"));
        assert!(d.get("json").is_some());
    }

    #[test]
    fn test_extract_macro_name() {
        let macro_node = TreeNode {
            id: "0".into(),
            path: "0".into(),
            node_type: "macro_invocation".into(),
            content: "json!(...)".into(),
            start_line: 1, end_line: 1, start_col: 0, end_col: 0,
            children: vec![TreeNode {
                id: "0.0".into(),
                path: "0.0".into(),
                node_type: "identifier".into(),
                content: "json".into(),
                start_line: 1, end_line: 1, start_col: 0, end_col: 0,
                children: vec![],
            }],
        };
        assert_eq!(extract_macro_name(&macro_node), Some("json"));

        let not_macro = TreeNode {
            id: "0".into(), path: "0".into(), node_type: "function_item".into(),
            content: "fn foo".into(), start_line: 1, end_line: 1, start_col: 0, end_col: 0,
            children: vec![],
        };
        assert_eq!(extract_macro_name(&not_macro), None);
    }
}

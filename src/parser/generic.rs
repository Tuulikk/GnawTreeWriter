use crate::parser::{ParserEngine, TreeNode};
use anyhow::Result;

/// Node type used for generic (unknown) file parsing.
pub const GENERIC_NODE_TYPE: &str = "generic";

/// Generic parser for unknown file types.
///
/// The parser treats the entire file as a single node (id = "0", path = "0")
/// and stores the file contents in the `content` field. This enables
/// project-wide backups, history and basic edits for files that do not have
/// a dedicated AST parser.
pub struct GenericParser;

impl Default for GenericParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl ParserEngine for GenericParser {
    /// Parse the entire file as a single node.
    fn parse(&self, code: &str) -> Result<TreeNode> {
        let lines = code.lines().collect::<Vec<&str>>();
        let line_count = lines.len();

        Ok(TreeNode {
            id: "0".to_string(),
            path: "0".to_string(),
            node_type: GENERIC_NODE_TYPE.to_string(),
            content: code.to_string(),
            start_line: 1,
            end_line: if line_count == 0 { 1 } else { line_count },
            children: Vec::new(),
        })
    }

    /// Generic parser does not advertise specific extensions - it is a fallback.
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_file() -> Result<()> {
        let p = GenericParser::new();
        let tree = p.parse("")?;
        assert_eq!(tree.id, "0");
        assert_eq!(tree.path, "0");
        assert_eq!(tree.node_type, GENERIC_NODE_TYPE);
        assert_eq!(tree.content, "");
        assert_eq!(tree.start_line, 1);
        assert_eq!(tree.end_line, 1);
        assert!(tree.children.is_empty());
        Ok(())
    }

    #[test]
    fn parse_multi_line_file() -> Result<()> {
        let code = "line one\nline two\nline three\n";
        let p = GenericParser::new();
        let tree = p.parse(code)?;
        assert_eq!(tree.node_type, GENERIC_NODE_TYPE);
        assert_eq!(tree.content, code);
        assert_eq!(tree.start_line, 1);
        assert_eq!(tree.end_line, 3);
        Ok(())
    }
}

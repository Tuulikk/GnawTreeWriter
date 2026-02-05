use crate::parser::{ParserEngine, TreeNode, ParseResult};

/// Node type used for generic (unknown) file parsing.
pub const GENERIC_NODE_TYPE: &str = "generic";

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
    fn parse(&self, code: &str) -> ParseResult<TreeNode> {
        let lines = code.lines().collect::<Vec<&str>>();
        let line_count = lines.len();

        Ok(TreeNode { start_col: 0, end_col: 0, 
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
    fn parse_empty_file() -> anyhow::Result<()> {
        let p = GenericParser::new();
        let tree = p.parse("")?;
        assert_eq!(tree.id, "0");
        Ok(())
    }
}
use crate::parser::{ParserEngine, TreeNode, ParseResult, SyntaxError};

pub struct PythonParser;

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    pub fn new() -> Self {
        Self
    }

    fn find_error<'a>(&self, node: &tree_sitter::Node<'a>, _cursor: &mut tree_sitter::TreeCursor<'a>) -> Option<tree_sitter::Node<'a>> {
        if node.is_error() || node.is_missing() {
            return Some(*node);
        }
        let mut child_cursor = node.walk();
        for child in node.children(&mut child_cursor) {
            let mut recursive_cursor = child.walk();
            if let Some(err) = self.find_error(&child, &mut recursive_cursor) {
                return Some(err);
            }
        }
        None
    }

    fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> anyhow::Result<TreeNode> {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let content = if let Some(s) = source.get(start_byte..end_byte) {
            s.to_string()
        } else {
            String::new()
        };

        let node_type = node.kind().to_string();
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        let mut children = Vec::new();
        let mut cursor = node.walk();

        for (i, child) in node.children(&mut cursor).enumerate() {
            let child_path = if path.is_empty() {
                i.to_string()
            } else {
                format!("{}.{}", path, i)
            };
            children.push(Self::build_tree(&child, source, child_path)?);
        }

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

impl ParserEngine for PythonParser {
    fn parse(&self, code: &str) -> ParseResult<TreeNode> {
        let mut parser = tree_sitter::Parser::new();
        let language = unsafe {
            std::mem::transmute::<tree_sitter_language::LanguageFn, fn() -> tree_sitter::Language>(
                tree_sitter_python::LANGUAGE,
            )()
        };
        if let Err(e) = parser.set_language(&language) {
            return Err(SyntaxError::from(anyhow::anyhow!("Failed to set Python language: {}", e)));
        }

        let tree = parser
            .parse(code, None)
            .ok_or_else(|| SyntaxError::from(anyhow::anyhow!("Failed to parse Python: No tree returned")))?;

        if tree.root_node().has_error() {
            let mut cursor = tree.walk();
            if let Some(err_node) = self.find_error(&tree.root_node(), &mut cursor) {
                return Err(SyntaxError {
                    message: "Syntax error in Python code".to_string(),
                    line: err_node.start_position().row + 1,
                    column: err_node.start_position().column + 1,
                    expected: None,
                });
            }
        }

        crate::parser::to_parse_result(Self::build_tree(&tree.root_node(), code, "".to_string()))
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["py"]
    }
}
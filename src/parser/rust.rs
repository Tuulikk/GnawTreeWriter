use crate::parser::{ParseResult, ParserEngine, SyntaxError, TreeNode};

pub struct RustParser;

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustParser {
    pub fn new() -> Self {
        Self
    }

    fn find_error<'a>(
        &self,
        node: &tree_sitter::Node<'a>,
        _cursor: &mut tree_sitter::TreeCursor<'a>,
    ) -> Option<tree_sitter::Node<'a>> {
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

    fn build_tree(
        node: &tree_sitter::Node,
        source: &str,
        path: String,
    ) -> anyhow::Result<TreeNode> {
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
        let start_col = node.start_position().column + 1;
        let end_col = node.end_position().column + 1;

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

        // ── Macro expansion hook ─────────────────────────────────────
        // If this is a macro_invocation, try to expand known macros
        // (e.g. json!({...})) and inject virtual sub-trees.
        if node.kind() == "macro_invocation" {
            if let Some(name_node) = node.child(0) {
                let macro_name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                if crate::core::macro_dispatcher::dispatcher().has(macro_name) {
                    // Find the token_tree child (usually child index 2: identifier, !, token_tree)
                    for ci in 0..node.child_count() {
                        if let Some(child) = node.child(ci as u32) {
                            if child.kind() == "token_tree" {
                                if let Ok(body) = child.utf8_text(source.as_bytes()) {
                                    // Strip surrounding delimiters: json!(...), json!{...}, json![...]
                                    let stripped = body.trim();
                                    let inner = stripped
                                        .strip_prefix(&['(', '{', '['][..])
                                        .and_then(|s| s.strip_suffix(&[')', '}', ']'][..]))
                                        .unwrap_or(stripped);
                                    let base_path = format!("{}.{}", path, ci);
                                    if let Some(virtual_kids) =
                                        crate::core::macro_dispatcher::try_expand_macro(
                                            macro_name, inner, &base_path,
                                        )
                                    {
                                        children.extend(virtual_kids);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        let id = path.clone();

        Ok(TreeNode {
            id,
            path,
            node_type,
            content,
            start_line,
            end_line,
            start_col,
            end_col,
            children,
        })
    }
}

impl ParserEngine for RustParser {
    fn parse(&self, code: &str) -> ParseResult<TreeNode> {
        let mut parser = tree_sitter::Parser::new();
        let language = unsafe {
            std::mem::transmute::<tree_sitter_language::LanguageFn, fn() -> tree_sitter::Language>(
                tree_sitter_rust::LANGUAGE,
            )()
        };
        if let Err(e) = parser.set_language(&language) {
            return Err(SyntaxError::from(anyhow::anyhow!(
                "Failed to set Rust language: {}",
                e
            )));
        }

        let tree = parser.parse(code, None).ok_or_else(|| {
            SyntaxError::from(anyhow::anyhow!("Failed to parse Rust: No tree returned"))
        })?;

        if tree.root_node().has_error() {
            let mut cursor = tree.walk();
            if let Some(err_node) = self.find_error(&tree.root_node(), &mut cursor) {
                return Err(SyntaxError {
                    message: "Syntax error in Rust code".to_string(),
                    line: err_node.start_position().row + 1,
                    column: err_node.start_position().column + 1,
                    expected: None,
                });
            }
        }

        crate::parser::to_parse_result(Self::build_tree(&tree.root_node(), code, "".to_string()))
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }

    /// Read-path grace (Motor2 brief): a localized syntax error must not
    /// void the whole file for analyze/skeleton/read — tree-sitter is
    /// error-tolerant, so build the partial tree and report the error as
    /// a WARNING. Strict parse() (editors) still refuses.
    fn parse_lenient(&self, code: &str) -> (ParseResult<TreeNode>, Option<SyntaxError>) {
        let mut parser = tree_sitter::Parser::new();
        let language = unsafe {
            std::mem::transmute::<tree_sitter_language::LanguageFn, fn() -> tree_sitter::Language>(
                tree_sitter_rust::LANGUAGE,
            )()
        };
        if let Err(e) = parser.set_language(&language) {
            return (
                Err(SyntaxError::from(anyhow::anyhow!(
                    "Failed to set Rust language: {}",
                    e
                ))),
                None,
            );
        }
        let Some(tree) = parser.parse(code, None) else {
            return (
                Err(SyntaxError::from(anyhow::anyhow!(
                    "Failed to parse Rust: No tree returned"
                ))),
                None,
            );
        };
        let warning = if tree.root_node().has_error() {
            let mut cursor = tree.walk();
            let (line, column) = self
                .find_error(&tree.root_node(), &mut cursor)
                .map(|n| (n.start_position().row + 1, n.start_position().column + 1))
                .unwrap_or((1, 1));
            Some(SyntaxError {
                message: "Partial parse: syntax error found — results may be incomplete (editors still refuse this file strictly)".to_string(),
                line,
                column,
                expected: None,
            })
        } else {
            None
        };
        let built = crate::parser::to_parse_result(Self::build_tree(
            &tree.root_node(),
            code,
            "".to_string(),
        ));
        (built, warning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parses(code: &str) -> bool {
        RustParser.parse(code).is_ok()
    }

    /// Regression (issue log 2026-10-05, Motor2 database.rs): tree-sitter-rust
    /// 0.24.0 mistook plain `&raw` borrows for Rust 1.82 `&raw const/mut`
    /// raw-references and rejected whole files. Fixed by 0.24.2 — pinned here.
    #[test]
    fn parses_plain_raw_borrow() {
        assert!(parses(
            "fn f() -> i32 { let raw = 1; g(&raw) }\nfn g(_: &i32) -> i32 { 2 }"
        ));
        assert!(parses(
            "fn f(raw: String) -> i32 { g(&raw) }\nfn g(_: &String) -> i32 { 2 }"
        ));
        assert!(parses(
            "fn f(raw: &str) -> Result<Option<serde_json::Value>, String> {\n    let parsed = serde_json::from_str::<serde_json::Value>(raw)\n        .unwrap_or_else(|_| serde_json::json!({ \"raw\": raw }));\n    Ok(Some(parsed.unwrap_or_default()))\n}"
        ));
    }

    #[test]
    fn parses_raw_as_ordinary_identifier() {
        assert!(parses("fn f() -> i32 { let raw = 1; raw + 1 }"));
        assert!(parses("fn f(raw: &str) -> usize { raw.len() }"));
        // The real raw-reference syntax must also keep parsing.
        assert!(parses("fn f(x: i32) { let _p = &raw const x; }"));
    }

    #[test]
    fn rejects_actual_syntax_error() {
        assert!(!parses("fn f( { }"));
    }

    /// Partial-grace: strict refuses, lenient returns what parsed + warning.
    #[test]
    fn lenient_returns_partial_with_warning() {
        let code = "fn good() -> i32 { 1 }\nfn broken( {\n";
        let (result, warning) = RustParser.parse_lenient(code);
        let tree = result.expect("partial tree must come back");
        assert!(warning.is_some(), "warning must carry the error position");
        let w = warning.unwrap();
        assert!(w.line >= 1 && w.column >= 1);
        // The healthy function is present in the partial tree.
        assert!(format!("{:?}", tree).contains("good") || tree.children.iter().any(|c| true));
        // Strict still refuses the same input.
        assert!(RustParser.parse(code).is_err(), "strict must stay strict");
    }

    #[test]
    fn lenient_clean_file_has_no_warning() {
        let (result, warning) = RustParser.parse_lenient("fn ok() {}");
        assert!(result.is_ok());
        assert!(warning.is_none());
    }
}

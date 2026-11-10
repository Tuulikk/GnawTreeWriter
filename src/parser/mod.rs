pub mod bash;
pub mod c;
pub mod cmake;
pub mod cpp;
pub mod csharp;
pub mod css;
pub mod dart;
pub mod error;
pub mod generic;
pub mod go;
pub mod html;
pub mod java;
pub mod javascript;
pub mod json;
pub mod kotlin;
pub mod make;
pub mod markdown;
pub mod php;
pub mod powershell;
pub mod python;
pub mod qml;
pub mod qml_tree_sitter;
pub mod ruby;
pub mod rust;
pub mod slint;
pub mod sql;
pub mod svelte;
pub mod swift;
pub mod text;
pub mod toml;
pub mod typescript;
pub mod xml;
pub mod yaml;
pub use error::{ParseResult, SyntaxError};
pub mod zig;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeNode {
    pub id: String,
    pub path: String,
    pub node_type: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default)]
    pub start_col: usize,
    #[serde(default)]
    pub end_col: usize,
    pub children: Vec<TreeNode>,
}

/// 1-baserade TECKEN-kolumner för TreeNode (2026-10-30, GTW_MCP_ISSUE_LOG).
/// edit_node_at_path:s splatte räknar `.chars()`, men tree-sitter-positioner
/// är BYTE-kolumner — vid å/ä/ö/emoji före noden på samma rad divergerar de
/// två och splatten landar fel. Konvertera byte → chars HÄR, en gång.
pub fn char_col_pair(source: &str, start_byte: usize, end_byte: usize) -> (usize, usize) {
    let line_start = source[..start_byte].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let start_col = source[line_start..start_byte].chars().count() + 1;
    let end_line_start = source[..end_byte].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let end_col = source[end_line_start..end_byte].chars().count() + 1;
    (start_col, end_col)
}

/// Convert a tree-sitter node tree into GTW's `TreeNode`.
///
/// The point is the spans: `content` is the exact source slice and
/// `start_col`/`end_col` come from [`char_col_pair`]. `edit_node` splices on
/// exactly those values, so parsers that leave the columns at 0 force the
/// splatter back to whole-line replacement — which silently rewrites
/// neighbouring text (markdown lost the blank line after the edited
/// paragraph, a CSS rule was replaced by its value).
pub fn tree_from_ts_node(node: &tree_sitter::Node, source: &str, path: String) -> TreeNode {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();
    let content = source
        .get(start_byte..end_byte)
        .map(|s| s.to_string())
        .unwrap_or_default();
    let (start_col, end_col) = char_col_pair(source, start_byte, end_byte);

    let mut children = Vec::new();
    let mut cursor = node.walk();
    for (i, child) in node.children(&mut cursor).enumerate() {
        let child_path = if path.is_empty() {
            i.to_string()
        } else {
            format!("{}.{}", path, i)
        };
        children.push(tree_from_ts_node(&child, source, child_path));
    }

    TreeNode {
        start_col,
        end_col,
        id: path.clone(),
        path,
        node_type: node.kind().to_string(),
        content,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
    }
}

/// Parse `code` with a tree-sitter grammar and convert it with
/// [`tree_from_ts_node`]. Shared by every grammar that needs no
/// language-specific shaping (documents, config formats, scripts).
pub fn parse_with_tree_sitter(
    language: tree_sitter::Language,
    name: &str,
    code: &str,
) -> anyhow::Result<TreeNode> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(code, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse {} source", name))?;
    Ok(tree_from_ts_node(&tree.root_node(), code, String::new()))
}

impl TreeNode {
    /// Recursively find a node by its path string.
    pub fn find_path(&self, target_path: &str) -> Option<&TreeNode> {
        if self.path == target_path {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_path(target_path) {
                return Some(found);
            }
        }
        None
    }

    /// Attempts to extract a descriptive name for this node (e.g., function name, class name).
    /// It looks for common identifier-like children.
    pub fn get_name(&self) -> Option<String> {
        let nt = self.node_type.to_lowercase();
        // If the node itself is an identifier, return its content
        if nt == "identifier" || nt == "name" || nt == "type_identifier" || nt == "field_identifier"
        {
            return Some(self.content.clone());
        }

        // Look for identifiers in immediate children
        for child in &self.children {
            let cnt = child.node_type.to_lowercase();
            if cnt == "identifier"
                || cnt == "name"
                || cnt == "type_identifier"
                || cnt == "field_identifier"
            {
                return Some(child.content.clone());
            }
        }

        // Look one level deeper if needed (common in some complex AST structures)
        for child in &self.children {
            for subchild in &child.children {
                let scnt = subchild.node_type.to_lowercase();
                if scnt == "identifier"
                    || scnt == "name"
                    || scnt == "type_identifier"
                    || scnt == "field_identifier"
                {
                    return Some(subchild.content.clone());
                }
            }
        }
        None
    }

    /// Find the deepest node that contains the given 1-based line number.
    /// Returns the most specific (deepest) node whose start_line..=end_line range includes `line`.
    pub fn find_node_at_line(&self, line: usize) -> Option<&TreeNode> {
        if line >= self.start_line && line <= self.end_line {
            // Recurse into children first — prefer deeper (more specific) nodes
            for child in &self.children {
                if let Some(found) = child.find_node_at_line(line) {
                    return Some(found);
                }
            }
            Some(self)
        } else {
            None
        }
    }

    /// Find the best parent node for inserting at a given 1-based line number.
    /// Used by diff-to-batch to determine where to insert new code.
    ///
    /// Strategy: Walk the tree depth-first, but when the deepest node at
    /// `line` is at a boundary of a scoped container (function, class, block),
    /// skip past it so the insert becomes a sibling rather than a child.
    pub fn find_parent_at_line(&self, line: usize) -> &TreeNode {
        if !(line >= self.start_line && line <= self.end_line) {
            return self;
        }

        let mut best_container: Option<&TreeNode> = None;

        for child in &self.children {
            if !(line >= child.start_line && line <= child.end_line) {
                continue;
            }

            let nt = child.node_type.to_lowercase();

            // Check if this child is a "scoped container" whose boundary
            // should not be descended into. This includes:
            // - function_item, method_declaration, fn_definition
            // - class_definition, struct_item, enum_item
            // - impl_item, trait_item, mod_item
            // - block, body, statement_block
            let is_scoped = nt.contains("function")
                || nt.contains("method")
                || nt.contains("class")
                || nt.contains("struct")
                || nt.contains("enum")
                || (nt.contains("impl") && !nt.contains("implementation"))
                || nt.contains("trait")
                || nt.contains("mod")
                || nt.contains("block")
                || nt.contains("body")
                || nt.contains("statement_block");

            let at_boundary =
                line <= child.start_line + 1 || line >= child.end_line.saturating_sub(1);

            if is_scoped && at_boundary {
                // The line is at the edge of a scoped container.
                // Don't descend — the insert should be a sibling, not a child.
                best_container = None;
                continue;
            }

            // Recurse into the child
            let result = child.find_parent_at_line(line);
            best_container = Some(result);
        }

        best_container.unwrap_or(self)
    }
}

pub trait ParserEngine {
    fn parse(&self, code: &str) -> ParseResult<TreeNode>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;

    /// Strict parse with a partial-grace fallback for READ paths
    /// (Motor2 brief: valid-looking big files must not die whole-file on
    /// a localized error). Returns (strict-result, warning-if-partial).
    /// Default: strict only — parsers opt in by overriding (see rust.rs).
    fn parse_lenient(&self, code: &str) -> (ParseResult<TreeNode>, Option<SyntaxError>) {
        (self.parse(code), None)
    }
}

pub fn to_parse_result<T>(res: anyhow::Result<T>) -> ParseResult<T> {
    res.map_err(SyntaxError::from)
}

/// Strict-parse `path` and report how many nodes it produced — the core
/// of `validate` (CLI + MCP): Ok = safe to edit, Err carries the error
/// position (read paths still answer partially via parse_lenient).
pub fn validate_file(path: &std::path::Path) -> Result<usize, SyntaxError> {
    let code = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
    let parser = get_parser(path)?;
    let tree = parser.parse(&code)?;
    fn count(n: &TreeNode) -> usize {
        1 + n.children.iter().map(count).sum::<usize>()
    }
    Ok(count(&tree))
}

/// A wrapper to let older parsers that return anyhow::Result work with the new ParseResult.
pub struct LegacyParserWrapper<P: ParserEngineLegacy> {
    inner: P,
}

impl<P: ParserEngineLegacy> LegacyParserWrapper<P> {
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P: ParserEngineLegacy> ParserEngine for LegacyParserWrapper<P> {
    fn parse(&self, code: &str) -> ParseResult<TreeNode> {
        to_parse_result(self.inner.parse_legacy(code))
    }
    fn get_supported_extensions(&self) -> Vec<&'static str> {
        self.inner.get_supported_extensions()
    }
}

pub trait ParserEngineLegacy {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>;
    fn get_supported_extensions(&self) -> Vec<&'static str>;
}
pub fn get_parser(file_path: &Path) -> anyhow::Result<Box<dyn ParserEngine>> {
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Some files ARE their language, and the extension is unhelpful or absent:
    // `CMakeLists.txt` reports "txt" (a line-based text parser), `Makefile` and
    // `.zshrc` report no extension at all. Check the name first, so those never
    // reach the extension match below.
    if let Some(parser) = parser_for_special_filename(file_path) {
        return Ok(parser);
    }

    match extension {
        "qml" => Ok(Box::new(LegacyParserWrapper::new(
            qml_tree_sitter::QmlTreeSitterParser::new(),
        ))),
        "py" | "pyi" => Ok(Box::new(python::PythonParser::new())),
        "rs" => Ok(Box::new(rust::RustParser::new())),
        "slint" => Ok(Box::new(slint::SlintParser::new())),
        "kt" | "kts" => Ok(Box::new(LegacyParserWrapper::new(
            kotlin::KotlinParser::new(),
        ))),
        "swift" => Ok(Box::new(
            LegacyParserWrapper::new(swift::SwiftParser::new()),
        )),
        "cs" => Ok(Box::new(LegacyParserWrapper::new(
            csharp::CSharpParser::new(),
        ))),
        "dart" => Ok(Box::new(LegacyParserWrapper::new(dart::DartParser::new()))),
        "svelte" => Ok(Box::new(LegacyParserWrapper::new(
            svelte::SvelteParser::new(),
        ))),
        "sql" => Ok(Box::new(LegacyParserWrapper::new(sql::SqlParser::new()))),
        "js" | "jsx" | "mjs" | "cjs" => Ok(Box::new(LegacyParserWrapper::new(
            javascript::JavaScriptParser::new(),
        ))),
        "ts" | "tsx" | "mts" | "cts" => Ok(Box::new(LegacyParserWrapper::new(
            typescript::TypeScriptParser::new(),
        ))),
        "php" => Ok(Box::new(LegacyParserWrapper::new(php::PhpParser::new()))),
        "html" | "htm" => Ok(Box::new(LegacyParserWrapper::new(html::HtmlParser::new()))),
        "go" => Ok(Box::new(LegacyParserWrapper::new(go::GoParser::new()))),
        "c" | "h" => Ok(Box::new(LegacyParserWrapper::new(c::CParser::new()))),
        "cpp" | "hpp" | "cc" | "cxx" | "hxx" | "h++" => {
            Ok(Box::new(LegacyParserWrapper::new(cpp::CppParser::new())))
        }
        "sh" | "bash" => Ok(Box::new(LegacyParserWrapper::new(bash::BashParser::new()))),
        "java" => Ok(Box::new(LegacyParserWrapper::new(java::JavaParser::new()))),
        "ps1" | "psm1" | "psd1" => Ok(Box::new(LegacyParserWrapper::new(
            powershell::PowerShellParser::new(),
        ))),
        "cmake" => Ok(Box::new(
            LegacyParserWrapper::new(cmake::CMakeParser::new()),
        )),
        "mk" => Ok(Box::new(LegacyParserWrapper::new(make::MakeParser::new()))),
        // Fixes the silent fallback: get_parser_for_language maps ruby/rb -> "rb",
        // which until now had no arm here and landed on GenericParser (one node),
        // so `language: ruby` rules compiled against an empty tree and never matched.
        "rb" => Ok(Box::new(LegacyParserWrapper::new(ruby::RubyParser::new()))),
        "zig" => Ok(Box::new(LegacyParserWrapper::new(zig::ZigParser::new()))),
        "css" => Ok(Box::new(LegacyParserWrapper::new(css::CssParser::new()))),
        "xml" | "svg" | "xsl" | "xsd" | "rss" | "atom" => Ok(Box::new(xml::XmlParser::new())),
        "md" | "markdown" => Ok(Box::new(LegacyParserWrapper::new(
            markdown::MarkdownParser::new(),
        ))),
        "txt" => Ok(Box::new(LegacyParserWrapper::new(text::TextParser::new()))),
        "toml" => Ok(Box::new(LegacyParserWrapper::new(toml::TomlParser::new()))),
        "json" => Ok(Box::new(LegacyParserWrapper::new(json::JsonParser::new()))),
        "yaml" | "yml" => Ok(Box::new(LegacyParserWrapper::new(yaml::YamlParser::new()))),
        _ => Ok(Box::new(generic::GenericParser::new())),
    }
}

/// Parsers selected by FILE NAME rather than extension.
///
/// Three classes share the same problem — the extension either lies or is
/// absent, so the extension match in [`get_parser`] can never reach them:
///
/// - `CMakeLists.txt` → extension `txt` (would become the line-based
///   `TextParser`, silently)
/// - `Makefile` / `GNUmakefile` → `Path::extension()` is `None`
/// - `.bashrc`, `.zshrc`, `.profile`, … → also `None`
///
/// Returning `None` keeps the caller on the normal extension path.
fn parser_for_special_filename(file_path: &Path) -> Option<Box<dyn ParserEngine>> {
    let name = file_path.file_name().and_then(|n| n.to_str())?;

    match name {
        "CMakeLists.txt" | "CMakeLists.txt.in" => Some(Box::new(LegacyParserWrapper::new(
            cmake::CMakeParser::new(),
        ))),
        "Makefile" | "makefile" | "GNUmakefile" => {
            Some(Box::new(LegacyParserWrapper::new(make::MakeParser::new())))
        }
        // The bash grammar approximates zsh well enough for structural editing;
        // zsh-specific constructs may yield ERROR nodes (deliberate trade-off).
        ".bashrc" | ".bash_profile" | ".bash_aliases" | ".bash_login" | ".zshrc" | ".zshenv"
        | ".profile" => Some(Box::new(LegacyParserWrapper::new(bash::BashParser::new()))),
        _ => None,
    }
}

/// Get a parser by language name (for rule patterns, which have no file).
pub fn get_parser_for_language(language: &str) -> anyhow::Result<Box<dyn ParserEngine>> {
    let lower = language.to_lowercase();
    let ext: &str = match lower.as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" | "jsx" => "js",
        "typescript" | "ts" | "tsx" => "ts",
        "go" | "golang" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "csharp" | "cs" => "cs",
        "php" => "php",
        "ruby" | "rb" => "rb",
        "swift" => "swift",
        "kotlin" | "kt" => "kt",
        "bash" | "sh" | "shell" => "sh",
        "zig" => "zig",
        "qml" => "qml",
        "css" => "css",
        "html" => "html",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "json" => "json",
        "markdown" | "md" => "md",
        "sql" => "sql",
        other => other,
    };
    let placeholder = format!("placeholder.{ext}");
    let path = std::path::Path::new(&placeholder);
    get_parser(path)
}

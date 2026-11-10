// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Markdown, parsed with the tree-sitter grammar tree_sitter_md.
//!
//! Replaces the regex/line scanner. That parser gave heading nodes a span covering the whole line while content held only the text, so edit_node wrote the text back and DELETED the #; it also rebuilt block content instead of slicing the source and left start_col/end_col at 0, forcing whole-line replacement (a paragraph edit swallowed the following blank line). Tree-sitter gives byte-exact spans and nests each heading section under the previous one.

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct MarkdownParser;

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for MarkdownParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_md::LANGUAGE.into(), "Markdown", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["md", "markdown"]
    }
}

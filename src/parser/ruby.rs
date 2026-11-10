// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Ruby, parsed with the tree-sitter grammar tree_sitter_ruby.
//!
//! A repair, not a feature: get_parser_for_language mapped ruby/rb to "rb" but get_parser had no "rb" arm, so Ruby silently fell through to GenericParser (a single node) and language: ruby rules compiled against an empty tree and never matched.

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct RubyParser;

impl Default for RubyParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RubyParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for RubyParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_ruby::LANGUAGE.into(), "Ruby", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["rb"]
    }
}

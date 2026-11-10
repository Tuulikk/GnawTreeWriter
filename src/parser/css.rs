// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Css, parsed with the tree-sitter grammar tree_sitter_css.
//!
//! Replaces the custom parser whose spans had width zero: editing a declaration value replaced the ENTIRE rule line with the value. tree-sitter-css was disabled in Cargo.toml citing version conflicts; that no longer holds (probe: no second runtime against tree-sitter 0.26.13).

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct CssParser;

impl Default for CssParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CssParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for CssParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_css::LANGUAGE.into(), "Css", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["css"]
    }
}

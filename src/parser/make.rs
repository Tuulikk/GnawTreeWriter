// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Makefile, parsed with the tree-sitter grammar tree_sitter_make.
//!
//! Makefiles are reached by NAME, not extension: get_parser keys on Path::extension(), which is None for Makefile. See parser_for_extensionless in this module.

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct MakeParser;

impl Default for MakeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for MakeParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_make::LANGUAGE.into(), "Makefile", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["mk"]
    }
}

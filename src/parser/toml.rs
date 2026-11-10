// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Toml, parsed with the tree-sitter grammar tree_sitter_toml_ng.
//!
//! Replaces the serde-based parser, position-blind in the same way (line 1, columns 0-0) and likewise unusable for edits. tree-sitter-toml-ng reports pair nodes, so a value can be edited without touching its key.

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct TomlParser;

impl Default for TomlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TomlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for TomlParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_toml_ng::LANGUAGE.into(), "Toml", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["toml"]
    }
}

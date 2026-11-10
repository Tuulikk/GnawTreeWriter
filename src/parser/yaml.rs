// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Yaml, parsed with the tree-sitter grammar tree_sitter_yaml.
//!
//! Replaces the serde-based parser, whose nodes had no positions at all (every node reported line 1, columns 0-0) and whose paths were mapping keys. Edits were therefore refused by Duplex validation, making YAML effectively read-only.

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct YamlParser;

impl Default for YamlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for YamlParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_yaml::LANGUAGE.into(), "Yaml", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["yaml", "yml"]
    }
}

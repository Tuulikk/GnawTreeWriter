// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CMake, parsed with the tree-sitter grammar tree_sitter_cmake.
//!
//! CMake sits next to the C/C++ surface GTW already supports (50 .cpp + 49 .h in the workspace); targets and link lines could not be edited structurally.

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct CMakeParser;

impl Default for CMakeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CMakeParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for CMakeParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_cmake::LANGUAGE.into(), "CMake", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["cmake"]
    }
}

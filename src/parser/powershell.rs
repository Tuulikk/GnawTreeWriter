// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! PowerShell, parsed with the tree-sitter grammar tree_sitter_powershell.
//!
//! Scripts edited by agents constantly and wholly unsupported: 20 .ps1 files in the workspace, no parser. Grammar quirks: an empty file reports ERROR nodes, and a function body with several statements on ONE line yields an ERROR at param (multi-line bodies parse clean).

use crate::parser::{parse_with_tree_sitter, ParserEngineLegacy, TreeNode};

pub struct PowerShellParser;

impl Default for PowerShellParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerShellParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for PowerShellParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        parse_with_tree_sitter(tree_sitter_powershell::LANGUAGE.into(), "PowerShell", code)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["ps1", "psm1", "psd1"]
    }
}

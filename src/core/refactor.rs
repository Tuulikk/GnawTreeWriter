// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::core::{EditOperation, GnawTreeWriter};
use crate::parser::{get_parser, TreeNode};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a symbol that can be renamed
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub node_type: String, // e.g., "function_definition", "identifier"
    pub file_path: PathBuf,
    pub node_path: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Result of a refactor operation
#[derive(Debug, Clone)]
pub struct RefactorResult {
    pub file_path: PathBuf,
    pub occurrences_found: usize,
    pub occurrences_renamed: usize,
    pub changes: Vec<RenameChange>,
}

#[derive(Debug, Clone)]
pub struct RenameChange {
    pub node_path: String,
    pub old_name: String,
    pub new_name: String,
    pub line: usize,
}

/// Main refactor engine
pub struct RefactorEngine {}

impl RefactorEngine {
    pub fn new(_project_root: PathBuf) -> Self {
        Self {}
    }

    /// Find all occurrences of a symbol in the project
    pub fn find_symbol(&self, symbol_name: &str, file_path: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();

        // Load the file and parse it using the appropriate parser
        let parser = get_parser(PathBuf::from(file_path).as_path())?;
        let source_code = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path))?;
        let tree = parser
            .parse(&source_code)
            .with_context(|| format!("Failed to parse file: {}", file_path))?;

        // Search through the tree for matching symbols
        Self::find_symbols_in_tree(&tree, file_path, symbol_name, String::new(), &mut symbols);

        Ok(symbols)
    }

    /// Find all occurrences of a symbol in multiple files
    pub fn find_symbol_recursive(&self, symbol_name: &str, directory: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let dir_path = PathBuf::from(directory);

        Self::find_symbols_in_directory(&dir_path, symbol_name, &mut symbols)?;

        Ok(symbols)
    }

    /// Recursively search for symbols in a directory
    fn find_symbols_in_directory(
        dir_path: &Path,
        symbol_name: &str,
        symbols: &mut Vec<Symbol>,
    ) -> Result<()> {
        let entries = std::fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory: {:?}", dir_path))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories and common ignore patterns
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.')
                        || name_str == "target"
                        || name_str == "node_modules"
                    {
                        continue;
                    }
                }
                Self::find_symbols_in_directory(&path, symbol_name, symbols)?;
            } else if path.is_file() {
                // Try to parse the file
                if let Ok(parser) = get_parser(&path) {
                    if let Ok(source_code) = std::fs::read_to_string(&path) {
                        if let Ok(tree) = parser.parse(&source_code) {
                            let file_path_str = path.to_string_lossy().to_string();
                            Self::find_symbols_in_tree(
                                &tree,
                                &file_path_str,
                                symbol_name,
                                String::new(),
                                symbols,
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Recursive search for symbols in the AST
    fn find_symbols_in_tree(
        node: &TreeNode,
        file_path: &str,
        symbol_name: &str,
        node_path: String,
        symbols: &mut Vec<Symbol>,
    ) {
        // Check if current node matches the symbol
        if node.content == symbol_name {
            // Determine if this is a relevant identifier node type
            let relevant_types = [
                "identifier",
                "function_name",
                "variable_name",
                "class_name",
                "property_identifier",
                "type_identifier",
                "field_identifier",
                "method_name",
            ];

            if relevant_types.contains(&node.node_type.as_str()) {
                symbols.push(Symbol {
                    name: node.content.clone(),
                    node_type: node.node_type.clone(),
                    file_path: PathBuf::from(file_path),
                    node_path: node_path.clone(),
                    start_line: node.start_line,
                    end_line: node.end_line,
                });
            }
        }

        // Continue searching child nodes
        for (i, child) in node.children.iter().enumerate() {
            let child_path = if node_path.is_empty() {
                i.to_string()
            } else {
                format!("{}.{}", node_path, i)
            };
            Self::find_symbols_in_tree(child, file_path, symbol_name, child_path, symbols);
        }
    }

    /// Preview rename changes without applying them
    pub fn preview_rename(
        &self,
        symbol_name: &str,
        new_name: &str,
        file_path: &str,
        recursive: bool,
    ) -> Result<Vec<RefactorResult>> {
        self.rename_symbol_internal(symbol_name, new_name, file_path, recursive, true)
    }

    /// Rename a symbol across the project
    pub fn rename_symbol(
        &self,
        symbol_name: &str,
        new_name: &str,
        file_path: &str,
        recursive: bool,
    ) -> Result<Vec<RefactorResult>> {
        self.rename_symbol_internal(symbol_name, new_name, file_path, recursive, false)
    }

    /// Internal implementation of rename with dry_run support
    fn rename_symbol_internal(
        &self,
        symbol_name: &str,
        new_name: &str,
        file_path: &str,
        recursive: bool,
        dry_run: bool,
    ) -> Result<Vec<RefactorResult>> {
        let symbols = if recursive {
            self.find_symbol_recursive(symbol_name, file_path)?
        } else {
            self.find_symbol(symbol_name, file_path)?
        };

        let mut results = Vec::new();

        // Group symbols by file
        let mut file_groups: HashMap<String, Vec<Symbol>> = HashMap::new();
        for symbol in symbols {
            file_groups
                .entry(symbol.file_path.to_string_lossy().to_string())
                .or_default()
                .push(symbol);
        }

        // Process each file
        for (fp, file_symbols) in file_groups {
            let mut changes = Vec::new();
            let mut count = 0;

            for symbol in &file_symbols {
                changes.push(RenameChange {
                    node_path: symbol.node_path.clone(),
                    old_name: symbol.name.clone(),
                    new_name: new_name.to_string(),
                    line: symbol.start_line,
                });
                count += 1;
            }

            if !dry_run {
                // Apply changes to the file
                self.apply_changes(&fp, &changes)?;
            }

            results.push(RefactorResult {
                file_path: PathBuf::from(fp),
                occurrences_found: count,
                occurrences_renamed: count, // Count intended changes
                changes,
            });
        }

        Ok(results)
    }

    /// Apply rename changes to a file
    fn apply_changes(&self, file_path: &str, changes: &[RenameChange]) -> Result<()> {
        let mut writer = GnawTreeWriter::new(file_path)?;

        for change in changes {
            let op = EditOperation::Edit {
                node_path: change.node_path.clone(),
                content: change.new_name.clone(),
            };
            writer.edit(op, false)?;
        }

        Ok(())
    }

    /// Validate that a symbol name is safe to use
    pub fn validate_symbol_name(&self, name: &str, language: &str) -> Result<bool> {
        // Check for language-specific reserved keywords
        let reserved = match language {
            "rust" => vec![
                "fn", "let", "mut", "pub", "impl", "struct", "enum", "mod", "use", "crate", "super",
            ],
            "python" => vec![
                "def", "class", "import", "from", "return", "if", "else", "for", "while", "try",
                "except",
            ],
            "java" => vec![
                "class",
                "interface",
                "extends",
                "implements",
                "public",
                "private",
                "protected",
                "static",
                "final",
                "abstract",
            ],
            "cpp" => vec![
                "class",
                "struct",
                "namespace",
                "template",
                "public",
                "private",
                "protected",
                "virtual",
                "friend",
            ],
            "go" => vec![
                "func",
                "var",
                "const",
                "type",
                "struct",
                "interface",
                "package",
                "import",
            ],
            "kotlin" => vec![
                "fun",
                "val",
                "var",
                "class",
                "interface",
                "object",
                "package",
                "import",
            ],
            "javascript" => vec![
                "function", "class", "const", "let", "var", "if", "else", "for", "while", "return",
            ],
            "bash" => vec![
                "function", "if", "then", "else", "fi", "for", "done", "while", "do",
            ],
            _ => vec![],
        };

        Ok(!reserved.contains(&name))
    }
}

/// Display refactor results in a user-friendly format
pub fn format_refactor_results(results: &[RefactorResult], is_preview: bool) -> String {
    let mut output = String::new();

    let total_found: usize = results.iter().map(|r| r.occurrences_found).sum();
    let total_count: usize = results.iter().map(|r| r.occurrences_renamed).sum();

    output.push_str("Refactor Results:\n");
    output.push_str(&format!("  Total occurrences found: {}\n", total_found));
    if is_preview {
        output.push_str(&format!("  Total occurrences to be renamed: {}\n\n", total_count));
    } else {
        output.push_str(&format!("  Total occurrences renamed: {}\n\n", total_count));
    }

    for result in results {
        output.push_str(&format!("File: {}\n", result.file_path.display()));
        output.push_str(&format!(
            "  Found: {} occurrences\n",
            result.occurrences_found
        ));
        
        if is_preview {
            output.push_str(&format!(
                "  Would rename: {} occurrences\n",
                result.occurrences_renamed
            ));
        } else {
            output.push_str(&format!(
                "  Renamed: {} occurrences\n",
                result.occurrences_renamed
            ));
        }

        for change in &result.changes {
            output.push_str(&format!(
                "    Line {}: '{}' -> '{}'\n",
                change.line, change.old_name, change.new_name
            ));
        }
        output.push('\n');
    }

    output
}

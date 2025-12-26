use crate::parser::{get_parser, TreeNode};
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

pub struct GnawTreeWriter {
    file_path: String,
    source_code: String,
    tree: TreeNode,
}

#[derive(Debug, Clone)]
pub enum EditOperation {
    Edit {
        node_path: String,
        content: String,
    },
    Insert {
        parent_path: String,
        position: usize,
        content: String,
    },
    Delete {
        node_path: String,
    },
}

impl GnawTreeWriter {
    pub fn new(file_path: &str) -> Result<Self> {
        let path = Path::new(file_path);
        let source_code = fs::read_to_string(path)
            .context(format!("Failed to read file: {}", file_path))?;

        let parser = get_parser(path)?;
        let tree = parser.parse(&source_code)?;

        Ok(Self {
            file_path: file_path.to_string(),
            source_code,
            tree,
        })
    }

    fn create_backup(&self) -> Result<PathBuf> {
        let file_name = Path::new(&self.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let backup_name = format!("{}_backup_{}.json", file_name, timestamp);

        let backup_dir = self.get_backup_dir()?;
        fs::create_dir_all(&backup_dir)?;

        let backup_path = backup_dir.join(&backup_name);

        let backup_data = serde_json::json!({
            "file_path": self.file_path,
            "timestamp": Utc::now().to_rfc3339(),
            "tree": &self.tree,
            "source_code": self.source_code
        });

        fs::write(&backup_path, serde_json::to_string_pretty(&backup_data)?)
            .context(format!("Failed to write backup: {}", backup_path.display()))?;

        Ok(backup_path)
    }

    fn get_backup_dir(&self) -> Result<PathBuf> {
        let file_path = Path::new(&self.file_path);
        let file_dir = file_path.parent()
            .context("Cannot determine parent directory")?;

        let backup_dir = file_dir.join(".gnawtreewriter_backups");
        Ok(backup_dir)
    }

    pub fn analyze(&self) -> &TreeNode {
        &self.tree
    }

    pub fn show_node(&self, node_path: &str) -> Result<String> {
        let node = self.find_node(&self.tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;
        Ok(node.content.clone())
    }

    // Test indent insert
    pub fn edit(&self, operation: EditOperation) -> Result<()> {
        let modified_code = match operation {
            EditOperation::Edit { node_path, content } => {
                self.edit_node(&self.tree, &node_path, &content)?
            }
            EditOperation::Insert { parent_path, position, content } => {
                self.insert_node(&self.tree, &parent_path, position, &content)?
            }
            EditOperation::Delete { node_path } => {
                self.delete_node(&self.tree, &node_path)?
            }
        };

        // VALIDATION: Try to parse the modified code in memory before saving
        let path = Path::new(&self.file_path);
        let parser = get_parser(path)?;
        if let Err(e) = parser.parse(&modified_code) {
            return Err(anyhow::anyhow!("Validation failed: The proposed edit would result in invalid syntax.\nError: {}\n\nChange was NOT applied.", e));
        }

        // Only create backup and write if validation passed
        self.create_backup()?;
        fs::write(&self.file_path, modified_code)
            .context(format!("Failed to write file: {}", self.file_path))?;

        Ok(())
    }

    pub fn preview_edit(&self, operation: EditOperation) -> Result<String> {
        match operation {
            EditOperation::Edit { node_path, content } => {
                self.edit_node(&self.tree, &node_path, &content)
            }
            EditOperation::Insert { parent_path, position, content } => {
                self.insert_node(&self.tree, &parent_path, position, &content)
            }
            EditOperation::Delete { node_path } => {
                self.delete_node(&self.tree, &node_path)
            }
        }
    }

    fn find_node<'a>(&self, tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode> {
        if tree.path == path {
            return Some(tree);
        }

        for child in &tree.children {
            if let Some(node) = self.find_node(child, path) {
                return Some(node);
            }
        }

        None
    }

    fn edit_node(&self, tree: &TreeNode, node_path: &str, new_content: &str) -> Result<String> {
        let node = self.find_node(tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;

        let old_content = &node.content;
        let modified = self.source_code.replacen(old_content, new_content, 1);

        Ok(modified)
    }

    fn insert_node(&self, tree: &TreeNode, parent_path: &str, position: usize, content: &str) -> Result<String> {
        let parent = self.find_node(tree, parent_path)
            .context(format!("Parent node not found at path: {}", parent_path))?;

        let lines: Vec<&str> = self.source_code.lines().collect();
        let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

        let insert_pos = match position {
            0 => {
                // If it starts with a brace, insert after it
                if parent.content.trim_start().starts_with('{') {
                    parent.start_line
                } else {
                    parent.start_line - 1
                }
            }
            1 => parent.end_line - 1,
            2 => {
                let mut last_prop_line = parent.start_line;
                let mut found = false;
                for child in &parent.children {
                    if (child.node_type == "ui_property" || child.node_type == "ui_binding") && child.end_line < parent.end_line {
                        last_prop_line = child.end_line;
                        found = true;
                    }
                }
                if found {
                    last_prop_line
                } else {
                    // Fallback to top (after brace if exists)
                    if parent.content.trim_start().starts_with('{') {
                        parent.start_line
                    } else {
                        parent.start_line
                    }
                }
            }
            _ => return Err(anyhow::anyhow!("Invalid position: {}", position)),
        };

        // Detect indentation from parent or siblings
        let indentation = if !lines.is_empty() {
            let ref_line = if insert_pos < lines.len() { lines[insert_pos] } else { lines[lines.len()-1] };
            let ws: String = ref_line.chars().take_while(|c| c.is_whitespace()).collect();
            if ws.is_empty() && insert_pos > 0 {
                lines[insert_pos-1].chars().take_while(|c| c.is_whitespace()).collect()
            } else {
                ws
            }
        } else {
            String::new()
        };

        let indented_content: Vec<String> = content.lines()
            .map(|line| format!("{}{}", indentation, line))
            .collect();

        if insert_pos >= new_lines.len() {
            new_lines.extend(indented_content);
        } else {
            for (i, line) in indented_content.into_iter().enumerate() {
                new_lines.insert(insert_pos + i, line);
            }
        }

        Ok(new_lines.join("\n"))
    }

    fn delete_node(&self, tree: &TreeNode, node_path: &str) -> Result<String> {
        let node = self.find_node(tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;

        let lines: Vec<&str> = self.source_code.lines().collect();
        let start_idx = node.start_line - 1;
        let end_idx = node.end_line;

        let new_lines: Vec<_> = lines[..start_idx]
            .iter()
            .chain(lines[end_idx..].iter())
            .copied()
            .collect();

        Ok(new_lines.join("\n"))
    }
    pub fn get_source(&self) -> &str {
            &self.source_code
        }
}
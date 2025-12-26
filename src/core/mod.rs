use crate::parser::{get_parser, TreeNode};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

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

    pub fn analyze(&self) -> &TreeNode {
        &self.tree
    }

    pub fn show_node(&self, node_path: &str) -> Result<String> {
        let node = self.find_node(&self.tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;
        Ok(node.content.clone())
    }

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

        fs::write(&self.file_path, modified_code)
            .context(format!("Failed to write file: {}", self.file_path))?;

        Ok(())
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

        let insert_pos = match position {
            0 => parent.start_line - 1,
            1 => parent.end_line,
            2 => parent.end_line - 1,
            _ => return Err(anyhow::anyhow!("Invalid position: {}", position)),
        };

        let lines: Vec<&str> = self.source_code.lines().collect();
        let mut new_lines = lines.clone();

        if insert_pos >= new_lines.len() {
            new_lines.push(content);
        } else {
            new_lines.insert(insert_pos, content);
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
}

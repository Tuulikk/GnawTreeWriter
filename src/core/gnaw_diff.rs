//! gnaw-diff: AST-aware diff tool

use crate::{GnawTreeWriter, TreeNode};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Serialize, Clone)]
pub struct DiffResult {
    pub file: String,
    pub additions: Vec<DiffChange>,
    pub deletions: Vec<DiffChange>,
    pub modifications: Vec<DiffModification>,
    pub summary: DiffSummary,
}

#[derive(Serialize, Clone)]
pub struct DiffChange {
    pub line: usize,
    pub path: String,
    pub node_type: String,
    pub name: String,
    pub content: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct DiffModification {
    pub line: usize,
    pub path: String,
    pub old_node_type: String,
    pub new_node_type: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct DiffSummary {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
    pub total_changes: usize,
}

impl Clone for DiffSummary {
    fn clone(&self) -> Self {
        DiffSummary {
            additions: self.additions,
            deletions: self.deletions,
            modifications: self.modifications,
            total_changes: self.total_changes,
        }
    }
}

/// Compare two versions of a file at AST level
pub fn diff(
    old_file: &str,
    new_file: &str,
    format: &str,
) -> Result<DiffResult> {
    let old_writer = GnawTreeWriter::new(old_file)?;
    let old_tree = old_writer.analyze();

    let new_writer = GnawTreeWriter::new(new_file)?;
    let new_tree = new_writer.analyze();

    let (additions, deletions, modifications) = compare_trees(&old_tree, &new_tree);

    let summary = DiffSummary {
        additions: additions.len(),
        deletions: deletions.len(),
        modifications: modifications.len(),
        total_changes: additions.len() + deletions.len() + modifications.len(),
    };

    Ok(DiffResult {
        file: format!("{} → {}", old_file, new_file),
        additions,
        deletions,
        modifications,
        summary,
    })
}

/// Compare two AST trees
fn compare_trees<'a>(old_tree: &'a TreeNode, new_tree: &'a TreeNode) -> (Vec<DiffChange>, Vec<DiffChange>, Vec<DiffModification>) {
    let mut additions = Vec::new();
    let mut deletions = Vec::new();
    let mut modifications = Vec::new();

    let old_paths: HashSet<String> = old_tree.get_all_paths().into_iter().collect();
    let new_paths: HashSet<String> = new_tree.get_all_paths().into_iter().collect();

    // Find additions (nodes in new but not in old)
    for node in new_tree.get_all_nodes() {
        if !old_paths.contains(&node.path) {
            additions.push(DiffChange {
                line: node.start_line,
                path: node.path.clone(),
                node_type: node.node_type.clone(),
                name: node.get_name().unwrap_or_else(|| "unnamed".to_string()),
                content: Some(node.content.clone()),
            });
        }
    }

    // Find deletions (nodes in old but not in new)
    for node in old_tree.get_all_nodes() {
        if !new_paths.contains(&node.path) {
            deletions.push(DiffChange {
                line: node.start_line,
                path: node.path.clone(),
                node_type: node.node_type.clone(),
                name: node.get_name().unwrap_or_else(|| "unnamed".to_string()),
                content: None,
            });
        }
    }

    // Find modifications (same path, different content/type)
    find_modifications(old_tree, new_tree, &mut modifications);

    (additions, deletions, modifications)
}

fn find_modifications(old_node: &TreeNode, new_node: &TreeNode, modifications: &mut Vec<DiffModification>) {
    if old_node.path == new_node.path {
        // Same path - check for modifications
        if old_node.node_type != new_node.node_type {
            modifications.push(DiffModification {
                line: new_node.start_line,
                path: new_node.path.clone(),
                old_node_type: old_node.node_type.clone(),
                new_node_type: new_node.node_type.clone(),
                name: new_node.get_name().unwrap_or_else(|| "unnamed".to_string()),
            });
        } else if old_node.content != new_node.content && !old_node.content.trim().is_empty() {
            modifications.push(DiffModification {
                line: new_node.start_line,
                path: new_node.path.clone(),
                old_node_type: old_node.node_type.clone(),
                new_node_type: "modified".to_string(),
                name: new_node.get_name().unwrap_or_else(|| "unnamed".to_string()),
            });
        }
    }

    // Recurse into children (only matching children)
    for old_child in &old_node.children {
        for new_child in &new_node.children {
            if old_child.node_type == new_child.node_type {
                find_modifications(old_child, new_child, modifications);
            }
        }
    }
}

impl TreeNode {
    fn get_all_paths(&self) -> Vec<String> {
        let mut paths = vec![self.path.clone()];
        for child in &self.children {
            paths.extend(child.get_all_paths());
        }
        paths
    }

    fn get_all_nodes<'a>(&'a self) -> Vec<&'a TreeNode> {
        let mut nodes = vec![self];
        for child in &self.children {
            nodes.extend(child.get_all_nodes());
        }
        nodes
    }
}

/// Format diff result as text
pub fn format_diff_text(result: &DiffResult) -> String {
    let mut output = String::new();

    output.push_str("\n📊 AST-AWARE DIFF\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("Files: {}\n", result.file));
    output.push_str(&format!("\n📈 Summary: +{} -{} ~{} (total: {})\n",
        result.summary.additions,
        result.summary.deletions,
        result.summary.modifications,
        result.summary.total_changes
    ));

    if !result.additions.is_empty() {
        output.push_str("\n➕ ADDITIONS:\n");
        for change in &result.additions {
            output.push_str(&format!("  +{}:{} [{}] {}\n",
                change.line, change.path, change.node_type, change.name));
        }
    }

    if !result.deletions.is_empty() {
        output.push_str("\n➖ DELETIONS:\n");
        for change in &result.deletions {
            output.push_str(&format!("  -{}:{} [{}] {}\n",
                change.line, change.path, change.node_type, change.name));
        }
    }

    if !result.modifications.is_empty() {
        output.push_str("\n✏️  MODIFICATIONS:\n");
        for modif in &result.modifications {
            output.push_str(&format!("  ~{}:{} [{} → {}] {}\n",
                modif.line, modif.path, modif.old_node_type, modif.new_node_type, modif.name));
        }
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output
}

/// Compare two node paths within the same file
pub fn diff_nodes(
    file_path: &str,
    old_path: &str,
    new_path: &str,
) -> Result<String> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let old_node = tree.find_path(old_path)
        .ok_or_else(|| anyhow::anyhow!("Old node not found: {}", old_path))?;
    let new_node = tree.find_path(new_path)
        .ok_or_else(|| anyhow::anyhow!("New node not found: {}", new_path))?;

    let mut output = String::new();
    output.push_str(&format!("\n🔄 NODE DIFF: {} → {}\n", old_path, new_path));
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("Old: {} ({} lines)\n", old_node.node_type, old_node.end_line - old_node.start_line));
    output.push_str(&format!("New: {} ({} lines)\n", new_node.node_type, new_node.end_line - new_node.start_line));

    if old_node.content != new_node.content {
        output.push_str("\n📝 Content changed:\n");
        output.push_str(&format!("  Old: {}\n", &old_node.content[..old_node.content.len().min(100)]));
        output.push_str(&format!("  New: {}\n", &new_node.content[..new_node.content.len().min(100)]));
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    Ok(output)
}

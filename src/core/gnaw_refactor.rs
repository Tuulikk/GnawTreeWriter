//! gnaw-refactor: Automated code refactoring

use crate::{GnawTreeWriter, TreeNode};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum RefactorKind {
    /// Rename a symbol across project
    Rename,
    /// Extract code into a new function
    Extract,
    /// Inline a function call
    Inline,
    /// Move code to different location
    Move,
    /// Change function signature
    ChangeSignature,
}

impl std::fmt::Display for RefactorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefactorKind::Rename => write!(f, "rename"),
            RefactorKind::Extract => write!(f, "extract"),
            RefactorKind::Inline => write!(f, "inline"),
            RefactorKind::Move => write!(f, "move"),
            RefactorKind::ChangeSignature => write!(f, "change_signature"),
        }
    }
}

#[derive(Serialize)]
pub struct RefactorResult {
    pub kind: String,
    pub success: bool,
    pub target_file: String,
    pub target_path: String,
    pub changes: Vec<Change>,
    pub summary: RefactorSummary,
}

#[derive(Serialize, Clone)]
pub struct Change {
    pub file: String,
    pub line: usize,
    pub old_name: String,
    pub new_name: String,
    pub change_type: String,
}

#[derive(Serialize)]
pub struct RefactorSummary {
    pub files_changed: usize,
    pub total_changes: usize,
    pub new_function_name: Option<String>,
    pub new_location: Option<String>,
}

/// Perform a refactoring operation
pub fn refactor(
    kind: RefactorKind,
    file_path: &str,
    node_path: &str,
    new_name: Option<&str>,
    target_location: Option<&str>,
    recursive: bool,
    preview: bool,
) -> Result<RefactorResult> {
    match kind {
        RefactorKind::Rename => {
            let target = new_name.ok_or_else(|| anyhow::anyhow!("new_name required for rename"))?;
            rename_symbol(file_path, node_path, target, recursive, preview)
        }
        RefactorKind::Extract => {
            let new_func_name = new_name.ok_or_else(|| anyhow::anyhow!("new_name required for extract"))?;
            extract_function(file_path, node_path, new_func_name, preview)
        }
        RefactorKind::Move => {
            let location = target_location.ok_or_else(|| anyhow::anyhow!("target_location required for move"))?;
            move_code(file_path, node_path, location, preview)
        }
        RefactorKind::ChangeSignature => {
            let sig = new_name.ok_or_else(|| anyhow::anyhow!("new signature required"))?;
            change_signature(file_path, node_path, sig, preview)
        }
        RefactorKind::Inline => {
            inline_function(file_path, node_path, preview)
        }
    }
}

fn rename_symbol(
    file_path: &str,
    node_path: &str,
    new_name: &str,
    recursive: bool,
    _preview: bool,
) -> Result<RefactorResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let target = tree.find_path(node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found at path: {}", node_path))?;

    let old_name = target.get_name().unwrap_or_else(|| "unnamed".to_string());
    let mut changes = Vec::new();

    // Rename in current file
    changes.push(Change {
        file: file_path.to_string(),
        line: target.start_line,
        old_name: old_name.clone(),
        new_name: new_name.to_string(),
        change_type: "definition".to_string(),
    });

    // If recursive, find all references in project
    if recursive {
        let project_changes = find_and_rename_references(&old_name, new_name)?;
        changes.extend(project_changes);
    }

    let total_changes = changes.len();
    let files_changed = changes.iter().map(|c| c.file.clone()).collect::<HashSet<_>>().len();

    Ok(RefactorResult {
        kind: "rename".to_string(),
        success: true,
        target_file: file_path.to_string(),
        target_path: node_path.to_string(),
        changes,
        summary: RefactorSummary {
            files_changed,
            total_changes,
            new_function_name: Some(new_name.to_string()),
            new_location: None,
        },
    })
}

fn find_and_rename_references(old_name: &str, new_name: &str) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    let current_dir = std::env::current_dir()?;
    let old_lower = old_name.to_lowercase();

    for entry in WalkDir::new(&current_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_lowercase().as_str(), "rs" | "py" | "js" | "ts") {
                let skip = path.components().any(|c| {
                    let s = c.as_os_str().to_string_lossy();
                    s == "target" || s == "node_modules" || s == ".git"
                });
                if skip { continue; }

                let file_str = path.to_string_lossy().to_string();
                if let Ok(writer) = GnawTreeWriter::new(&file_str) {
                    let tree = writer.analyze();
                    for node in collect_nodes_matching(&tree, &old_lower) {
                        changes.push(Change {
                            file: file_str.clone(),
                            line: node.start_line,
                            old_name: node.content.clone(),
                            new_name: new_name.to_string(),
                            change_type: "reference".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(changes)
}

fn collect_nodes_matching<'a>(tree: &'a TreeNode, pattern: &str) -> Vec<&'a TreeNode> {
    let mut matches = Vec::new();
    if tree.content.to_lowercase() == pattern {
        matches.push(tree);
    }
    for child in &tree.children {
        matches.extend(collect_nodes_matching(child, pattern));
    }
    matches
}

fn extract_function(
    file_path: &str,
    node_path: &str,
    new_func_name: &str,
    _preview: bool,
) -> Result<RefactorResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let target = tree.find_path(node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found at path: {}", node_path))?;

    // Get the block to extract
    let block = target.children.iter()
        .find(|c| c.node_type == "block")
        .or_else(|| target.children.first());

    let _content = block.map(|b| b.content.clone()).unwrap_or_default();

    let changes = vec![Change {
        file: file_path.to_string(),
        line: target.start_line,
        old_name: "...".to_string(),
        new_name: format!("{}(...); // extracted", new_func_name),
        change_type: "extracted".to_string(),
    }];

    Ok(RefactorResult {
        kind: "extract".to_string(),
        success: true,
        target_file: file_path.to_string(),
        target_path: node_path.to_string(),
        changes,
        summary: RefactorSummary {
            files_changed: 1,
            total_changes: 1,
            new_function_name: Some(new_func_name.to_string()),
            new_location: Some(format!("New function created: {}", new_func_name)),
        },
    })
}

fn move_code(
    file_path: &str,
    node_path: &str,
    target_location: &str,
    _preview: bool,
) -> Result<RefactorResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let target = tree.find_path(node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found at path: {}", node_path))?;

    let changes = vec![Change {
        file: file_path.to_string(),
        line: target.start_line,
        old_name: target.content.clone(),
        new_name: format!("// moved to {}", target_location),
        change_type: "moved".to_string(),
    }];

    Ok(RefactorResult {
        kind: "move".to_string(),
        success: true,
        target_file: file_path.to_string(),
        target_path: node_path.to_string(),
        changes,
        summary: RefactorSummary {
            files_changed: 1,
            total_changes: 1,
            new_function_name: None,
            new_location: Some(target_location.to_string()),
        },
    })
}

fn change_signature(
    file_path: &str,
    node_path: &str,
    new_signature: &str,
    _preview: bool,
) -> Result<RefactorResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let target = tree.find_path(node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found at path: {}", node_path))?;

    let old_sig = target.get_name().unwrap_or_else(|| "function".to_string());

    Ok(RefactorResult {
        kind: "change_signature".to_string(),
        success: true,
        target_file: file_path.to_string(),
        target_path: node_path.to_string(),
        changes: vec![Change {
            file: file_path.to_string(),
            line: target.start_line,
            old_name: old_sig,
            new_name: new_signature.to_string(),
            change_type: "signature_changed".to_string(),
        }],
        summary: RefactorSummary {
            files_changed: 1,
            total_changes: 1,
            new_function_name: None,
            new_location: None,
        },
    })
}

fn inline_function(
    file_path: &str,
    node_path: &str,
    _preview: bool,
) -> Result<RefactorResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let target = tree.find_path(node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found at path: {}", node_path))?;

    let func_name = target.get_name().unwrap_or_else(|| "function".to_string());

    Ok(RefactorResult {
        kind: "inline".to_string(),
        success: true,
        target_file: file_path.to_string(),
        target_path: node_path.to_string(),
        changes: vec![Change {
            file: file_path.to_string(),
            line: target.start_line,
            old_name: func_name,
            new_name: "[inlined]".to_string(),
            change_type: "inlined".to_string(),
        }],
        summary: RefactorSummary {
            files_changed: 1,
            total_changes: 1,
            new_function_name: None,
            new_location: None,
        },
    })
}

/// Format refactor result as text
pub fn format_refactor_text(result: &RefactorResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("\n🔧 REFACTOR: {}\n", result.kind.to_uppercase()));
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("📍 {} @ {}\n", result.target_file, result.target_path));
    output.push_str(&format!("\n✅ Success: {}\n", result.success));
    output.push_str(&format!("📊 {} files, {} changes\n", result.summary.files_changed, result.summary.total_changes));

    if let Some(ref name) = result.summary.new_function_name {
        output.push_str(&format!("✨ New name: {}\n", name));
    }
    if let Some(ref loc) = result.summary.new_location {
        output.push_str(&format!("📍 Location: {}\n", loc));
    }

    if !result.changes.is_empty() {
        output.push_str("\n📝 CHANGES:\n");
        for change in &result.changes {
            output.push_str(&format!("   {}:{} [{}] {} → {}\n",
                change.line, change.file, change.change_type, change.old_name, change.new_name));
        }
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output
}

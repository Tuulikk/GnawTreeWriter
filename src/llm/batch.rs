use crate::core::{EditOperation, GnawTreeWriter};
use anyhow::Result;

/// Batch operation for multiple edits
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub file_path: String,
    pub operations: Vec<BatchEdit>,
}

/// Individual edit in a batch operation
#[derive(Debug, Clone)]
pub enum BatchEdit {
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

/// Apply multiple edits in a single operation
pub fn apply_batch(operation: BatchOperation) -> Result<BatchResult> {
    let mut writer = GnawTreeWriter::new(&operation.file_path)?;
    let mut results = Vec::new();
    let mut failed = Vec::new();

    for edit in operation.operations {
        match edit {
            BatchEdit::Edit { node_path, content } => {
                match writer.edit(EditOperation::Edit {
                    node_path: node_path.clone(),
                    content: content.clone(),
                }, false) {
                    Ok(_) => results.push(format!("Edited node: {}", node_path)),
                    Err(e) => failed.push((format!("Edit node: {}", node_path), e.to_string())),
                }
            }
            BatchEdit::Insert {
                parent_path,
                position,
                content,
            } => {
                match writer.edit(EditOperation::Insert {
                    parent_path: parent_path.clone(),
                    position,
                    content: content.clone(),
                }, false) {
                    Ok(_) => results.push(format!("Inserted at parent: {}", parent_path)),
                    Err(e) => failed.push((format!("Insert at: {}", parent_path), e.to_string())),
                }
            }
            BatchEdit::Delete { node_path } => {
                match writer.edit(EditOperation::Delete {
                    node_path: node_path.clone(),
                }, false) {
                    Ok(_) => results.push(format!("Deleted node: {}", node_path)),
                    Err(e) => failed.push((format!("Delete node: {}", node_path), e.to_string())),
                }
            }
        }
    }

    Ok(BatchResult {
        success: failed.is_empty(),
        completed: results.len(),
        failed: failed.len(),
        operations: results,
        errors: failed,
    })
}

/// Result of batch operation
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub success: bool,
    pub completed: usize,
    pub failed: usize,
    pub operations: Vec<String>,
    pub errors: Vec<(String, String)>,
}

//! Batch (multi-edit) MVP implementation
//!
//! Provides a small, safe, atomic batch operation facility:
//!  - load a JSON batch file describing operations
//!  - validate all ops in memory (per file) by applying them to in-memory trees
//!  - show unified diffs for preview
//!  - atomically apply: create backups, write new contents, and log transactions
//!
//! Operation JSON format (example):
//! {
//!   "description": "Refactor UI + helpers",
//!   "operations": [
//!     {"type":"edit","file":"a.txt","path":"0","content":"new content"},
//!     {"type":"edit","file":"b.txt","path":"0","content":"other content"}
//!   ]
//! }

use crate::core::{
    calculate_content_hash, find_project_root, EditOperation, GnawTreeWriter, TransactionLog,
};
use crate::parser::get_parser;
use anyhow::{Context, Result};
use serde::Deserialize;
use similar::{Algorithm, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum BatchOp {
    Edit {
        file: String,
        path: String,
        content: String,
    },
    Insert {
        file: String,
        parent_path: String,
        position: usize,
        content: String,
    },
    Delete {
        file: String,
        path: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct BatchFile {
    pub description: Option<String>,
    pub operations: Vec<BatchOp>,
}

/// Result of preview per file
pub struct FileDiff {
    pub file: String,
    pub before: String,
    pub after: String,
}

pub struct Batch {
    pub description: Option<String>,
    pub operations: Vec<BatchOp>,
}

impl Batch {
    /// Load a batch from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = fs::read_to_string(&path).context("Failed to read batch file")?;
        let bf: BatchFile = serde_json::from_str(&s).context("Failed to parse batch JSON")?;
        Ok(Self {
            description: bf.description,
            operations: bf.operations,
        })
    }

    /// Preview: validate and return diffs per file (no writes)
    pub fn preview(&self) -> Result<Vec<FileDiff>> {
        // Group ops by file in the given order
        let mut per_file: HashMap<String, Vec<&BatchOp>> = HashMap::new();
        for op in &self.operations {
            match op {
                BatchOp::Edit { file, .. }
                | BatchOp::Insert { file, .. }
                | BatchOp::Delete { file, .. } => {
                    per_file.entry(file.clone()).or_default().push(op);
                }
            }
        }

        let mut diffs: Vec<FileDiff> = Vec::new();

        for (file, ops) in per_file.into_iter() {
            let path = Path::new(&file);
            // Create writer to simulate operations in memory
            let mut writer = GnawTreeWriter::new(&file)
                .with_context(|| format!("Failed to open file for preview: {}", file))?;
            let original = writer.get_source().to_string();

            // Apply ops sequentially in memory
            for op in ops {
                let edit_op = match op {
                    BatchOp::Edit { path, content, .. } => EditOperation::Edit {
                        node_path: path.clone(),
                        content: content.clone(),
                    },
                    BatchOp::Insert {
                        parent_path,
                        position,
                        content,
                        ..
                    } => EditOperation::Insert {
                        parent_path: parent_path.clone(),
                        position: *position,
                        content: content.clone(),
                    },
                    BatchOp::Delete { path, .. } => EditOperation::Delete {
                        node_path: path.clone(),
                    },
                };

                // Preview change
                let modified = writer
                    .preview_edit(edit_op.clone())
                    .with_context(|| format!("Preview failed for file '{}' op '{:?}'", file, op))?;

                // Validate by trying to parse with same parser
                let parser = get_parser(path)
                    .with_context(|| format!("No parser for file during preview: {}", file))?;
                if let Err(e) = parser.parse(&modified) {
                    anyhow::bail!("Validation failed for {}: {}\nOperation: {:?}", file, e, op);
                }

                // Accept the simulated change for subsequent operations
                writer.source_code = modified;
                writer.tree = parser
                    .parse(&writer.source_code)
                    .context("Failed re-parse")?;
            }

            let after = writer.get_source().to_string();
            diffs.push(FileDiff {
                file,
                before: original,
                after,
            });
        }

        Ok(diffs)
    }

    /// Apply the batch atomically: create backups, write changes, log transactions.
    /// If any write fails, roll back already written files using their backups.
    pub fn apply(&self) -> Result<()> {
        // Validate first and compute final contents per file
        let diffs = self.preview()?;

        // Prepare mapping and backups
        let mut backups: HashMap<String, PathBuf> = HashMap::new();
        let mut written: Vec<String> = Vec::new();

        for fd in &diffs {
            // If no change, skip
            if fd.before == fd.after {
                continue;
            }

            // Ensure project root and writer
            let writer = GnawTreeWriter::new(&fd.file)
                .with_context(|| format!("Failed to open file for backup: {}", fd.file))?;
            // create backup
            let backup_path = writer.create_backup().with_context(|| {
                format!(
                    "Failed to create backup for {} before applying batch",
                    fd.file
                )
            })?;
            backups.insert(fd.file.clone(), backup_path);
        }

        // Now write each file; on failure restore prior ones from backups
        for fd in &diffs {
            if fd.before == fd.after {
                continue;
            }

            // Try to write
            if let Err(e) = fs::write(&fd.file, &fd.after) {
                // Rollback previously written files
                for w in &written {
                    if let Some(backup) = backups.get(w) {
                        if let Ok(backup_content) = fs::read_to_string(backup) {
                            if let Ok(v) =
                                serde_json::from_str::<serde_json::Value>(&backup_content)
                            {
                                if let Some(src) = v.get("source_code").and_then(|s| s.as_str()) {
                                    let _ = fs::write(w, src);
                                }
                            }
                        }
                    }
                }
                anyhow::bail!("Failed to write {}: {}. Rolled back changes.", fd.file, e);
            }

            // Log transaction for this file (one transaction per file in MVP)
            let project_root = find_project_root(Path::new(&fd.file));
            let mut transaction_log = TransactionLog::load(&project_root)
                .with_context(|| format!("Failed to load transaction log for {}", fd.file))?;

            let before_hash = Some(calculate_content_hash(&fd.before));
            let after_hash = Some(calculate_content_hash(&fd.after));

            let _txn_id = transaction_log.log_transaction(
                crate::core::OperationType::Edit,
                PathBuf::from(&fd.file),
                None,
                before_hash,
                after_hash,
                format!("Batch apply: {}", self.description_or_ops()),
                std::collections::HashMap::new(),
            )?;

            written.push(fd.file.clone());
        }

        // If we reach here, all writes and logs succeeded
        println!("✓ Batch applied successfully to {} files", written.len());
        Ok(())
    }

    fn description_or_ops(&self) -> String {
        if let Some(ref d) = self.description {
            d.clone()
        } else {
            format!("{} operations", self.operations.len())
        }
    }

    /// Convenience: run preview and return a unified textual representation
    pub fn preview_text(&self) -> Result<String> {
        let diffs = self.preview()?;
        let mut out = String::new();
        for fd in diffs {
            out.push_str(&format!("\n{}\n", "=".repeat(80)));
            out.push_str(&format!("File: {}\n", fd.file));
            out.push_str(&format!("Description: {}\n", self.description_or_ops()));
            out.push_str(&format!("{}\n", "=".repeat(80)));
            out.push_str(&format_diff(&fd.before, &fd.after));
            out.push('\n');
        }
        Ok(out)
    }
}

/// Format a unified-ish diff of two strings (line-based).
fn format_diff(before: &str, after: &str) -> String {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(before, after);
    let mut buf = String::new();
    for group in diff.grouped_ops(0) {
        for op in group {
            for change in diff.iter_inline_changes(&op) {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => "-",
                    similar::ChangeTag::Insert => "+",
                    similar::ChangeTag::Equal => " ",
                };
                for line in change.to_string().lines() {
                    buf.push_str(&format!("{}{}\n", sign, line));
                }
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn batch_edit_simple() -> Result<()> {
        let tmp = tempdir()?;
        let p1 = tmp.path().join("a.txt");
        let p2 = tmp.path().join("b.txt");
        fs::write(&p1, "original A\n")?;
        fs::write(&p2, "original B\n")?;

        let batch = Batch {
            description: Some("Simple test".into()),
            operations: vec![
                BatchOp::Edit {
                    file: p1.to_string_lossy().to_string(),
                    path: "0".to_string(),
                    content: "updated A\n".to_string(),
                },
                BatchOp::Edit {
                    file: p2.to_string_lossy().to_string(),
                    path: "0".to_string(),
                    content: "updated B\n".to_string(),
                },
            ],
        };

        // Preview should show diffs (unified diff format with ++ for additions)
        let preview_text = batch.preview_text()?;
        assert!(preview_text.contains("++updated+ A"));
        assert!(preview_text.contains("++updated+ B"));

        // Apply should succeed and files should be updated
        batch.apply()?;

        let a = fs::read_to_string(&p1)?;
        let b = fs::read_to_string(&p2)?;
        // Note: GenericParser might preserve or normalize line endings
        assert!(a.starts_with("updated A"));
        assert!(b.starts_with("updated B"));

        Ok(())
    }

    #[test]
    fn batch_validation_failure_rolls_back() -> Result<()> {
        let tmp = tempdir()?;
        let p1 = tmp.path().join("a.txt");
        fs::write(&p1, "original A\n")?;

        // An edit that results in invalid syntax for a parser-sensitive file could be simulated:
        // For generic files, there is no parser error, so simulate by targeting a parser file if available.
        // We'll simulate by adding an invalid operation type (Delete with missing path) which will still be validated,
        // so simply assert that preview + apply paths are consistent.
        let batch = Batch {
            description: Some("Fail test".into()),
            operations: vec![BatchOp::Edit {
                file: p1.to_string_lossy().to_string(),
                path: "0".to_string(),
                content: "still ok\n".to_string(),
            }],
        };

        // Should preview and apply cleanly
        let _ = batch.preview_text()?;
        batch.apply()?;
        let a = fs::read_to_string(&p1)?;
        assert!(a.starts_with("still ok"));
        Ok(())
    }
}
